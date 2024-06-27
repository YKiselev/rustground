use std::io::ErrorKind::UnexpectedEof;
use std::time::Instant;

use log::info;
use rmp_serde::decode::Error::InvalidMarkerRead;
use rmp_serde::Deserializer;
use serde::Deserialize;

use crate::net::{Endpoint, MAX_DATAGRAM_SIZE, Message};

#[derive(Debug)]
pub struct Client {
    name: String,
    last_seen: Instant,
    endpoint: Endpoint,
}

impl Client {
    pub fn new(name: &str, endpoint: Endpoint) -> Self {
        Client {
            name: name.to_string(),
            last_seen: Instant::now(),
            endpoint,
        }
    }

    pub(crate) fn touch(&mut self) {
        self.last_seen = Instant::now();
    }

    pub(crate) fn send(&mut self, msg: &Message) -> anyhow::Result<usize> {
        self.endpoint.send(msg)
    }

    fn clear_buffers(&mut self) {
        self.endpoint.clear_buffers();
    }

    pub(crate) fn flush(&mut self) -> anyhow::Result<usize> {
        self.endpoint.flush()
    }

    pub(crate) fn process_message(&mut self, msg: Message) -> anyhow::Result<()> {
        self.touch();
        info!("Got from connected client: {msg:?}");
        Ok(())
    }

    pub(crate) fn update(&mut self) -> anyhow::Result<()> {
        self.clear_buffers();
        loop {
            let mut buf = Vec::with_capacity(MAX_DATAGRAM_SIZE);
            buf.resize(MAX_DATAGRAM_SIZE, 0);
            if let Some((amount, addr)) = self.endpoint.receive(&mut buf)? {
                buf.truncate(amount);
                self.last_seen = Instant::now();
                info!("Got {:?} bytes from {:?}", amount, addr);
                let mut des = Deserializer::from_read_ref(&buf);
                loop {
                    match Message::deserialize(&mut des) {
                        Ok(msg) => {
                            self.process_message(msg)?;
                        }
                        Err(InvalidMarkerRead(io_err)) => {
                            if io_err.kind() == UnexpectedEof {
                                break;
                            } else {
                                return Err(anyhow::Error::from(io_err));
                            }
                        }
                        Err(e) => {
                            return Err(anyhow::Error::from(e));
                        }
                    }
                }
            } else {
                break;
            }
        }
        Ok(())
    }
}
