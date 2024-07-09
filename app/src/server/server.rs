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
use rmp_serde::decode::Error::InvalidMarkerRead;
use rmp_serde::Deserializer;
use serde::Deserialize;

use core::arguments::Arguments;

use crate::config::{Config, ServerConfig};
use crate::net::{ConnectData, Endpoint, MAX_DATAGRAM_SIZE, Message, ServerInfoData};
use crate::server::key_pair::KeyPair;
use crate::server::sv_client::Client;

#[derive(Debug, Eq, PartialEq, Hash)]
struct ClientId(SocketAddr);

pub(crate) struct Server {
    endpoint: Endpoint,
    clients: HashMap<ClientId, Client>,
    keys: KeyPair,
    password: Option<String>,
    exit_flag: AtomicBool,
}

impl Server {
    pub(crate) fn update(&mut self) -> anyhow::Result<()> {
        let mut buf = Vec::with_capacity(MAX_DATAGRAM_SIZE);

        for (_, c) in self.clients.iter_mut() {
            c.update(&mut buf)?;
        }

        self.listen(&mut buf)?;

        for (id, c) in self.clients.iter_mut() {
            if let Err(e) = c.flush() {
                warn!("Flush failed for {id:?}: {e:?}");
            }
        }

        Ok(())
    }

    pub(crate) fn is_exit(&self) -> bool {
        self.exit_flag.load(Ordering::Relaxed)
    }

    pub(crate) fn shutdown(&mut self) {
        self.exit_flag.store(true, Ordering::Release);
    }

    pub fn local_address(&self) -> io::Result<SocketAddr> {
        self.endpoint.socket().local_addr()
    }

    pub fn new(cfg: &ServerConfig) -> Self {
        info!("Starting server...");
        let addr: SocketAddr = cfg.address.parse().expect("Invalid address!");
        let endpoint = Endpoint::with_address(addr).expect("Unable to create server endpoint!");
        let keys = KeyPair::new(cfg.key_bits).expect("Unable to create server key!");
        info!("Server bound to {:?}", endpoint.socket().local_addr().expect("Unable to get server address!"));
        Server {
            endpoint,
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

    fn on_connect(&mut self, key: ClientId, data: &ConnectData, addr: &SocketAddr) -> anyhow::Result<()> {
        if !self.check_password(data.password.as_ref()) {
            info!("Wrong password!");
            return Ok(());
        }
        match self.clients.entry(key) {
            Entry::Vacant(v) => {
                let endpoint = self.endpoint.try_clone()?;
                endpoint.connect(addr)?;
                let client = v.insert(Client::new(&data.name, endpoint));
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
            Message::Connect(ref conn) => {
                self.on_connect(key, conn, addr)
            }
            Message::Hello => {
                self.endpoint.send_to(&Message::ServerInfo(ServerInfoData { key: self.keys.public_key_as_pem() }), &addr)?;
                Ok(())
            }
            other => {
                self.pass_to_client(key, other)
            }
        }
    }

    pub fn listen(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        buf.resize(MAX_DATAGRAM_SIZE, 0);
        if let Some((amount, addr)) = self.endpoint.receive(buf)? {
            buf.truncate(amount);
            info!("Got {:?} bytes from {:?}", amount, addr);
            let mut des = Deserializer::from_read_ref(&buf);
            loop {
                match Message::deserialize(&mut des) {
                    Ok(msg) => {
                        self.process_message(&msg, &addr)?;
                    }
                    Err(InvalidMarkerRead(io_err)) => {
                        if io_err.kind() == UnexpectedEof {
                            break;
                        } else {
                            return Err(anyhow::Error::from(io_err));
                        }
                    }
                    Err(e) => {
                        return Err(anyhow::Error::from(e));
                    }
                }
            }
        }
        Ok(())
    }
}
