use core::error;
use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
};

use log::{error, warn};
use mio::net::UdpSocket;
use rg_net::{
    net_rw::{try_write, NetBufWriter, NetWriter},
    protocol::{ProtocolError, MAX_DATAGRAM_SIZE},
    server_info::write_server_info,
};

use super::sv_clients::ClientId;

pub(super) struct Guest {
    send_buf: VecDeque<Vec<u8>>,
}

impl Guest {
    pub fn new() -> Self {
        Self {
            send_buf: VecDeque::new(),
        }
    }

    pub fn send_nello(&mut self, key: &[u8]) {
        self.try_send(|w| write_server_info(w, key));
    }

    pub fn send_rejected(&mut self) {}

    pub fn flush(&mut self, addr: SocketAddr, socket: &UdpSocket) {
        while let Some(buf) = self.send_buf.pop_front() {
            match socket.send_to(buf.as_slice(), addr) {
                Ok(amount) => {
                    if amount < buf.len() {
                        warn!("Partial send: {} of {}", amount, buf.len());
                    }
                }
                Err(e) => {
                    self.send_buf.push_front(buf);
                    error!("Failed to send to client {}: {:?}", addr, e);
                }
            }
        }
    }

    ///
    /// Calls [handler] for last send buffer (and if that fails adds new buffer and retries).
    ///
    fn try_send<H>(&mut self, mut handler: H) -> Result<(), ProtocolError>
    where
        H: FnMut(&mut NetBufWriter) -> Result<(), ProtocolError>,
    {
        for _ in 0..2 {
            if let Some(buf) = self.send_buf.back_mut() {
                match try_write(buf, &mut handler) {
                    Ok(flag) => {
                        if flag {
                            break;
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
            self.send_buf
                .push_back(Vec::with_capacity(MAX_DATAGRAM_SIZE));
        }
        Ok(())
    }
}

pub(super) struct Guests {
    guests: HashMap<ClientId, Guest>,
}

impl Guests {
    pub fn new() -> Self {
        Self {
            guests: HashMap::new(),
        }
    }

    pub fn get_or_create(&mut self, id: ClientId) -> &mut Guest {
        self.guests.entry(id).or_insert_with(|| Guest::new())
    }

    pub fn flush(&mut self, socket: &UdpSocket) {
        for (client_id, guest) in self.guests.iter_mut() {
            guest.flush(client_id.0, socket);
        }
    }
}
