use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
};

use rg_common::App;

use crate::{
    client::{self, run_client_worker},
    error::AppError,
    server::{self, run_server_worker},
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

pub fn init_client_server_async_runtime(
    app: Arc<App>,
) -> Result<(JoinHandle<()>, ServerChannel, ClientChannel), AppError> {
    let (server_tx, from_server_rx) = flume::unbounded::<server::Request>();
    let (to_server_tx, server_rx) = flume::unbounded::<server::Response>();
    let (client_tx, from_client_rx) = flume::unbounded::<client::Request>();
    let (to_client_tx, client_rx) = flume::unbounded::<client::Response>();

    let handle = thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name_fn(|| {
                static ID: AtomicUsize = AtomicUsize::new(1);
                let id = ID.fetch_add(1, Ordering::SeqCst);
                format!("async-{}", id)
            })
            .build()
            .expect("Async runtime initialization failed!");

        let _ = rt.block_on(async {
            let server_handle = rt.spawn(async move {
                let _ = run_server_worker(from_server_rx, to_server_tx).await;
            });

            let client_handle = rt.spawn(async move {
                let _ = run_client_worker(from_client_rx, to_client_tx).await;
            });

            let _ = server_handle.await;
            let _ = client_handle.await;
        });
    });

    Ok((
        handle,
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
