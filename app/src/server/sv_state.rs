use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use log::{debug, info, warn};
use rg_common::App;
use rg_net::{process_buf, read_connect, read_hello, read_ping, NetBufReader, PacketKind};

use crate::{
    error::AppError,
    server::{
        messages::{sv_connect::on_connect, sv_hello::on_hello, sv_ping::on_ping},
        server::ServerConfig,
        sv_clients::{ClientId, Clients},
        sv_guests::Guests,
        sv_poll::ServerPoll,
        sv_security::ServerSecurity,
    },
};

#[derive(Debug)]
pub(super) struct ServerState {
    poll_thread: ServerPoll,
    clients: Clients,
    guests: Guests,
    security: ServerSecurity,
}

impl ServerState {
    pub fn new(app: &App, config: &Arc<RwLock<ServerConfig>>) -> Result<Self, AppError> {
        info!("Starting server...");
        let cfg_guard = config.read()?;
        let cfg = &cfg_guard;
        let addr: SocketAddr = cfg.address.parse()?;
        let poll_thread = ServerPoll::new(addr)?;
        let security = ServerSecurity::new(cfg.key_bits, &cfg.password)?;
        let server_address = poll_thread.local_addr()?;
        info!("Server bound to {:?}", server_address);
        drop(cfg_guard);
        let mut cfg_guard = config.write()?;
        let cfg = &mut cfg_guard;
        cfg.bound_to = Some(server_address.to_string());
        Ok(ServerState {
            poll_thread,
            clients: Clients::new(),
            guests: Guests::new(),
            security,
        })
    }

    pub fn shutdow(self) {
        self.poll_thread.shutdown();
    }

    pub fn update(&mut self) -> Result<(), AppError> {
        let clients = &mut self.clients;
        let guests = &mut self.guests;
        let security = &self.security;

        for p in self.poll_thread.rx().try_iter() {
            let client_id = ClientId::new(p.address);
            let mut reader = NetBufReader::new(p.bytes.as_slice());
            let _ = process_buf(&mut reader, |header, reader| {
                debug!("Got {:?} from client {}", header, p.address);
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

        guests.flush(&self.poll_thread.tx());

        clients.flush(&self.poll_thread.tx());

        Ok(())
    }
}
