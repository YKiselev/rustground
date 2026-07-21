use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    time::{Duration, Instant},
};

use bytes::BytesMut;
use log::{debug, info, warn};
use rg_net::{
    Connect, Hello, MAX_DATAGRAM_SIZE, NetBufWriter, PROTOCOL_VERSION, PacketKind, Ping,
    ProtocolError, RejectionReason, try_write, write_accepted, write_pong, write_rejected,
    write_server_info, write_with_header,
};

use crate::{
    error::AppError,
    server::{self, sv_security::ServerSecurity},
};

use super::sv_clients::ClientId;

const OBSOLETE_AFTER: Duration = Duration::from_secs(2 * 60);
const BUF_ALLOCATOR_SIZE: usize = 8 * MAX_DATAGRAM_SIZE;

#[derive()]
pub(super) struct Guest {
    send_buf: VecDeque<BytesMut>,
    received_at: Option<Instant>,
    buf_allocator: BytesMut,
}

impl Guest {
    pub fn new() -> Self {
        Self {
            send_buf: VecDeque::new(),
            received_at: None,
            buf_allocator: BytesMut::with_capacity(BUF_ALLOCATOR_SIZE),
        }
    }

    pub fn send_server_info(&mut self, key: &[u8]) {
        debug!("Writing server info...");
        let _ = self
            .write_to_send_buf(|w| {
                write_with_header(w, PacketKind::ServerInfo, |w| write_server_info(w, key))
            })
            .inspect_err(|e| warn!("Failed to write server info: {:?}", e));
    }

    pub fn send_rejected(&mut self, reason: RejectionReason) {
        let _ = self
            .write_to_send_buf(|w| {
                write_with_header(w, PacketKind::Rejected, |w| write_rejected(w, reason))
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

    pub fn send_pong(&mut self, ping: &Ping) {
        let _ = self
            .write_to_send_buf(|w| {
                write_with_header(w, PacketKind::Pong, |w| write_pong(w, ping.time))
            })
            .inspect_err(|e| warn!("Failed to write pong: {:?}", e));
    }

    pub fn flush(&mut self, addr: SocketAddr, tx: &flume::Sender<server::Request>) {
        while let Some(bytes) = self.send_buf.pop_front() {
            let len = bytes.len();
            debug!("Sending {} bytes to channel", len);
            if let Err(_) = tx.send(server::Request::SendDatagram {
                addr,
                bytes: bytes.freeze(),
            }) {
                debug!("Send channel is closed!");
                break;
            }
            debug!("Sent {} bytes to channel", len);
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

            if !self.buf_allocator.try_reclaim(MAX_DATAGRAM_SIZE) {
                warn!("Unable to reclaim {} bytes", MAX_DATAGRAM_SIZE);
            }

            let rest = self.buf_allocator.split_off(MAX_DATAGRAM_SIZE);
            let new_buf = std::mem::replace(&mut self.buf_allocator, rest);
            self.send_buf.push_back(new_buf);
        }
        Ok(())
    }
}

#[derive()]
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

    pub fn flush(&mut self, tx: &flume::Sender<server::Request>) {
        for (client_id, guest) in self.guests.iter_mut() {
            guest.flush(client_id.0, tx);
        }
        self.cleanup();
    }

    pub fn on_hello(&mut self, client_id: &ClientId, hello: &Hello, key: &[u8]) {
        let guest = self.get_or_create(*client_id);
        if hello.version.0 <= PROTOCOL_VERSION.0 && hello.version.1 <= PROTOCOL_VERSION.1 {
            guest.send_server_info(key);
        } else {
            guest.send_rejected(RejectionReason::UnsupportedVersion);
        }
    }

    pub fn on_connect(
        &mut self,
        client_id: &ClientId,
        connect: &Connect,
        security: &ServerSecurity,
    ) -> Result<Option<ClientId>, AppError> {
        let decoded = security.decode(connect.password)?;
        let guest = self.get_or_create(*client_id);
        if !security.is_password_ok(&decoded) {
            info!("Wrong password from client {:?}!", client_id);
            guest.send_rejected(RejectionReason::Unauthorized);
            Ok(None)
        } else {
            guest.send_accepted();
            Ok(Some(*client_id))
        }
    }

    pub fn on_ping(&mut self, client_id: &ClientId, ping: &Ping) {
        let guest = self.get_or_create(*client_id);
        guest.send_pong(&ping);
    }

    fn cleanup(&mut self) {
        self.guests.retain(|_, v| !v.is_obsolete());
    }
}
