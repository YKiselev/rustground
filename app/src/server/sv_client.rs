use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use bytes::BytesMut;
use log::debug;
use rg_net::MAX_DATAGRAM_SIZE;

use crate::server;

const BUF_ALLOCATOR_SIZE: usize = 8 * MAX_DATAGRAM_SIZE;

#[derive(Debug)]
pub struct Client {
    name: String,
    last_seen: Instant,
    send_buf: VecDeque<BytesMut>,
    buf_allocator: BytesMut
}

impl Client {
    pub fn new(name: &str) -> Self {
        Client {
            name: name.to_string(),
            last_seen: Instant::now(),
            send_buf: VecDeque::new(),
            buf_allocator: BytesMut::with_capacity(BUF_ALLOCATOR_SIZE)
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
