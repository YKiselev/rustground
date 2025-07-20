use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::mpsc::Sender;
use std::time::Instant;


use log::error;

use super::sv_poll::Packet;

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
}
