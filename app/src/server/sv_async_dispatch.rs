use std::{net::SocketAddr, sync::Arc};

use bytes::{Bytes, BytesMut};
use tracing::{debug, error, info, warn};
use rg_net::NET_BUF_SIZE;
use tokio::{net::UdpSocket, task::JoinHandle};
use tokio_util::sync::CancellationToken;

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
    let mut token = CancellationToken::new();

    while let Ok(request) = rx.recv_async().await {
        let tx = tx.clone();

        match request {
            Request::StartNetworkLoop(addr) => {
                token = CancellationToken::new();

                if let Some((s, h)) = init_udp_socket(addr, tx, token.clone()).await {
                    socket = Some(s);
                    handle = Some(h);
                }
            }
            Request::StopNetworkLoop => {
                token.cancel();

                if let Some(handle) = handle.take() {
                    let _ = handle.abort();
                }
                let _ = socket.take();
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
    token: CancellationToken,
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
                run_socket_receive_loop(socket_clone, tx, token).await;
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
    token: CancellationToken,
) {
    debug!("Entering server receive loop...");
    let mut bytes = BytesMut::with_capacity(8 * NET_BUF_SIZE);

    loop {
        bytes.resize(NET_BUF_SIZE, 0);

        tokio::select! {
            result = socket.recv_from(&mut bytes) => {
                match result {
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
                                debug!("Tx channel closed");
                                break; // channel is closed, leave loop
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Socket error: {}", e);
                    }
                }
            },

            _ = token.cancelled() => {
                debug!("Token cancelled!");
                break;
            }
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
