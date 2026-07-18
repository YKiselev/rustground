use std::thread::{self};

use log::debug;

use crate::{
    client::{self, dispatch_client_request},
    error::AppError,
    server::{self, dispatch_server_request},
};

#[derive(Clone)]
pub struct ClientChannel {
    pub tx: flume::Sender<client::Request>,
    pub rx: flume::Receiver<client::Response>,
}

#[derive(Clone)]
pub struct ServerChannel {
    pub tx: flume::Sender<server::Request>,
    pub rx: flume::Receiver<server::Response>,
}

pub fn init_client_server_tokio_runtime() -> Result<(ServerChannel, ClientChannel), AppError> {
    let (server_tx, from_server_rx) = flume::unbounded::<server::Request>();
    let (to_server_tx, server_rx) = flume::unbounded::<server::Response>();
    let (client_tx, from_client_rx) = flume::unbounded::<client::Request>();
    let (to_client_tx, client_rx) = flume::unbounded::<client::Response>();

    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Async runtime initialization failed!");

        // Server worker
        rt.spawn(async move {
            debug!("Starting server worker...");
            while let Ok(request) = from_server_rx.recv_async().await {
                let tx = to_server_tx.clone();
                let send_rx = from_server_rx.clone();
                tokio::spawn(async move {
                    let _ = dispatch_server_request(request, tx, send_rx.clone()).await;
                });
            }
            debug!("Leaving server worker loop...");
        });

        // Client worker
        rt.block_on(async move {
            debug!("Starting client worker...");
            while let Ok(request) = from_client_rx.recv_async().await {
                let tx = to_client_tx.clone();
                let send_rx = from_client_rx.clone();

                tokio::spawn(async move {
                    let _ = dispatch_client_request(request, tx, send_rx).await;
                });
            }
            debug!("Leaving client worker loop...");
        });
    });

    Ok((
        ServerChannel {
            tx: server_tx,
            rx: server_rx,
        },
        ClientChannel {
            tx: client_tx,
            rx: client_rx,
        },
    ))
}
