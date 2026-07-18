use std::{
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use log::{debug, error, info, warn};
use rg_net::{BufferPool, NET_BUF_SIZE, PooledBuffer};

use crate::error::AppError;

#[derive()]
pub enum Request {
    StartNetworkLoop(SocketAddr),
    StopNetworkLoop,
    SendDatagram {
        addr: SocketAddr,
        bytes: PooledBuffer,
    },
}

#[derive()]
pub enum Response {
    Error(AppError),
    NetworkLoopStarted(SocketAddr),
    DatagramReceived {
        bytes: PooledBuffer,
        address: SocketAddr,
    },
}

#[derive(Default)]
struct AsyncState {
    exit_flag: AtomicBool,
    buffer_pool: Mutex<BufferPool>,
}

impl AsyncState {
    fn should_exit(&self) -> bool {
        self.exit_flag.load(Ordering::Relaxed)
    }

    fn aquire_buffer(&self) -> Option<PooledBuffer> {
        if let Ok(mut pool) = self.buffer_pool.lock() {
            Some(pool.aquire_buffer())
        } else {
            None
        }
    }

    fn release_buffer(&self, buf: PooledBuffer) {
        if let Ok(mut pool) = self.buffer_pool.lock() {
            pool.release_buffer(buf);
        }
    }
}

pub async fn dispatch_server_request(
    request: Request,
    tx: flume::Sender<Response>,
    sender_rx: flume::Receiver<Request>,
) {
    let state = Arc::new(AsyncState::default());
    match request {
        Request::StartNetworkLoop(addr) => {
            state.exit_flag.store(false, Ordering::Release);
            tokio::spawn(async move {
                init_udp_socket_loops(addr, tx, sender_rx, state).await;
            });
        }
        Request::StopNetworkLoop => {
            state.exit_flag.store(true, Ordering::Release);
        }
        _ => {}
    }
}

async fn init_udp_socket_loops(
    addr: SocketAddr,
    tx: flume::Sender<Response>,
    sender_rx: flume::Receiver<Request>,
    state: Arc<AsyncState>,
) {
    match tokio::net::UdpSocket::bind(addr).await {
        Ok(socket) => {
            let socket = Arc::new(socket);
            match socket.local_addr() {
                Ok(local_addr) => {
                    if let Err(_) = tx
                        .send_async(Response::NetworkLoopStarted(local_addr))
                        .await
                    {
                        return; // channel is closed, leave
                    }
                    info!("Server socket bound to {}", local_addr);
                }
                Err(e) => {
                    warn!("Unable to get socket's local address: {}", e);
                    return;
                }
            }

            let socket_clone = Arc::clone(&socket);
            let state_clone = state.clone();
            let receive_loop = tokio::spawn(async move {
                run_socket_receive_loop(socket_clone, tx, state_clone).await;
            });
            let socket_clone = Arc::clone(&socket);
            let state_clone = state.clone();
            let send_loop = tokio::spawn(async move {
                run_socket_send_loop(socket_clone, sender_rx, state_clone).await;
            });

            let _ = tokio::join!(receive_loop, send_loop);
        }
        Err(e) => {
            if let Err(e) = tx
                .send_async(Response::Error(AppError::AsyncError(e.to_string())))
                .await
            {
                error!("Unable to bind server udp socket: {}", e.to_string());
            }
        }
    };
}

async fn run_socket_receive_loop(
    socket: Arc<tokio::net::UdpSocket>,
    tx: flume::Sender<Response>,
    state: Arc<AsyncState>,
) {
    debug!("Entering server receive loop...");
    loop {
        let bytes = state.aquire_buffer();
        if bytes.is_none() {
            warn!("Unable to get network buffer from pool!");
            break;
        }
        let mut bytes = bytes.unwrap();
        bytes.resize(NET_BUF_SIZE, 0);
        match socket.recv_from(bytes.as_mut_slice()).await {
            Ok((size, client_addr)) => {
                bytes.truncate(size);
                if let Err(_) = tx
                    .send_async(Response::DatagramReceived {
                        bytes,
                        address: client_addr,
                    })
                    .await
                {
                    debug!("tx channel closed");
                    break; // channel is closed, leave loop
                }
            }
            Err(e) => {
                debug!("Socket error: {}", e);
            }
        }
        if state.should_exit() {
            debug!("Exit flag is set");
            break;
        }
    }
    debug!("Leaving server receive loop...");
}

async fn run_socket_send_loop(
    socket: Arc<tokio::net::UdpSocket>,
    rx: flume::Receiver<Request>,
    state: Arc<AsyncState>,
) {
    debug!("Entering server send loop...");
    while let Ok(request) = rx.recv_async().await
        && !state.should_exit()
    {
        match request {
            Request::SendDatagram { addr, bytes } => {
                if let Err(e) = socket.send_to(bytes.as_slice(), addr).await {
                    warn!(
                        "Failed to send {} byte(s) to {}: {:?}",
                        bytes.len(),
                        addr,
                        e
                    );
                }
                state.release_buffer(bytes);
            }
            _ => {}
        }
    }
    debug!("Leaving server send loop...");
}
