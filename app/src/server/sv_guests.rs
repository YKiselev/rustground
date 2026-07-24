use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use bytes::{Bytes, BytesMut};
use rg_net::{
    ClientInfo, Connect, Hello, MAX_DATAGRAM_SIZE, NetBufWriter, PROTOCOL_VERSION, PacketKind,
    Ping, ProtocolError, RejectionReason, try_write, write_accepted, write_pong, write_rejected,
    write_server_info, write_with_header,
};
use tracing::{debug, info, warn};
use x25519_dalek::{EphemeralSecret, PublicKey};

use crate::{
    cipher::Cipher, error::{AppError, to_illegal_state}, server::{self, sv_security::ServerSecurity},
};

use super::sv_clients::ClientId;

const OBSOLETE_AFTER: Duration = Duration::from_secs(2 * 60);
const BUF_ALLOCATOR_SIZE: usize = 8 * MAX_DATAGRAM_SIZE;

#[derive()]
pub(super) struct Guest {
    send_buf: VecDeque<BytesMut>,
    received_at: Option<Instant>,
    buf_allocator: BytesMut,
    keys: Option<(EphemeralSecret, PublicKey)>,
    cipher: Option<Cipher>,
}

impl Guest {
    pub fn new() -> Self {
        Self {
            send_buf: VecDeque::new(),
            received_at: None,
            buf_allocator: BytesMut::with_capacity(BUF_ALLOCATOR_SIZE),
            keys: None,
            cipher: None,
        }
    }

    pub fn send_server_info(&mut self) {
        if self.keys.is_none() {
            debug!("Initializing secret");
            self.init_keys();
        }

        debug!("Writing server info...");

        if let Some((_, key)) = self.keys {
            let _ = self
                .write_to_send_buf(|w| {
                    write_with_header(w, PacketKind::ServerInfo, |w| {
                        write_server_info(w, key.as_bytes())
                    })
                })
                .inspect_err(|e| warn!("Failed to write server info: {:?}", e));
        }
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
        static IDX: AtomicU64 = AtomicU64::new(1);

        while let Some(bytes) = self.send_buf.pop_front() {
            let len = bytes.len();
            let index = IDX.fetch_add(1, Ordering::Relaxed);
            debug!("Sending {} bytes #{} to channel", len, index);
            if let Err(_) = tx.send(server::Request::SendDatagram {
                addr,
                bytes: bytes.freeze(),
                index,
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

    pub fn init_keys(&mut self) {
        if self.keys.is_none() {
            let secret = EphemeralSecret::random();
            let public = PublicKey::from(&secret);
            self.keys = Some((secret, public));
        }
    }

    pub fn init_cipher(&mut self, client_public_key: PublicKey) {
        if let Some((secret, _)) = self.keys.take() {
            match Cipher::new(secret, &client_public_key) {
                Ok(cipher) => self.cipher = Some(cipher),
                Err(e) => warn!("Failed to create cipher: {:?}", e),
            }
        } else {
            warn!("No secret to create cipher!");
        }
    }

    pub fn try_connect(
        &mut self,
        client_id: &ClientId,
        connect: &Connect,
        security: &ServerSecurity,
    ) -> Result<Option<Cipher>, AppError> {
        if let Some(cipher) = self.cipher.as_mut() {
            let bytes = Bytes::copy_from_slice(connect.password);
            let decoded = cipher.decode(&bytes)?;

            if !security.is_password_ok(&decoded) {
                info!("Wrong password from client {:?}!", client_id);
                self.send_rejected(RejectionReason::Unauthorized);
                Ok(None)
            } else {
                self.send_accepted();
                Ok(self.cipher.take())
            }
        } else {
            Err(to_illegal_state("No cipher!"))
        }
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

    pub fn on_hello(&mut self, client_id: &ClientId, hello: &Hello) {
        let guest = self.get_or_create(*client_id);
        if hello.version.0 <= PROTOCOL_VERSION.0 && hello.version.1 <= PROTOCOL_VERSION.1 {
            guest.send_server_info();
        } else {
            guest.send_rejected(RejectionReason::UnsupportedVersion);
        }
    }

    pub fn on_client_info(&mut self, client_id: &ClientId, info: &ClientInfo) {
        let guest = self.get_or_create(*client_id);
        if info.key.len() != 32 {
            warn!(
                "Client key length mismatch: expected {} got {}",
                32,
                info.key.len()
            );
            return;
        }
        let bytes: [u8; 32] = info.key.try_into().unwrap();
        let client_public_key = PublicKey::from(bytes);

        guest.init_cipher(client_public_key);
    }

    pub fn on_connect(
        &mut self,
        client_id: &ClientId,
        connect: &Connect,
        security: &ServerSecurity,
    ) -> Result<Option<Cipher>, AppError> {
        let guest = self.get_or_create(*client_id);
        guest.try_connect(client_id, connect, security)
    }

    pub fn on_ping(&mut self, client_id: &ClientId, ping: &Ping) {
        let guest = self.get_or_create(*client_id);
        guest.send_pong(&ping);
    }

    fn cleanup(&mut self) {
        self.guests.retain(|_, v| !v.is_obsolete());
    }
}
