use std::{net::SocketAddr, sync::Arc};

use bytes::{Bytes, BytesMut};
use log::{debug, error, warn};
use rg_net::NET_BUF_SIZE;

use crate::error::AppError;

#[derive(Debug)]
pub enum Request {
    NetworkConnect(SocketAddr),
    SendDatagram { bytes: Bytes },
}

#[derive(Debug)]
pub enum Response {
    Connected(SocketAddr),
    DatagramReceived { bytes: Bytes, address: SocketAddr },
    Error(AppError),
}

pub async fn dispatch_client_request(
    request: Request,
    tx: flume::Sender<Response>,
    sender_rx: flume::Receiver<Request>,
) {
    match request {
        Request::NetworkConnect(addr) => {
            tokio::spawn(async move {
                init_udp_socket_loops(addr, tx, sender_rx).await;
            });
        }
        _ => {}
    }
}

async fn init_udp_socket_loops(
    addr: SocketAddr,
    tx: flume::Sender<Response>,
    sender_rx: flume::Receiver<Request>,
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
            let receive_loop = tokio::spawn(async move {
                run_socket_receive_loop(socket_clone, tx).await;
            });
            let socket_clone = Arc::clone(&socket);
            let send_loop = tokio::spawn(async move {
                run_socket_send_loop(socket_clone, sender_rx).await;
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

async fn run_socket_receive_loop(socket: Arc<tokio::net::UdpSocket>, tx: flume::Sender<Response>) {
    let mut bytes = BytesMut::with_capacity(8 * NET_BUF_SIZE);
    loop {
        bytes.resize(NET_BUF_SIZE, 0);
        match socket.recv_from(&mut bytes).await {
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
        }
    }
    debug!("Leaving client receive loop...");
}

async fn run_socket_send_loop(socket: Arc<tokio::net::UdpSocket>, rx: flume::Receiver<Request>) {
    while let Ok(request) = rx.recv_async().await {
        match request {
            Request::SendDatagram { bytes } => {
                debug!("Sending {} bytes to server", bytes.len());
                if let Err(e) = socket.send(&bytes).await {
                    warn!("Failed to send {} byte(s): {:?}", bytes.len(), e);
                }
            }
            _ => {}
        }
    }
    debug!("Leaving client send loop...");
}
