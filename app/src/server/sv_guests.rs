use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

use log::{error, info, warn};
use mio::net::UdpSocket;
use rg_net::{
    try_write, write_accepted, write_rejected, write_server_info, write_with_header, NetBufWriter,
    PacketKind, ProtocolError, RejectionReason, MAX_DATAGRAM_SIZE,
};

use super::{sv_clients::ClientId, sv_poll::Packet};

const OBSOLETE_AFTER: Duration = Duration::from_secs(2 * 60);

#[derive(Debug)]
pub(super) struct Guest {
    send_buf: VecDeque<Vec<u8>>,
    received_at: Option<Instant>,
}

impl Guest {
    pub fn new() -> Self {
        Self {
            send_buf: VecDeque::new(),
            received_at: None,
        }
    }

    pub fn send_server_info(&mut self, key: &[u8]) {
        let _ = self
            .write_to_send_buf(|w| {
                write_with_header(w, PacketKind::ServerInfo, |w| write_server_info(w, key))
            })
            .inspect_err(|e| warn!("Failed to write server info: {:?}", e));
    }

    pub fn send_rejected(&mut self, reason: RejectionReason) {
        let _ = self
            .write_to_send_buf(|w| {
                write_with_header(w, PacketKind::Accepted, |w| write_rejected(w, reason))
            })
            .inspect_err(|e| warn!("Failed to write server info: {:?}", e));
    }

    pub fn send_accepted(&mut self) {
        let _ = self
            .write_to_send_buf(|w| {
                write_with_header(w, PacketKind::Accepted, |w| write_accepted(w))
            })
            .inspect_err(|e| warn!("Failed to write server info: {:?}", e));
    }

    pub fn flush(&mut self, addr: SocketAddr, tx: &Sender<Packet>) {
        while let Some(buf) = self.send_buf.pop_front() {
            match tx.send(Packet {
                bytes: buf,
                address: addr,
            }) {
                Ok(_) => {}
                Err(_) => {
                    error!("Send channel is closed!");
                    break;
                }
            }
        }
    }

    pub fn is_obsolete(&self) -> bool {
        self.received_at
            .map(|v| v.elapsed() >= OBSOLETE_AFTER)
            .unwrap_or(false)
    }

    ///
    /// Calls [handler] for last send buffer (and if that fails due to overflow - adds new buffer and retries).
    ///
    fn write_to_send_buf<H>(&mut self, mut handler: H) -> Result<(), ProtocolError>
    where
        H: FnMut(&mut NetBufWriter) -> Result<(), ProtocolError>,
    {
        self.received_at = Some(Instant::now());
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

#[derive(Debug)]
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

    pub fn flush(&mut self, tx: &Sender<Packet>) {
        for (client_id, guest) in self.guests.iter_mut() {
            guest.flush(client_id.0, tx);
        }
        self.cleanup();
    }

    fn cleanup(&mut self) {
        self.guests.retain(|_, v| !v.is_obsolete());
    }
}
