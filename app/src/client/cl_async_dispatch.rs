use std::{net::SocketAddr, sync::Arc, time::Duration};

use bytes::{Bytes, BytesMut};
use rg_net::NET_BUF_SIZE;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::{application::async_runtime::AsyncApp, error::AppError};

#[derive(Debug)]
pub enum Request {
    SendDatagram {
        bytes: Bytes,
    },
    NetworkConnect(SocketAddr),
    Disconnect,
    LoadResource {
        name: String,
        buffer: BytesMut,
        reply_channel: tokio::sync::oneshot::Sender<Result<Bytes, AppError>>,
    },
}

#[derive(Debug)]
pub enum Response {
    Connected(SocketAddr),
    DatagramReceived { bytes: Bytes, address: SocketAddr },
    Error(AppError),
}

pub async fn run_client_worker(
    rx: flume::Receiver<Request>,
    tx: flume::Sender<Response>,
    app: Arc<AsyncApp>,
) {
    debug!("Starting client worker...");

    while let Ok(request) = rx.recv_async().await {
        let tx = tx.clone();
        let rx_clone = rx.clone();

        match request {
            Request::NetworkConnect(addr) => {
                let _ = init_udp_socket(addr, tx, rx_clone, CancellationToken::new()).await;
            }
            Request::LoadResource {
                name,
                buffer,
                reply_channel,
            } => {
                let result = app.files.load_file(name, buffer).await;
                let _ = reply_channel.send(result);
            }
            _ => {}
        }
    }
    debug!("Leaving client worker loop...");
}

async fn init_udp_socket(
    addr: SocketAddr,
    tx: flume::Sender<Response>,
    rx: flume::Receiver<Request>,
    token: CancellationToken,
) {
    match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
        Ok(socket) => {
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
            let token_clone = token.clone();
            let receive_loop = tokio::spawn(async move {
                run_socket_receive_loop(socket_clone, tx, token_clone).await;
            });
            let socket_clone = Arc::clone(&socket);
            let send_loop = tokio::spawn(async move {
                run_socket_send_loop(socket_clone, rx, token).await;
            });

            let _ = send_loop.await;
            let _ = receive_loop.await;
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
    token: CancellationToken,
) {
    let mut bytes = BytesMut::with_capacity(8 * NET_BUF_SIZE);

    loop {
        bytes.resize(NET_BUF_SIZE, 0);

        tokio::select! {
            packet = socket.recv_from(&mut bytes) => match packet {
                Ok((size, client_addr)) => {
                    if size > 0 {
                        debug!("Received {} bytes from server", size);
                        let buf = bytes.split_to(size).freeze();
                        if let Err(_) = tx
                            .send_async(Response::DatagramReceived {
                                bytes: buf,
                                address: client_addr,
                            })
                            .await
                        {
                            debug!("tx channel is closed!");
                            break; // channel is closed, leave loop
                        }
                    }
                }
                Err(e) => {
                    debug!("Socket error: {}", e);
                }
            },

            _ = token.cancelled() => {
                debug!("Token cancelled!");
                break;
            },

            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                if tx.is_disconnected() {
                    debug!("Tx channel is closed!");
                    break;
                }
            }
        }
    }

    debug!("Leaving client receive loop...");
}

async fn run_socket_send_loop(
    socket: Arc<tokio::net::UdpSocket>,
    rx: flume::Receiver<Request>,
    token: CancellationToken,
) {
    loop {
        tokio::select! {
            Ok(request) = rx.recv_async() => {
                match request {
                    Request::SendDatagram { bytes } => {
                        debug!("Sending {} bytes to server", bytes.len());
                        if let Err(e) = socket.send(&bytes).await {
                            warn!("Failed to send {} byte(s): {:?}", bytes.len(), e);
                        }
                    }
                    Request::Disconnect => {
                        token.cancel();
                        debug!("Token cancelled!");
                        break;
                    }
                    _ => {
                        debug!("Unexpected request: {:?}", request);
                    }
                }
            },

            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                if rx.is_disconnected() {
                    break;
                }
            }
        }
    }
    debug!("Leaving client send loop...");
}
