use std::{
    collections::VecDeque,
    io::ErrorKind,
    net::SocketAddr,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use log::{error, warn};
use mio::{net::UdpSocket, Events, Interest, Poll, Token};
use rg_net::{protocol::NET_BUF_SIZE, MAX_DATAGRAM_SIZE};

use crate::error::AppError;

const MAX_PENDING_PACKETS: usize = 64;

#[derive(Debug)]
pub(crate) struct Packet {
    pub address: SocketAddr,
    pub bytes: Vec<u8>,
}

pub(crate) struct ServerPoll {
    pub(crate) local_addr: SocketAddr,
    rx: Receiver<Packet>,
    tx: Sender<Packet>,
    poll_thread: JoinHandle<()>,
    send_thread: JoinHandle<()>,
}

impl ServerPoll {
    const SERVER: Token = Token(1);

    pub(crate) fn new(addr: SocketAddr) -> Result<Self, AppError> {
        let (out_tx, out_rx) = mpsc::channel();
        let (in_tx, in_rx) = mpsc::channel::<Packet>();
        let mut socket = UdpSocket::bind(addr)?;
        let mut poll = Poll::new()?;
        let mut events = Events::with_capacity(256);
        poll.registry()
            .register(&mut socket, Self::SERVER, Interest::READABLE)?;
        let local_addr = socket.local_addr()?;
        let timeout = Some(Duration::from_millis(200));
        let socket = Arc::new(socket);
        let send_socket = Arc::clone(&socket);
        let new_buf = || Vec::with_capacity(MAX_DATAGRAM_SIZE);
        let poll_thread_handle = thread::spawn(move || {
            let mut buf = new_buf();
            let mut exit_flag = false;
            while !exit_flag {
                match poll.poll(&mut events, timeout) {
                    Ok(_) => {
                        for e in events.iter() {
                            if e.token() == Self::SERVER {
                                if e.is_readable() {
                                    loop {
                                        buf.resize(NET_BUF_SIZE, 0);
                                        match socket.recv_from(buf.as_mut_slice()) {
                                            Ok((available, remote_addr)) => {
                                                if available > 0 {
                                                    buf.truncate(available);
                                                    if let Err(_) = out_tx.send(Packet {
                                                        address: remote_addr,
                                                        bytes: std::mem::replace(
                                                            &mut buf,
                                                            new_buf(),
                                                        ),
                                                    }) {
                                                        error!("Send channel is closed!");
                                                        exit_flag = true;
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                if e.kind() == ErrorKind::WouldBlock {
                                                    break;
                                                } else {
                                                    warn!("Failed to receive: {e:?}");
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
        let send_thread_handle = thread::spawn(move || {
            let mut unsent = VecDeque::<Packet>::new();
            loop {
                if unsent.len() > 10 {
                    warn!("Too many unsent packets!");
                }
                while let Some(packet) = unsent.pop_front() {
                    if let Some(p) = try_send(send_socket.as_ref(), packet) {
                        unsent.push_front(p);
                    }
                }
                match in_rx.recv_timeout(timeout.unwrap()) {
                    Ok(packet) => {
                        if let Some(p) = try_send(send_socket.as_ref(), packet) {
                            unsent.push_back(p);
                        }
                    }
                    Err(e) => match e {
                        mpsc::RecvTimeoutError::Timeout => {}
                        mpsc::RecvTimeoutError::Disconnected => break,
                    },
                }
            }
        });

        Ok(Self {
            local_addr,
            rx: out_rx,
            tx: in_tx,
            poll_thread: poll_thread_handle,
            send_thread: send_thread_handle,
        })
    }

    pub(crate) fn local_addr(&self) -> Result<SocketAddr, AppError> {
        Ok(self.local_addr)
    }

    pub(crate) fn rx(&self) -> &Receiver<Packet> {
        &self.rx
    }

    pub(crate) fn tx(&self) -> &Sender<Packet> {
        &self.tx
    }

    pub(crate) fn shutdown(mut self) {
        std::mem::drop(self.rx);
        std::mem::drop(self.tx);
        let _ = self
            .poll_thread
            .join()
            .inspect_err(|e| warn!("Poll thread join failed: {:?}", e));
        let _ = self
            .send_thread
            .join()
            .inspect_err(|e| warn!("Send thread join failed: {:?}", e));
    }
}

fn try_send(socket: &UdpSocket, packet: Packet) -> Option<Packet> {
    match socket.send_to(packet.bytes.as_slice(), packet.address) {
        Ok(amount) => {
            if amount < packet.bytes.len() {
                warn!("Partial send: {amount} of {}", packet.bytes.len());
            }
        }
        Err(e) => {
            if e.kind() == ErrorKind::WouldBlock {
                return Some(packet);
            } else {
                error!("Failed to send packet: {e:?}");
            }
        }
    }
    None
}
