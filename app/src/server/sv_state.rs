use std::{
    cell::RefCell,
    net::SocketAddr,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};

use log::{debug, info, warn};
use rg_common::App;
use rg_net::{
    BufferPool, NET_BUF_SIZE, NetBufReader, PacketKind, PooledBuffer, process_buf, read_connect,
    read_hello, read_ping,
};

use crate::{
    application::async_runtime::ServerChannel,
    error::AppError,
    server::{
        self,
        messages::{sv_connect::on_connect, sv_hello::on_hello, sv_ping::on_ping},
        server::ServerConfig,
        sv_clients::{ClientId, Clients},
        sv_guests::Guests,
        sv_security::ServerSecurity,
    },
};

#[derive()]
pub(super) struct ServerState {
    config: Arc<RwLock<ServerConfig>>,
    clients: Clients,
    guests: Guests,
    security: ServerSecurity,
    channel: ServerChannel,
    buffer_pool: Arc<Mutex<BufferPool>>,
}

impl ServerState {
    pub fn new(
        _app: &App,
        config: &Arc<RwLock<ServerConfig>>,
        channel: ServerChannel,
    ) -> Result<Self, AppError> {
        info!("Starting server...");
        let cfg = config.read()?;
        let addr: SocketAddr = cfg.address.parse()?;
        let _ = channel
            .tx
            .send(server::Request::StartNetworkLoop(addr))
            .map_err(|e| AppError::ChannelError(e.to_string()))?;

        let security = ServerSecurity::new(cfg.key_bits, &cfg.password)?;

        drop(cfg);

        let buffer_pool = Arc::new(Mutex::new(BufferPool::new(NET_BUF_SIZE, "server")));

        Ok(ServerState {
            config: Arc::clone(config),
            clients: Clients::new(),
            guests: Guests::new(Arc::clone(&buffer_pool)),
            security,
            channel,
            buffer_pool,
        })
    }

    pub fn shutdow(self) {
        if let Err(_) = self.channel.tx.send(server::Request::StopNetworkLoop) {
            warn!("Unable to send shutdown signal to async workers!");
        }
    }

    pub fn update(&mut self) -> Result<(), AppError> {
        let rx = self.channel.rx.clone();
        for p in rx.try_iter() {
            match p {
                server::Response::Error(e) => {
                    warn!("Async runtime reports error: {}", e);
                }
                server::Response::NetworkLoopStarted(socket_addr) => {
                    let mut cfg = self.config.write()?;
                    cfg.bound_to = Some(socket_addr.to_string());
                }
                server::Response::DatagramReceived { bytes, address } => {
                    self.process_network_datagram(address, &bytes);
                    if let Ok(mut pool) = self.buffer_pool.lock() {
                        pool.release_buffer(bytes);
                    }
                }
            }
        }

        self.guests.flush(&self.channel.tx);

        self.clients.flush(&self.channel.tx);

        Ok(())
    }

    fn process_network_datagram(&mut self, address: SocketAddr, bytes: &PooledBuffer) {
        let clients = &mut self.clients;
        let guests = &mut self.guests;
        let security = &self.security;

        let client_id = ClientId::new(address);
        let mut reader = NetBufReader::new(bytes.as_slice());
        let _ = process_buf(&mut reader, |header, reader| {
            debug!("Got {:?} from client {}", header, address);
            match header.kind {
                PacketKind::Hello => {
                    if clients.exists(&client_id) {
                        false
                    } else if let Ok(ref hello) = read_hello(reader) {
                        on_hello(&client_id, hello, guests, security.keys.public_key_bytes());
                        true
                    } else {
                        false
                    }
                }

                PacketKind::Connect => {
                    if let Ok(ref connect) = read_connect(reader) {
                        let _ = on_connect(&client_id, connect, guests, clients, security)
                            .inspect_err(|e| warn!("Unable to connect client: {:?}", e));
                        true
                    } else {
                        false
                    }
                }

                PacketKind::Ping => {
                    if let Ok(ref ping) = read_ping(reader) {
                        let _ = on_ping(&client_id, ping, guests);
                        true
                    } else {
                        false
                    }
                }

                _ => false,
            }
        });
    }
}
