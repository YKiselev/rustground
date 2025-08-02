use std::net::SocketAddr;
use std::sync::Arc;

use log::{debug, info, warn};
use rg_common::App;
use rg_net::process_buf;
use rg_net::read_connect;
use rg_net::read_hello;
use rg_net::read_ping;
use rg_net::NetBufReader;
use rg_net::PacketKind;

use crate::error::AppError;
use crate::server::key_pair::KeyPair;
use crate::server::messages::sv_ping::on_ping;
use crate::server::sv_guests::Guests;
use crate::server::sv_poll::ServerPoll;

use super::messages::sv_connect::on_connect;
use super::messages::sv_hello::on_hello;
use super::sv_clients::{ClientId, Clients};

#[derive(Debug)]
pub(super) struct ServerSecurity {
    keys: KeyPair,
    password: Option<String>,
}

impl ServerSecurity {
    fn new(key_bits: usize, pwd: &Option<String>) -> Result<Self, AppError> {
        let keys = KeyPair::new(key_bits)?;
        Ok(Self {
            keys,
            password: pwd.to_owned(),
        })
    }

    pub fn decode(&self, value: &[u8]) -> Result<Vec<u8>, AppError> {
        self.keys.decode(value)
    }

    pub fn is_password_ok(&self, pwd: &[u8]) -> bool {
        if let Some(p) = self.password.as_ref() {
            p.as_bytes().eq(pwd)
        } else {
            pwd.is_empty()
        }
    }
}

#[derive(Debug)]
struct ServerState {
    poll_thread: ServerPoll,
    clients: Clients,
    guests: Guests,
    security: ServerSecurity,
}

impl ServerState {
    fn new(app: &App) -> Result<Self, AppError> {
        info!("Starting server...");
        let mut cfg_guard = app.config.lock()?;
        let cfg = &mut cfg_guard.server;
        let addr: SocketAddr = cfg.address.parse()?;
        let poll_thread = ServerPoll::new(addr)?;
        let security = ServerSecurity::new(cfg.key_bits, &cfg.password)?;
        let server_address = poll_thread.local_addr()?;
        info!("Server bound to {:?}", server_address);
        cfg.bound_to = Some(server_address.to_string());
        Ok(ServerState {
            poll_thread,
            clients: Clients::new(),
            guests: Guests::new(),
            security,
        })
    }

    fn shutdow(self) {
        self.poll_thread.shutdown();
    }

    fn update(&mut self) -> Result<(), AppError> {
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

#[derive(Default)]
pub(crate) struct Server(Option<ServerState>);

impl Server {
    pub fn init(&mut self, app: &Arc<App>) -> Result<(), AppError> {
        if self.0.is_none() {
            self.0 = Some(ServerState::new(app)?);
        }
        Ok(())
    }

    pub fn shutdown(&mut self) {
        if let Some(s) = self.0.take() {
            s.shutdow();
        }
    }

    pub(crate) fn update(&mut self) -> Result<(), AppError> {
        self.0
            .as_mut()
            .map(|state| state.update())
            .unwrap_or(Ok(()))
    }
}
