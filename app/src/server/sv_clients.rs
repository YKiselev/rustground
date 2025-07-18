use std::{collections::{hash_map::Entry, HashMap}, net::SocketAddr};

use log::{error, info};
use rg_net::{
    header::read_header,
    net_rw::{NetBufReader, NetReader, WithPosition},
    protocol::{Header, PacketKind, MIN_HEADER_SIZE},
};

use crate::error::AppError;

use super::{sv_client::Client, sv_poll::Packet};

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
        false
    }

    pub fn update(&mut self) {}

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
