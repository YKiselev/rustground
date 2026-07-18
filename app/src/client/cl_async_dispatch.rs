use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use log::{debug, error, warn};
use rg_net::{BufferPool, NET_BUF_SIZE, PooledBuffer};

use crate::error::AppError;

#[derive(Debug)]
pub enum Request {
    NetworkConnect(SocketAddr),
    SendDatagram { bytes: PooledBuffer },
}

#[derive(Debug)]
pub enum Response {
    Connected(SocketAddr),
    DatagramReceived {
        bytes: PooledBuffer,
        address: SocketAddr,
    },
    Error(AppError),
}

#[derive(Default)]
struct AsyncState {
    buffer_pool: Mutex<BufferPool>,
}

impl AsyncState {
    fn aquire_buffer(&self) -> Option<PooledBuffer> {
        if let Ok(mut pool) = self.buffer_pool.lock() {
            return Some(pool.aquire_buffer());
        }

        None
    }
}

pub async fn dispatch_client_request(
    request: Request,
    tx: flume::Sender<Response>,
    sender_rx: flume::Receiver<Request>
) {
    match request {
        Request::NetworkConnect(addr) => {
            tokio::spawn(async move {
                init_udp_socket_loops(addr, tx, sender_rx).await;
            });
        }
        _ => {},
    }
}

async fn init_udp_socket_loops(
    addr: SocketAddr,
    tx: flume::Sender<Response>,
    sender_rx: flume::Receiver<Request>,
) {
    match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
        Ok(socket) => {
            let state = Arc::new(AsyncState::default());
            let socket = Arc::new(socket);
            match socket.connect(addr).await {
                Ok(_) => {
                    if let Err(_) = tx.send_async(Response::Connected(addr)).await {
                        debug!("tx channel is closed!");
                        return; // channel is closed, leave
                    }
                }
                Err(e) => {
                    warn!("Unable to connect to {}: {:?}", addr, e);
                }
            }

            let socket_clone = Arc::clone(&socket);
            let state_clone = Arc::clone(&state);
            let receive_loop = tokio::spawn(async move {
                run_socket_receive_loop(socket_clone, tx, state_clone).await;
            });
            let socket_clone = Arc::clone(&socket);
            let state_clone = Arc::clone(&state);
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
                error!("Unable to create client socket: {:?}", e);
                error!("tx channel is closed!");
            }
        }
    };
}

async fn run_socket_receive_loop(
    socket: Arc<tokio::net::UdpSocket>,
    tx: flume::Sender<Response>,
    state: Arc<AsyncState>,
) {
    loop {
        let bytes = state.aquire_buffer();
        if bytes.is_none() {
            warn!("Unable to aquire network buffer from pool!");
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
                    debug!("tx channel is closed!");
                    break; // channel is closed, leave loop
                }
            }
            Err(e) => {
                debug!("Socket error: {}", e);
            }
        }
    }
    debug!("Leaving client receive loop...");
}

async fn run_socket_send_loop(
    socket: Arc<tokio::net::UdpSocket>,
    rx: flume::Receiver<Request>,
    state: Arc<AsyncState>,
) {
    while let Ok(request) = rx.recv_async().await {
        match request {
            Request::SendDatagram { bytes } => {
                if let Err(e) = socket.send(bytes.as_slice()).await {
                    warn!("Failed to send {} byte(s): {:?}", bytes.len(), e);
                }
                if let Ok(mut pool) = state.buffer_pool.lock() {
                    pool.release_buffer(bytes);
                }
            }
            _ => {}
        }
    }
    debug!("Leaving client send loop...");
}
