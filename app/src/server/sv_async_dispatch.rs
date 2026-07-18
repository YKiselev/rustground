use std::{
    net::SocketAddr, ops::Deref, sync::{Arc, atomic::{AtomicBool, Ordering}},
};

use log::{debug, error, info, warn};
use rg_net::MAX_DATAGRAM_SIZE;

use crate::error::AppError;

#[derive(Debug)]
pub enum Request {
    StartNetworkLoop(SocketAddr),
    StopNetworkLoop,
    SendDatagram { addr: SocketAddr, bytes: Vec<u8> },
    LoadResource,
}

#[derive(Debug)]
pub enum Response {
    Error(AppError),
    NetworkLoopStarted(SocketAddr),
    DatagramReceived { bytes: Vec<u8>, address: SocketAddr },
}

pub async fn dispatch_server_request(
    request: Request,
    tx: flume::Sender<Response>,
    sender_rx: flume::Receiver<Request>,
) {
    let exit_flag = Arc::new(AtomicBool::new(false));
    match request {
        Request::StartNetworkLoop(addr) => {
            exit_flag.store(false, Ordering::Release);
            tokio::spawn(async move {
                init_udp_socket_loops(addr, tx, sender_rx, exit_flag).await;
            });
        }
        Request::LoadResource => todo!(),
        Request::StopNetworkLoop => {
            exit_flag.store(true, Ordering::Release);
        }
        Request::SendDatagram { addr: _, bytes: _ } => {
            // no-op here, handled in send worker
        }
    }
}

async fn init_udp_socket_loops(
    addr: SocketAddr,
    tx: flume::Sender<Response>,
    sender_rx: flume::Receiver<Request>,
    exit_flag: Arc<AtomicBool>,
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
            let exit_flag_clone = exit_flag.clone();
            let receive_loop = tokio::spawn(async move {
                run_socket_receive_loop(socket_clone, tx, exit_flag_clone).await;
            });
            let socket_clone = Arc::clone(&socket);
            let exit_flag_clone = exit_flag.clone();
            let send_loop = tokio::spawn(async move {
                run_socket_send_loop(socket_clone, sender_rx, exit_flag_clone).await;
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
    exit_flag: Arc<AtomicBool>,
) {
    let mut buf = Vec::with_capacity(MAX_DATAGRAM_SIZE);
    debug!("Entering server receive loop...");
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
        if exit_flag.load(Ordering::Relaxed) {
            break;
        }
    }
    debug!("Leaving server receive loop...");
}

async fn run_socket_send_loop(
    socket: Arc<tokio::net::UdpSocket>,
    rx: flume::Receiver<Request>,
    exit_flag: Arc<AtomicBool>,
) {
    debug!("Entering server send loop...");
    while let Ok(request) = rx.recv_async().await
        && !exit_flag.load(Ordering::Relaxed)
    {
        match request {
            Request::SendDatagram { addr, bytes } => {
                if let Err(e) = socket.send_to(&bytes, addr).await {
                    warn!(
                        "Failed to send {} byte(s) to {}: {:?}",
                        bytes.len(),
                        addr,
                        e
                    );
                }
            }
            _ => {}
        }
    }
    debug!("Leaving server send loop...");
}
