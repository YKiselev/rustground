use std::{
    collections::{HashMap, hash_map::Entry},
    net::SocketAddr,
};

use crate::{cipher::Cipher, server};

use super::sv_client::Client;

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub(crate) struct ClientId(pub SocketAddr);

impl ClientId {
    pub(crate) fn new(addr: SocketAddr) -> Self {
        Self(addr)
    }
}

pub(crate) struct Clients {
    clients: HashMap<ClientId, Client>,
}

impl Clients {
    pub(crate) fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub fn exists(&self, client_id: &ClientId) -> bool {
        self.clients.get(client_id).is_some()
    }

    pub fn flush(&mut self, tx: &flume::Sender<server::Request>) {
        for (client_id, client) in self.clients.iter_mut() {
            client.flush(client_id.0, tx);
        }
    }

    pub fn add(&mut self, client_id: ClientId, name: &str, cipher: Cipher) {
        match self.clients.entry(client_id) {
            Entry::Vacant(v) => {
                let _client = v.insert(Client::new(name, cipher));
            }
            Entry::Occupied(ref mut o) => {
                o.get_mut().touch();
            }
        }
    }
}
