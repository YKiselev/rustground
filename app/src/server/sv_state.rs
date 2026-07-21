use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use bytes::Bytes;
use log::{debug, info, warn};
use rg_common::App;
use rg_net::{NetBufReader, PacketKind, read_connect, read_hello, read_ping};

use crate::{
    application::async_runtime::ServerChannel,
    error::AppError,
    server::{
        self,
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

        let security = ServerSecurity::new(cfg.key_bits, cfg.password.to_owned())?;

        drop(cfg);

        Ok(ServerState {
            config: Arc::clone(config),
            clients: Clients::new(),
            guests: Guests::new(),
            security,
            channel,
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
                    self.process_network_datagram(address, bytes);
                }
            }
        }

        self.clients.flush(&self.channel.tx);
        self.guests.flush(&self.channel.tx);

        Ok(())
    }

    fn process_network_datagram(&mut self, address: SocketAddr, bytes: Bytes) {
        let clients = &mut self.clients;
        let guests = &mut self.guests;
        let security = &self.security;

        let client_id = ClientId::new(address);
        let mut reader = NetBufReader::new(&bytes);

        while let Some((header, mut payload)) = reader.read_next_packet() {
            debug!("Got {:?} from client {}", header, address);

            match header.kind {
                PacketKind::Hello => {
                    if !clients.exists(&client_id) {
                        match read_hello(&mut payload) {
                            Ok(ref hello) => {
                                guests.on_hello(&client_id, hello, security.keys.public_key_bytes())
                            }
                            Err(e) => {
                                warn!("Failed to parse: {:?}", e)
                            }
                        }
                    }
                }

                PacketKind::Connect => match read_connect(&mut payload) {
                    Ok(ref connect) => match guests.on_connect(&client_id, connect, security) {
                        Ok(id) => {
                            if id.is_some() {
                                clients.add(client_id, connect.name);
                            }
                        }
                        Err(e) => {
                            warn!("Unable to connect client: {:?}", e);
                        }
                    },
                    Err(e) => {
                        warn!("Failed to parse: {}", e);
                    }
                },

                PacketKind::Ping => match read_ping(&mut payload) {
                    Ok(ref ping) => {
                        guests.on_ping(&client_id, ping);
                    }
                    Err(e) => {
                        warn!("Failed to parse: {:?}", e);
                    }
                },

                _ => {}
            }
        }
    }
}
