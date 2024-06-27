use std::cmp::min;
use std::fmt::Debug;
use std::io::Error;
use std::io::ErrorKind::WouldBlock;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};

pub const MAX_DATAGRAM_SIZE: usize = 65507;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckData(pub u8);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectData {
    pub name: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "id")]
pub enum Message {
    Ack(AckData),
    Connect(ConnectData),
    Accepted,
}

#[derive(Debug)]
pub struct Endpoint {
    socket: UdpSocket,
    send_buf: Vec<u8>,
}

impl Endpoint {
    pub fn new() -> anyhow::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        socket.set_nonblocking(true)?;
        Ok(Endpoint {
            socket,
            send_buf: Vec::with_capacity(MAX_DATAGRAM_SIZE),
        })
    }

    pub fn try_clone(&self) -> anyhow::Result<Self> {
        let socket = self.socket.try_clone()?;
        Ok(Endpoint {
            socket,
            send_buf: Vec::with_capacity(MAX_DATAGRAM_SIZE),
        })
    }

    pub fn connect(&self, addr: SocketAddr) -> anyhow::Result<()> {
        self.socket.connect(addr).map_err(|e| anyhow::Error::from(e))
    }

    pub fn socket(&self) -> &UdpSocket {
        &self.socket
    }

    pub fn clear_buffers(&mut self) {
        self.send_buf.clear();
    }

    pub fn take_error(&self) -> anyhow::Result<Option<Error>> {
        self.socket.take_error().map_err(|e| anyhow::Error::from(e))
    }

    fn flush_exact(&mut self, amount: usize) -> anyhow::Result<usize> {
        let buf = &mut self.send_buf;
        assert!(amount <= buf.len());
        assert!(amount <= MAX_DATAGRAM_SIZE);
        let mut left = amount;
        while left > 0 {
            match self.socket.send(&buf[..left]) {
                Ok(written) => {
                    left -= written;
                    buf.drain(..written);
                }
                Err(e) => {
                    if e.kind() == WouldBlock {
                        // todo?
                        std::thread::yield_now();
                    } else {
                        return Err(anyhow::Error::from(e));
                    }
                }
            }
        }
        Ok(amount - left)
    }

    pub fn flush(&mut self) -> anyhow::Result<usize> {
        let buf = &self.send_buf;
        assert!(buf.len() <= MAX_DATAGRAM_SIZE);
        self.flush_exact(min(buf.len(), MAX_DATAGRAM_SIZE))
    }

    pub fn send_to(&mut self, msg: &Message, addr: &SocketAddr) -> anyhow::Result<usize> {
        let mut buf = Vec::new();
        let mut ser = Serializer::new(&mut buf);
        msg.serialize(&mut ser).map_err(|e| anyhow::Error::from(e))?;
        Ok(self.socket.send_to(buf.as_slice(), addr).map_err(anyhow::Error::from)?)
    }
    pub fn send(&mut self, msg: &Message) -> anyhow::Result<usize> {
        let mut buf = &mut self.send_buf;
        let before = buf.len();
        let mut ser = Serializer::new(&mut buf);
        match msg.serialize(&mut ser).map_err(|e| anyhow::Error::from(e)) {
            Ok(_) => {}
            Err(e) => {
                buf.truncate(before);
                return Err(e);
            }
        }
        if buf.len() >= MAX_DATAGRAM_SIZE {
            self.flush()?;
        }
        Ok(0)
    }

    pub fn receive(&self, mut buf: Vec<u8>) -> anyhow::Result<Option<(Vec<u8>, SocketAddr)>> {
        match self.socket.recv_from(&mut buf) {
            Ok((amount, addr)) => {
                if amount > 0 {
                    buf.resize(amount, 0);
                    Ok(Some((buf, addr)))
                } else { Ok(None) }
            }
            Err(e) => return if e.kind() == WouldBlock {
                Ok(None) // no data yet
            } else {
                Err(anyhow::Error::from(e))
            }
        }
    }
}