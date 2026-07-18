use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::Instant;

use log::debug;

use crate::server;

#[derive(Debug)]
pub struct Client {
    name: String,
    last_seen: Instant,
    send_buf: VecDeque<Vec<u8>>,
}

impl Client {
    pub fn new(name: &str) -> Self {
        Client {
            name: name.to_string(),
            last_seen: Instant::now(),
            send_buf: VecDeque::new(),
        }
    }

    pub(crate) fn touch(&mut self) {
        self.last_seen = Instant::now();
    }

    pub fn flush(&mut self, addr: SocketAddr, tx: &flume::Sender<server::Request>) {
        while let Some(bytes) = self.send_buf.pop_front() {
            match tx.send(server::Request::SendDatagram { addr, bytes }) {
                Ok(_) => {}
                Err(_) => {
                    debug!("Send channel is closed!");
                    break;
                }
            }
        }
    }
}
