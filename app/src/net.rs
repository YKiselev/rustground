use std::cmp::min;
use std::fmt::Debug;
use std::io::{Error, Write};
use std::io::ErrorKind::WouldBlock;
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};

use bitcode::{Decode, Encode};
use bitcode::__private::{Decoder, View};

pub const MAX_DATAGRAM_SIZE: usize = 65507;


#[derive(Debug, Clone, Encode, Decode)]
pub enum Message<'a> {
    Ack,
    Connect { name: &'a str, password: Vec<u8> },
    Accepted,
    Hello,
    ServerInfo { key: Vec<u8> },
    Ping { time: f64 },
    Pong { time: f64 },
}

#[derive(Debug)]
pub struct Endpoint {
    socket: UdpSocket,
    send_buf: Vec<u8>,
    scratch: Vec<u8>,
}

fn write<W: Write>(msg: &Message, mut w: W) -> anyhow::Result<()> {
    w.write(&bitcode::encode(msg)).map_err(|e| anyhow::Error::from(e)).map(|amount| ())
    //let mut ser = Serializer::new(w);
    //msg.serialize(&mut ser).map_err(|e| anyhow::Error::from(e))
}

impl Endpoint {
    pub fn with_address<A: ToSocketAddrs>(addr: A) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;
        Ok(Endpoint {
            socket,
            send_buf: Vec::with_capacity(MAX_DATAGRAM_SIZE),
            scratch: Vec::with_capacity(MAX_DATAGRAM_SIZE),
        })
    }
    pub fn new() -> anyhow::Result<Self> {
        Self::with_address((Ipv4Addr::UNSPECIFIED, 0))
    }

    pub fn try_clone(&self) -> anyhow::Result<Self> {
        let socket = self.socket.try_clone()?;
        Ok(Endpoint {
            socket,
            send_buf: Vec::with_capacity(MAX_DATAGRAM_SIZE),
            scratch: Vec::with_capacity(MAX_DATAGRAM_SIZE),
        })
    }

    pub fn connect(&self, addr: &SocketAddr) -> anyhow::Result<()> {
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
        let mut buf = &mut self.scratch;
        buf.clear();
        write(msg, &mut buf)?;
        Ok(self.socket.send_to(buf.as_slice(), addr).map_err(anyhow::Error::from)?)
    }
    pub fn send(&mut self, msg: &Message) -> anyhow::Result<usize> {
        let mut buf = &mut self.send_buf;
        let before = buf.len();
        match write(msg, &mut buf) {
            Ok(_) => {}
            Err(e) => {
                buf.truncate(before);
                return Err(e);
            }
        }
        let amount = buf.len() - before;
        if buf.len() >= MAX_DATAGRAM_SIZE {
            self.flush()?;
        }
        Ok(amount)
    }

    pub fn receive(&self, buf: &mut [u8]) -> anyhow::Result<Option<(usize, SocketAddr)>> {
        match self.socket.recv_from(buf) {
            Ok((amount, addr)) => {
                if amount > 0 {
                    Ok(Some((amount, addr)))
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

pub(crate) fn process_messages<F>(mut buf: &[u8], mut handler: F) -> anyhow::Result<()>
    where F: FnMut(&Message) -> anyhow::Result<()>
{
    let mut decoder = <Message<'_> as bitcode::Decode>::Decoder::default();
    let s = &mut buf;
    while !s.is_empty() {
        decoder.populate(s, 1).unwrap();
        handler(&decoder.decode())?;
    }
    Ok(())
}
