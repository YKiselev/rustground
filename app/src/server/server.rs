use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io;
use std::io::ErrorKind::UnexpectedEof;
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::ops::Deref;
use std::str::from_utf8;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use anyhow::__private::kind::{AdhocKind, TraitKind};
use anyhow::Error;
use log::{error, info, warn};
use rsa::RsaPublicKey;
use rsa::traits::PublicKeyParts;

use common::arguments::Arguments;

use crate::config::{Config, ServerConfig};
use crate::net::{Endpoint, MAX_DATAGRAM_SIZE, Message, NetEndpoint, ServerEndpoint};
use crate::server::key_pair::KeyPair;
use crate::server::sv_client::Client;

#[derive(Debug, Eq, PartialEq, Hash)]
struct ClientId(SocketAddr);

pub(crate) struct Server {
    endpoint: Box<dyn ServerEndpoint + Send + Sync>,
    recv_buf: Option<Vec<u8>>,
    clients: HashMap<ClientId, Client>,
    keys: KeyPair,
    password: Option<String>,
    exit_flag: AtomicBool,
}

impl Server {
    pub(crate) fn update(&mut self) -> anyhow::Result<()> {
        let mut buf = self.recv_buf.take().unwrap_or_else(|| Vec::new());

        for (_, c) in self.clients.iter_mut() {
            c.update(&mut buf)?;
        }

        self.listen(&mut buf)?;

        for (id, c) in self.clients.iter_mut() {
            if let Err(e) = c.flush() {
                warn!("Flush failed for {id:?}: {e:?}");
            }
        }

        self.recv_buf.replace(buf);
        Ok(())
    }

    pub(crate) fn is_exit(&self) -> bool {
        self.exit_flag.load(Ordering::Relaxed)
    }

    pub(crate) fn shutdown(&mut self) {
        self.exit_flag.store(true, Ordering::Release);
    }

    pub fn new(cfg: &ServerConfig) -> Self {
        info!("Starting server...");
        let addr: SocketAddr = cfg.address.parse().expect("Invalid address!");
        let endpoint = NetEndpoint::with_address(addr).expect("Unable to create server endpoint!");
        let keys = KeyPair::new(cfg.key_bits).expect("Unable to create server key!");
        info!("Server bound to {:?}", endpoint.local_addr().expect("Unable to get server address!"));
        Server {
            endpoint: Box::new(endpoint),
            recv_buf: Some(Vec::with_capacity(MAX_DATAGRAM_SIZE)),
            clients: HashMap::new(),
            keys,
            password: cfg.password.to_owned(),
            exit_flag: AtomicBool::new(false),
        }
    }

    fn check_password(&self, encoded: &[u8]) -> bool {
        if let Some(password) = &self.password {
            return self.keys.decode(encoded)
                .map_err(|e| anyhow::Error::from(e))
                .and_then(|v| from_utf8(&v)
                    .map(|p| password.eq(p))
                    .map_err(|e| anyhow::Error::from(e))
                ).unwrap_or(false);
        }
        true
    }

    fn on_connect(&mut self, key: ClientId, name: &str, password: &[u8], addr: &SocketAddr) -> anyhow::Result<()> {
        if !self.check_password(password) {
            info!("Wrong password from {:?}!", addr);
            return Ok(());
        }
        match self.clients.entry(key) {
            Entry::Vacant(v) => {
                let endpoint = self.endpoint.try_clone_and_connect(addr)?;
                //endpoint.connect(addr)?;
                let client = v.insert(Client::new(name, endpoint));
                client.send(&Message::Accepted).map(|_| ())
            }
            Entry::Occupied(ref mut o) => {
                o.get_mut().touch();
                Ok(())
            }
        }
    }

    fn pass_to_client(&mut self, key: ClientId, msg: &Message) -> anyhow::Result<()> {
        if let Entry::Occupied(ref mut o) = self.clients.entry(key) {
            o.get_mut().process_message(msg)
        } else {
            Ok(())
        }
    }

    fn process_message(&mut self, msg: &Message, addr: &SocketAddr) -> anyhow::Result<()> {
        let key = ClientId(*addr);
        match msg {
            Message::Connect { name, password } => {
                self.on_connect(key, name, password, addr)
            }
            Message::Hello => {
                let key = bitcode::serialize(self.keys.public_key()).unwrap();
                self.endpoint.send_to(&Message::ServerInfo { key }, addr)?;
                Ok(())
            }
            other => {
                self.pass_to_client(key, other)
            }
        }
    }

    pub fn listen(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        loop {
            match self.endpoint.receive_data(buf.as_mut()) {
                Ok(Some(mut data)) => {
                    let addr = data.addr;
                    while let Some(ref m) = data.read() {
                        self.process_message(m, &addr).unwrap();
                    }
                }
                Ok(None) => {
                    break;
                }
                Err(e) => {
                    error!("Failed to receive from client: {:?}", e);
                    break;
                }
            }
        }
        Ok(())
    }
}
