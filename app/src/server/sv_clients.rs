use std::{collections::{hash_map::Entry, HashMap}, net::SocketAddr, sync::mpsc::Sender};



use super::{sv_client::Client, sv_poll::Packet};

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub(crate) struct ClientId(pub SocketAddr);

impl ClientId {
    pub(crate) fn new(addr: SocketAddr) -> Self {
        Self(addr)
    }
}

#[derive(Debug)]
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

    pub fn flush(&mut self, tx: &Sender<Packet>) {
        for (client_id, client) in self.clients.iter_mut() {
            client.flush(client_id.0, tx);
        }
    }

    pub fn add(&mut self, client_id: ClientId, name: &str) {
        match self.clients.entry(client_id) {
            Entry::Vacant(v) => {
                let client = v.insert(Client::new(name));
            }
            Entry::Occupied(ref mut o) => {
                o.get_mut().touch();
            }
        }
    }
}
