use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::from_utf8;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use log::{error, info, warn};

use crate::app::App;
use crate::error::AppError;
use crate::server::key_pair::KeyPair;
use crate::server::sv_client::Client;
use crate::server::sv_poll::ServerPollThread;

use super::key_pair::KeyPairError;
use super::sv_error::ServerError;
use super::sv_poll::Packet;

#[derive(Debug, Eq, PartialEq, Hash)]
pub(crate) struct ClientId(SocketAddr);

pub(crate) struct Server {
    poll_thread: ServerPollThread,
    clients: HashMap<ClientId, Client>,
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
            clients: HashMap::new(),
            keys,
            password,
        })
    }

    pub(crate) fn update(&mut self) -> Result<(), AppError> {
        for p in self.poll_thread.rx.iter() {

        }

        self.poll_thread.update();
        //let mut buf = self.recv_buf.take().unwrap_or_else(|| Vec::new());

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

    // fn process_packet(&mut self, packet: &Packet) -> Result<(), AppError> {
    //     let key = ClientId(packet.address);
    //     match msg {
    //         Message::Connect { name, password } => self.on_connect(key, name, password, addr),
    //         Message::Hello => {
    //             let key = bitcode::serialize(self.keys.public_key()).unwrap();
    //             self.endpoint.send_to(&Message::ServerInfo { key }, addr)?;
    //             Ok(())
    //         }
    //         other => self.pass_to_client(key, other),
    //     }
    // }
}
