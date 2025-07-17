use std::{
    collections::{hash_map::Entry, HashMap, VecDeque},
    io::ErrorKind,
    iter::Map,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread::{self, scope, JoinHandle},
    time::Duration,
};

use log::{error, info, warn};
use mio::{event::Event, net::UdpSocket, Events, Interest, Poll, Token};
use rg_net::protocol::NET_BUF_SIZE;

use crate::app::ExitFlag;

use super::sv_error::ServerError;

const MAX_PENDING_PACKETS: usize = 64;

#[derive(Debug)]
pub(crate) struct Packet {
    pub address: SocketAddr,
    pub bytes: Vec<u8>,
}

pub(crate) struct ServerPollThread {
    pub(crate) socket: Arc<UdpSocket>,
    pub(crate) rx: Receiver<Packet>,
    pub(crate) send_buf: VecDeque<Packet>,
    handle: Option<JoinHandle<()>>,
}

impl ServerPollThread {
    const SERVER: Token = Token(1);

    pub(crate) fn new(addr: SocketAddr, exit_flag: ExitFlag) -> Result<Self, ServerError> {
        let (out_tx, out_rx) = mpsc::channel();
        let mut socket = UdpSocket::bind(addr)?;
        let mut poll = Poll::new()?;
        let mut events = Events::with_capacity(256);
        poll.registry()
            .register(&mut socket, Self::SERVER, Interest::READABLE)?;
        let timeout = Some(Duration::from_millis(200));
        let socket = Arc::new(socket);
        let poll_socket = Arc::clone(&socket);
        let handle = thread::spawn(move || {
            let mut buf = Vec::new();
            while !exit_flag.load() {
                match poll.poll(&mut events, timeout) {
                    Ok(_) => {
                        for e in events.iter() {
                            if e.token() == Self::SERVER {
                                if e.is_readable() {
                                    loop {
                                        buf.resize(NET_BUF_SIZE, 0);
                                        match poll_socket.recv_from(buf.as_mut_slice()) {
                                            Ok((available, remote_addr)) => {
                                                if available > 0 {
                                                    buf.truncate(available);
                                                    if let Err(e) = out_tx.send(Packet {
                                                        address: remote_addr,
                                                        bytes: buf.clone(), // todo - make buf recyclable
                                                    }) {
                                                        error!("Unable to send: {:?}", e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                if e.kind() == ErrorKind::WouldBlock {
                                                    break;
                                                } else {
                                                    warn!("Failed to read: {e:?}");
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Polling failed: {e:?}");
                    }
                }
            }
        });
        Ok(Self {
            socket,
            rx: out_rx,
            send_buf: VecDeque::new(),
            handle: Some(handle),
        })
    }

    pub(crate) fn send(&mut self, packet: Packet) -> Option<Packet> {
        match self.socket.send_to(&packet.bytes, packet.address) {
            Ok(written) => {
                if written < packet.bytes.len() {
                    self.send_buf.push_back(packet);
                    None
                } else {
                    Some(packet)
                }
            }
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    self.send_buf.push_back(packet);
                    None
                } else {
                    error!("Send failed: {e:?}");
                    Some(packet)
                }
            }
        }
    }

    fn send_to(&mut self, buf: &[u8], address: SocketAddr) -> bool {
        match self.socket.send_to(buf, address) {
            Ok(written) => written == buf.len(),
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    // no-op
                } else {
                    error!("Send failed: {e:?}");
                }
                false
            }
        }
    }

    pub(crate) fn update<F>(&mut self, mut processor: F)
    where
        F: FnMut(&Packet),
    {
        for p in self.rx.try_iter() {
            processor(&p);
        }

        // Try to send all pending packets
        while let Some(p) = self.send_buf.pop_front() {
            if !self.send_to(&p.bytes, p.address) {
                self.send_buf.push_front(p);
                break;
            }
        }
        // Take errors
        match self.socket.take_error() {
            Ok(op) => {
                if let Some(e) = op {
                    warn!("Got error: {e:?}");
                }
            }
            Err(e) => {
                error!("Failed to take socket error: {e:?}");
            }
        }
    }

    pub(crate) fn local_addr(&self) -> Result<SocketAddr, ServerError> {
        self.socket.local_addr().map_err(|e| ServerError::from(e))
    }
}

impl Drop for ServerPollThread {
    fn drop(&mut self) {
        self.handle.take().map(JoinHandle::join);
    }
}
