use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io;
use std::io::ErrorKind::UnexpectedEof;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::ops::Deref;
use std::time::Instant;

use anyhow::__private::kind::{AdhocKind, TraitKind};
use anyhow::Error;
use log::{error, info, warn};
use rmp_serde::decode::Error::InvalidMarkerRead;
use rmp_serde::Deserializer;
use serde::Deserialize;

use core::arguments::Arguments;

use crate::net::{Connect, Endpoint, MAX_DATAGRAM_SIZE, Message};

#[derive(Debug, Eq, PartialEq, Hash)]
struct ClientId(SocketAddr);

#[derive(Debug)]
struct Client {
    name: String,
    last_seen: Instant,
    endpoint: Endpoint,
}

impl Client {
    fn send(&mut self, msg: &Message) -> anyhow::Result<usize> {
        self.endpoint.send(msg)
    }

    fn clear_buffers(&mut self) {
        self.endpoint.clear_buffers();
    }

    fn flush(&mut self) -> anyhow::Result<usize> {
        self.endpoint.flush()
    }

    fn process_message(&self, msg: Message) -> anyhow::Result<()> {
        info!("Got from client {msg:?}");
        Ok(())
    }

    fn update(&mut self) -> anyhow::Result<()> {
        self.clear_buffers();
        loop {
            let buf = Vec::with_capacity(MAX_DATAGRAM_SIZE);
            if let Some((res_buf, addr)) = self.endpoint.receive(buf)? {
                self.last_seen = Instant::now();
                info!("Got {:?} bytes from {:?}", res_buf.len(), addr);
                let mut des = Deserializer::from_read_ref(&res_buf);
                loop {
                    match Message::deserialize(&mut des) {
                        Ok(msg) => {
                            self.process_message(msg)?;
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
            } else {
                break;
            }
        }
        Ok(())
    }
}

pub(crate) struct Server {
    endpoint: Endpoint,
    clients: HashMap<ClientId, Client>,
}

impl Server {
    pub(crate) fn update(&mut self) -> anyhow::Result<()> {
        self.listen()
    }

    pub fn local_address(&self) -> io::Result<SocketAddr> {
        self.endpoint.socket().local_addr()
    }

    pub fn new(args: &Arguments) -> Self {
        info!("Starting server...");
        let endpoint = Endpoint::new().expect("Unable to create server endpoint!");
        Server {
            endpoint,
            clients: HashMap::new(),
        }
    }

    fn on_connect(&mut self, key: ClientId, data: &Connect, addr: SocketAddr) -> anyhow::Result<()> {
        // todo - check password
        match self.clients.entry(key) {
            Entry::Vacant(v) => {
                let endpoint = self.endpoint.try_clone()?;
                endpoint.connect(addr)?;
                let client = v.insert(Client {
                    name: data.name.clone(),
                    last_seen: Instant::now(),
                    endpoint,
                });
                client.send(&Message::Accepted).map(|_| ())
            }
            Entry::Occupied(ref mut o) => {
                o.get_mut().last_seen = Instant::now();
                Ok(())
            }
        }
    }

    fn pass_to_client(&mut self, key: ClientId, msg: Message) -> anyhow::Result<()> {
        if let Entry::Occupied(ref mut o) = self.clients.entry(key) {
            o.get_mut().process_message(msg)
            //o.get_mut().last_seen = Instant::now();
        } else {
            Ok(())
        }
    }

    fn process_message(&mut self, msg: Message, addr: SocketAddr) -> anyhow::Result<()> {
        let key = ClientId(addr);
        match msg {
            Message::Connect(ref conn) => {
                self.on_connect(key, conn, addr)
            }
            other => {
                self.pass_to_client(key, other)
            }
        }
    }

    pub fn listen(&mut self) -> anyhow::Result<()> {
        let mut buf = Vec::new();
        buf.resize(MAX_DATAGRAM_SIZE, 0);
        for (_, mut c) in &mut self.clients {
            c.update()?;
        }
        if let Some((res_buf, addr)) = self.endpoint.receive(buf)? {
            info!("Got {:?} bytes from {:?}", res_buf.len(), addr);
            let mut des = Deserializer::from_read_ref(&res_buf);
            loop {
                match Message::deserialize(&mut des) {
                    Ok(msg) => {
                        self.process_message(msg, addr)?;
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
        for (id, mut c) in &mut self.clients {
            match c.flush() {
                Ok(_) => {}
                Err(e) => {
                    warn!("Flush failed for {id:?}: {e:?}");
                }
            }
        }
        Ok(())
    }
}
