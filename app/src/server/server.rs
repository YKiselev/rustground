use std::net::SocketAddr;
use std::sync::Arc;

use log::info;
use rg_net::connect::read_connect;
use rg_net::hello::read_hello;
use rg_net::net_rw::NetBufReader;
use rg_net::process_buf;
use rg_net::protocol::PacketKind;

use crate::app::App;
use crate::error::AppError;
use crate::server::key_pair::KeyPair;
use crate::server::sv_guests::Guests;
use crate::server::sv_poll::ServerPollThread;

use super::messages::sv_connect::on_connect;
use super::messages::sv_hello::on_hello;
use super::sv_clients::{ClientId, Clients};

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

pub(crate) struct Server {
    poll_thread: ServerPollThread,
    clients: Clients,
    guests: Guests,
    security: ServerSecurity,
}

impl Server {
    pub fn new(app: &Arc<App>) -> Result<Self, AppError> {
        info!("Starting server...");
        let mut cfg_guard = app.config().lock()?;
        let cfg = &mut cfg_guard.server;
        let addr: SocketAddr = cfg.address.parse()?;
        let poll_thread = ServerPollThread::new(addr, app.exit_flag())?;
        let security = ServerSecurity::new(cfg.key_bits, &cfg.password)?;
        let server_address = poll_thread.local_addr()?;
        info!("Server bound to {:?}", server_address);
        cfg.bound_to = Some(server_address.to_string());
        Ok(Server {
            poll_thread,
            clients: Clients::new(),
            guests: Guests::new(),
            security,
        })
    }

    pub(crate) fn update(&mut self) -> Result<(), AppError> {
        let rx = &mut self.poll_thread.rx;
        let clients = &mut self.clients;
        let guests = &mut self.guests;
        let security = &self.security;

        for p in rx.try_iter() {
            let client_id = ClientId::new(p.address);
            let mut reader = NetBufReader::new(p.bytes.as_slice());
            let _ = process_buf(&mut reader, |header, reader| {
                info!("Got {:?} from {}", header, p.address);
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
                        if clients.exists(&client_id) {
                            false
                        } else if let Ok(ref connect) = read_connect(reader) {
                            on_connect(&client_id, connect, guests, clients, security);
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            });
        }

        guests.flush(self.poll_thread.socket.as_ref());

        // for (_, c) in self.clients.iter_mut() {
        //     c.update(&mut buf)?;
        // }

        //self.listen(&mut buf)?;

        // for (id, c) in self.clients.iter_mut() {
        //     if let Err(e) = c.flush() {
        //         warn!("Flush failed for {id:?}: {e:?}");
        //     }
        // }

        //self.recv_buf.replace(buf);
        Ok(())
    }

    // fn pass_to_client(&mut self, key: ClientId, msg: &Message) -> Result<(), AppError> {
    //     if let Entry::Occupied(ref mut o) = self.clients.entry(key) {
    //         o.get_mut().process_message(msg)
    //     } else {
    //         Ok(())
    //     }
    // }
}
