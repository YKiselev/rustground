use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::from_utf8;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use log::{error, info, warn};
use rg_net::header::read_header;
use rg_net::hello::read_hello;
use rg_net::net_rw::{NetBufReader, NetReader, WithPosition};
use rg_net::protocol::{Header, Hello, PacketKind, MIN_HEADER_SIZE};

use crate::app::App;
use crate::error::AppError;
use crate::server::key_pair::KeyPair;
use crate::server::sv_client::Client;
use crate::server::sv_guests::Guests;
use crate::server::sv_poll::ServerPollThread;

use super::key_pair::KeyPairError;
use super::messages::sv_hello::on_hello;
use super::sv_clients::{ClientId, Clients};
use super::sv_error::ServerError;
use super::sv_poll::Packet;

pub(crate) struct Server {
    poll_thread: ServerPollThread,
    clients: Clients,
    guests: Guests,
    keys: KeyPair,
    password: Option<String>,
}

impl Server {
    pub fn new(app: &Arc<App>) -> Result<Self, ServerError> {
        info!("Starting server...");
        let mut cfg_guard = app.config().lock()?;
        let cfg = &mut cfg_guard.server;
        let addr: SocketAddr = cfg.address.parse()?;
        let poll_thread = ServerPollThread::new(addr, app.exit_flag())?;
        let keys = KeyPair::new(cfg.key_bits)?;
        let password = cfg.password.to_owned();
        let server_address = poll_thread.local_addr()?;
        info!("Server bound to {:?}", server_address);
        cfg.bound_to = Some(server_address.to_string());
        Ok(Server {
            poll_thread,
            clients: Clients::new(),
            guests: Guests::new(),
            keys,
            password,
        })
    }

    pub(crate) fn update(&mut self) -> Result<(), AppError> {
        let rx = &mut self.poll_thread.rx;
        let clients = &mut self.clients;
        let guests = &mut self.guests;

        for p in rx.try_iter() {
            let _ = process_packet(&p, |client_id, header, reader| {
                match header.kind {
                    PacketKind::Hello => {
                        if clients.exists(&client_id) {
                            false
                        } else {
                            if let Ok(ref hello) = read_hello(reader) {
                                on_hello(&client_id, hello, guests, self.keys.public_key_bytes());
                                true
                            } else {
                                false
                            }
                        }
                    }

                    PacketKind::Connect => {
                        //
                        true
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

    fn check_password(&self, encoded: &[u8]) -> bool {
        if let Some(password) = &self.password {
            return self.keys.decode(encoded).map_or_else(
                |_| false,
                |v| from_utf8(&v).map_or_else(|_| false, |p| password.eq(p)),
            );
        }
        true
    }

    // fn on_connect(
    //     &mut self,
    //     key: ClientId,
    //     name: &str,
    //     password: &[u8],
    //     addr: &SocketAddr,
    // ) -> Result<(), AppError> {
    //     if !self.check_password(password) {
    //         info!("Wrong password from {:?}!", addr);
    //         return Ok(());
    //     }
    //     match self.clients.entry(key) {
    //         Entry::Vacant(v) => {
    //             let endpoint = self.endpoint.try_clone_and_connect(addr)?;
    //             let client = v.insert(Client::new(name, endpoint));
    //             client.send(&Message::Accepted).map(|_| ())?;
    //             Ok(())
    //         }
    //         Entry::Occupied(ref mut o) => {
    //             o.get_mut().touch();
    //             Ok(())
    //         }
    //     }
    // }

    // fn pass_to_client(&mut self, key: ClientId, msg: &Message) -> Result<(), AppError> {
    //     if let Entry::Occupied(ref mut o) = self.clients.entry(key) {
    //         o.get_mut().process_message(msg)
    //     } else {
    //         Ok(())
    //     }
    // }
}


fn process_packet<H>(packet: &Packet, mut handler: H) -> Result<(), AppError>
where
    H: FnMut(ClientId, &Header, &mut NetBufReader) -> bool,
{
    let key = ClientId::new(packet.address);
    let mut reader = NetBufReader::new(packet.bytes.as_slice());

    while reader.available() > MIN_HEADER_SIZE {
        match read_header(&mut reader) {
            Ok(header) => {
                let amount = header.size as usize;
                info!("Got packet {:?} from {}", header.kind, packet.address);
                let mark = reader.pos();
                if !handler(key, &header, &mut reader) {
                    if let Err(e) = reader.set_pos(mark + amount) {
                        error!("Failed to skip packet: {e:?}");
                    }
                }
            }
            Err(e) => {
                error!("Failed to read client packet: {e:?}");
                break;
            }
        }
    }
    Ok(())
}
