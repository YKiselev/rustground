use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
};

use tokio::runtime::Runtime;
use tracing::error;

use crate::{
    application::async_files::AsyncFiles,
    client::{self, run_client_worker},
    error::AppError,
    server::{self, run_server_worker},
};

pub struct RequestResponseChannel<Rq, Rs> {
    pub tx: flume::Sender<Rq>,
    pub rx: flume::Receiver<Rs>,
}

impl<Rq, Rs> Clone for RequestResponseChannel<Rq, Rs> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            rx: self.rx.clone(),
        }
    }
}

pub type ClientChannel = RequestResponseChannel<client::Request, client::Response>;
pub type ServerChannel = RequestResponseChannel<server::Request, server::Response>;

pub struct AsyncApp {
    pub files: AsyncFiles,
}

pub fn init_client_server_async_runtime(
    files: AsyncFiles,
) -> Result<(JoinHandle<()>, ServerChannel, ClientChannel), AppError> {
    let (server_tx, from_server_rx) = flume::unbounded::<server::Request>();
    let (to_server_tx, server_rx) = flume::unbounded::<server::Response>();
    let (client_tx, from_client_rx) = flume::unbounded::<client::Request>();
    let (to_client_tx, client_rx) = flume::unbounded::<client::Response>();

    let async_app = Arc::new(AsyncApp { files });
    let app_clone = Arc::clone(&async_app);
    let app_clone2 = Arc::clone(&async_app);
    let handle = thread::spawn(move || {
        let rt = create_async_runtime()
            .inspect_err(|e| error!("Unable to create async runtime: {:?}", e))
            .unwrap();

        let _ = rt.block_on(async {
            let server_handle =
                rt.spawn(run_server_worker(from_server_rx, to_server_tx, app_clone));
            let client_handle =
                rt.spawn(run_client_worker(from_client_rx, to_client_tx, app_clone2));

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

fn create_async_runtime() -> Result<Runtime, std::io::Error> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(4)
        .thread_name_fn(|| {
            static ID: AtomicUsize = AtomicUsize::new(1);
            let id = ID.fetch_add(1, Ordering::SeqCst);
            format!("async-{}", id)
        })
        .build()
}
