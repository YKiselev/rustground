use std::{net::SocketAddr, sync::Arc};

use log::{debug, error, warn};
use rg_net::MAX_DATAGRAM_SIZE;

use crate::error::AppError;

#[derive(Debug)]
pub enum Request {
    NetworkConnect(SocketAddr),
    SendDatagram { bytes: Vec<u8> },
    LoadResource,
}

#[derive(Debug)]
pub enum Response {
    Ok, // debug
    Connected(SocketAddr),
    DatagramReceived { bytes: Vec<u8>, address: SocketAddr },
    Error(AppError),
}

pub async fn dispatch_client_request(
    request: Request,
    tx: flume::Sender<Response>,
    sender_rx: flume::Receiver<Request>,
) {
    match request {
        Request::SendDatagram { bytes: _ } => {
            // no-op here, handled in send worker
        }
        Request::NetworkConnect(addr) => {
            tokio::spawn(async move {
                init_udp_socket_loops(addr, tx, sender_rx).await;
            });
        }
        Request::LoadResource => todo!(),
    }
}

async fn init_udp_socket_loops(
    addr: SocketAddr,
    tx: flume::Sender<Response>,
    sender_rx: flume::Receiver<Request>,
) {
    match tokio::net::UdpSocket::bind( "0.0.0.0:0").await {
        Ok(socket) => {
            let socket = Arc::new(socket);
            match socket.connect(addr).await {
                Ok(_) => {
                    if let Err(_) = tx.send_async(Response::Connected(addr)).await {
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

            tokio::join!(receive_loop, send_loop);
        }
        Err(e) => {
            if let Err(e) = tx
                .send_async(Response::Error(AppError::AsyncError(e.to_string())))
                .await
            {
                error!("Unable to create client socket: {}", e.to_string());
            }
        }
    };
}

async fn run_socket_receive_loop(socket: Arc<tokio::net::UdpSocket>, tx: flume::Sender<Response>) {
    let mut buf = Vec::with_capacity(MAX_DATAGRAM_SIZE);
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((size, client_addr)) => {
                let bytes = buf[..size].to_vec();
                if let Err(_) = tx
                    .send_async(Response::DatagramReceived {
                        bytes,
                        address: client_addr,
                    })
                    .await
                {
                    break; // channel is closed, leave loop
                }
            }
            Err(_) => {
                break; // channel is closed, leave loop
            }
        }
    }
    debug!("Leaving client receive loop...");
}

async fn run_socket_send_loop(socket: Arc<tokio::net::UdpSocket>, rx: flume::Receiver<Request>) {
    while let Ok(request) = rx.recv_async().await {
        match request {
            Request::SendDatagram { bytes } => {
                if let Err(e) = socket.send(&bytes).await {
                    warn!("Failed to send {} byte(s): {:?}", bytes.len(), e);
                }
            }
            _ => {}
        }
    }
    debug!("Leaving client send loop...");
}
