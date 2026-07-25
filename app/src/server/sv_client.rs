use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use bytes::BytesMut;
use tracing::debug;
use rg_net::MAX_DATAGRAM_SIZE;

use crate::cipher::Cipher;
use crate::server;

pub struct Client {
    name: String,
    last_seen: Instant,
    send_buf: VecDeque<BytesMut>,
    cipher: Cipher
}

impl Client {
    pub fn new(name: &str, cipher: Cipher) -> Self {
        Client {
            name: name.to_string(),
            last_seen: Instant::now(),
            send_buf: VecDeque::new(),
            cipher
        }
    }

    pub(crate) fn touch(&mut self) {
        self.last_seen = Instant::now();
    }

    pub fn flush(&mut self, addr: SocketAddr, tx: &flume::Sender<server::Request>) {
        static IDX: AtomicU64 = AtomicU64::new(1);
        
        while let Some(bytes) = self.send_buf.pop_front() {
            let index = IDX.fetch_add(1, Ordering::Relaxed);
            match tx.send(server::Request::SendDatagram { addr, bytes: bytes.freeze(), index }) {
                Ok(_) => {}
                Err(_) => {
                    debug!("Send channel is closed!");
                    break;
                }
            }
        }
    }
}
