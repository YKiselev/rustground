use std::cmp::min;
use std::fmt::{Debug, Formatter};
use std::io;
use std::io::ErrorKind::WouldBlock;
use std::io::{Error, Write};
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::num::NonZeroUsize;

use bitcode::__private::{Buffer, Decoder, Encoder, View};
use bitcode::{Decode, Encode};

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

pub(crate) trait Endpoint: Debug {
    fn connect(&self, addr: SocketAddr) -> io::Result<()>;
    fn local_addr(&self) -> io::Result<SocketAddr>;
    fn peer_addr(&self) -> io::Result<SocketAddr>;
    fn clear_buffers(&mut self);
    fn take_error(&self) -> io::Result<Option<Error>>;
    fn flush(&mut self) -> io::Result<usize>;
    fn send_to(&mut self, msg: &Message, addr: &SocketAddr) -> io::Result<usize>;
    fn send(&mut self, msg: &Message) -> io::Result<usize>;
    fn receive_data<'a>(&mut self, buf: &'a mut Vec<u8>) -> io::Result<Option<ReceivedData<'a>>>;
}

pub(crate) trait ServerEndpoint: Endpoint {
    fn try_clone_and_connect(
        &self,
        addr: &SocketAddr,
    ) -> io::Result<Box<dyn Endpoint + Sync + Send>>;
}

pub struct NetEndpoint {
    socket: UdpSocket,
    send_buf: Vec<u8>,
    scratch: Vec<u8>,
    encoder: <Message<'static> as bitcode::Encode>::Encoder,
    decoder: <Message<'static> as bitcode::Decode<'static>>::Decoder,
}

impl Debug for NetEndpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Endpoint")
            .field("socket", &self.socket)
            .field("send_buf", &self.send_buf)
            .field("scratch", &self.scratch)
            .finish_non_exhaustive()
    }
}

impl NetEndpoint {
    fn from_socket(socket: UdpSocket) -> Self {
        NetEndpoint {
            socket,
            send_buf: Vec::with_capacity(MAX_DATAGRAM_SIZE),
            scratch: Vec::with_capacity(MAX_DATAGRAM_SIZE),
            encoder: <Message<'_> as bitcode::Encode>::Encoder::default(),
            decoder: <Message<'_> as bitcode::Decode>::Decoder::default(),
        }
    }

    pub fn with_address<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;
        Ok(Self::from_socket(socket))
    }
    pub fn new() -> io::Result<Self> {
        Self::with_address((Ipv4Addr::UNSPECIFIED, 0))
    }

    fn encode_to_scratch(&mut self, msg: &Message) -> usize {
        self.encoder.reserve(NonZeroUsize::new(1).unwrap());
        encode_inline_never(&mut self.encoder, msg);
        self.scratch.clear();
        self.encoder.collect_into(&mut self.scratch);
        self.scratch.len()
    }

    fn flush_exact(&mut self, amount: usize) -> io::Result<usize> {
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
                        return Err(e);
                    }
                }
            }
        }
        Ok(amount - left)
    }
}

impl Endpoint for NetEndpoint {
    fn connect(&self, addr: SocketAddr) -> io::Result<()> {
        self.socket.connect(addr)
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.socket.peer_addr()
    }

    fn clear_buffers(&mut self) {
        self.send_buf.clear();
    }

    fn take_error(&self) -> io::Result<Option<Error>> {
        self.socket.take_error()
    }

    fn flush(&mut self) -> io::Result<usize> {
        let buf = &self.send_buf;
        assert!(buf.len() <= MAX_DATAGRAM_SIZE);
        self.flush_exact(min(buf.len(), MAX_DATAGRAM_SIZE))
    }

    fn send_to(&mut self, msg: &Message, addr: &SocketAddr) -> io::Result<usize> {
        self.encode_to_scratch(msg);
        self.socket.send_to(&self.scratch, addr)
    }

    fn send(&mut self, msg: &Message) -> io::Result<usize> {
        self.encode_to_scratch(msg);
        if self.send_buf.len() + self.scratch.len() >= MAX_DATAGRAM_SIZE {
            self.flush()?;
        }
        self.send_buf.write(&self.scratch)
    }

    fn receive_data<'a>(&mut self, buf: &'a mut Vec<u8>) -> io::Result<Option<ReceivedData<'a>>> {
        buf.resize(MAX_DATAGRAM_SIZE, 0);
        match self.socket.recv_from(buf.as_mut_slice()) {
            Ok((amount, addr)) => {
                if amount > 0 {
                    buf.truncate(amount);
                    Ok(Some(ReceivedData::new(buf.as_slice(), addr)))
                } else {
                    Ok(None)
                }
            }
            Err(e) => {
                return if e.kind() == WouldBlock {
                    Ok(None) // no data yet
                } else {
                    Err(e)
                };
            }
        }
    }
}

impl ServerEndpoint for NetEndpoint {
    fn try_clone_and_connect(
        &self,
        addr: &SocketAddr,
    ) -> io::Result<Box<dyn Endpoint + Sync + Send>> {
        let socket = self.socket.try_clone()?;
        self.socket.connect(addr)?;
        Ok(Box::new(Self::from_socket(socket)))
    }
}

pub(crate) struct ReceivedData<'a> {
    pub addr: SocketAddr,
    slice: &'a [u8],
    decoder: Option<<Message<'a> as bitcode::Decode<'a>>::Decoder>,
}

impl<'a> ReceivedData<'a> {
    pub fn new(slice: &'a [u8], addr: SocketAddr) -> Self {
        ReceivedData {
            addr,
            slice,
            decoder: Some(<Message<'_> as bitcode::Decode>::Decoder::default()),
        }
    }

    pub fn read(&mut self) -> Option<Message> {
        if self.slice.is_empty() {
            return None;
        }
        let mut slice = &mut std::mem::take(&mut self.slice);
        let mut decoder = <Message<'_> as bitcode::Decode>::Decoder::default();
        decoder.populate(&mut slice, 1).unwrap();
        let msg: Message = decode_inline_never(&mut decoder);
        self.slice = slice;
        return Some(msg);
    }
}

#[inline(never)]
fn encode_inline_never<T: Encode + ?Sized>(encoder: &mut T::Encoder, t: &T) {
    encoder.encode(t);
}

#[inline(never)]
pub(crate) fn decode_inline_never<'a, T: Decode<'a>>(decoder: &mut T::Decoder) -> T {
    decoder.decode()
}
