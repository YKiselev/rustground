use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use bytes::{Bytes, BytesMut};
use log::{debug, error, info, warn};
use rg_net::NET_BUF_SIZE;
use tokio::{net::UdpSocket, task::JoinHandle};

use crate::error::AppError;

#[derive(Debug)]
pub enum Request {
    StartNetworkLoop(SocketAddr),
    StopNetworkLoop,
    SendDatagram {
        addr: SocketAddr,
        bytes: Bytes,
        index: u64,
    },
}

#[derive()]
pub enum Response {
    Error(AppError),
    NetworkLoopStarted(SocketAddr),
    DatagramReceived { bytes: Bytes, address: SocketAddr },
}

///
/// Server worker loop
///
pub async fn run_server_worker(rx: flume::Receiver<Request>, tx: flume::Sender<Response>) {
    debug!("Starting server worker...");

    let mut socket = None;
    let mut handle = None;
    let exit_flag = Arc::new(AtomicBool::new(false));

    while let Ok(request) = rx.recv_async().await {
        let tx = tx.clone();

        match request {
            Request::StartNetworkLoop(addr) => {
                exit_flag.store(false, Ordering::Release);
                if let Some((s, h)) =
                    init_udp_socket(addr, tx, Arc::clone(&exit_flag)).await
                {
                    socket = Some(s);
                    handle = Some(h);
                }
            }
            Request::StopNetworkLoop => {
                exit_flag.store(true, Ordering::Release);
                if let Some(handle) = handle.take() {
                    let _ = handle.abort();
                }
            }
            Request::SendDatagram { addr, bytes, index } => {
                if let Some(s) = socket.as_ref() {
                    socket_send(s, addr, bytes, index).await;
                } else {
                    warn!("No socket to send packet!");
                }
            }
        }
    }
    debug!("Leaving server worker loop...");
}

async fn init_udp_socket(
    addr: SocketAddr,
    tx: flume::Sender<Response>,
    exit_flag: Arc<AtomicBool>,
) -> Option<(Arc<UdpSocket>, JoinHandle<()>)> {
    match tokio::net::UdpSocket::bind(addr).await {
        Ok(socket) => {
            let socket = Arc::new(socket);
            match socket.local_addr() {
                Ok(local_addr) => {
                    if let Err(_) = tx
                        .send_async(Response::NetworkLoopStarted(local_addr))
                        .await
                    {
                        return None; // channel is closed, leave
                    }
                    info!("Server socket bound to {}", local_addr);
                }
                Err(e) => {
                    warn!("Unable to get socket's local address: {}", e);
                    return None;
                }
            }

            let socket_clone = Arc::clone(&socket);
            let receive_loop = tokio::spawn(async move {
                run_socket_receive_loop(socket_clone, tx, exit_flag).await;
            });
            return Some((socket, receive_loop));
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
    None
}

async fn run_socket_receive_loop(
    socket: Arc<tokio::net::UdpSocket>,
    tx: flume::Sender<Response>,
    exit_flag: Arc<AtomicBool>,
) {
    debug!("Entering server receive loop...");
    let mut bytes = BytesMut::with_capacity(8 * NET_BUF_SIZE);
    loop {
        bytes.resize(NET_BUF_SIZE, 0);
        match socket.recv_from(&mut bytes).await {
            Ok((size, client_addr)) => {
                if size > 0 {
                    debug!("Received {} bytes from client", size);
                    let buf = bytes.split_to(size).freeze();
                    if let Err(_) = tx
                        .send_async(Response::DatagramReceived {
                            bytes: buf,
                            address: client_addr,
                        })
                        .await
                    {
                        debug!("tx channel closed");
                        break; // channel is closed, leave loop
                    }
                }
            }
            Err(e) => {
                debug!("Socket error: {}", e);
            }
        }
        if exit_flag.load(Ordering::Relaxed) {
            debug!("Exit flag is set");
            break;
        }
    }
    debug!("Leaving server receive loop...");
}

async fn socket_send(
    socket: &Arc<tokio::net::UdpSocket>,
    addr: SocketAddr,
    bytes: Bytes,
    index: u64,
) {
    debug!("Sending {} bytes of #{} to client", bytes.len(), index);
    if let Err(e) = socket.send_to(&bytes, addr).await {
        warn!(
            "Failed to send {} byte(s) to {}: {:?}",
            bytes.len(),
            addr,
            e
        );
    }
}
