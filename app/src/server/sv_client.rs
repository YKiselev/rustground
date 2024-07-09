use std::io::ErrorKind::UnexpectedEof;
use std::time::Instant;

use chrono::Utc;
use log::info;
use rmp_serde::decode::Error::InvalidMarkerRead;
use rmp_serde::Deserializer;
use serde::Deserialize;

use crate::net::{Endpoint, MAX_DATAGRAM_SIZE, Message, process_messages, TimeData};
use crate::net::Message::{Ping, Pong};

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

    pub(crate) fn process_message(&mut self, msg: &Message) -> anyhow::Result<()> {
        self.touch();
        info!("Got from connected client: {msg:?}");
        match msg {
            // Message::Ack(_) => {}
            // Message::Connect(_) => {}
            // Message::Accepted => {}
            // Message::Hello => {}
            Pong(td) => {
                info!("Ping to client is {:.6} sec.", Instant::now().elapsed().as_secs_f64() - td.time)
            }
            Ping(td) => {
                self.endpoint.send(&Pong(TimeData { time: td.time }))?;
            }
            m => {
                info!("Ignoring unsupported message: {m:?}");
            }
        }
        Ok(())
    }

    pub(crate) fn update(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        self.clear_buffers();
        loop {
            //let mut buf = Vec::with_capacity(MAX_DATAGRAM_SIZE);
            buf.resize(MAX_DATAGRAM_SIZE, 0);
            if let Some((amount, addr)) = self.endpoint.receive(buf)? {
                buf.truncate(amount);
                self.last_seen = Instant::now();
                info!("Got {:?} bytes from {:?}", amount, addr);
                process_messages(&buf, |m| self.process_message(m))?;
            } else {
                break;
            }
        }
        Ok(())
    }
}
