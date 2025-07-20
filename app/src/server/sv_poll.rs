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

pub(crate) struct ServerPollThread {
    pub(crate) local_addr: SocketAddr,
    pub(crate) rx: Receiver<Packet>,
    pub(crate) tx: Sender<Packet>,
    poll_thread_handle: Option<JoinHandle<()>>,
    send_thread_handle: Option<JoinHandle<()>>,
}

impl ServerPollThread {
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
        let send_thread_handle = thread::spawn(move || {
            let mut unsent = VecDeque::<Packet>::new();
            loop {
                match in_rx.recv_timeout(timeout.unwrap()) {
                    Ok(p) => {
                        match send_socket.send_to(p.bytes.as_slice(), p.address) {
                            Ok(amount) => {
                                if amount < p.bytes.len() {
                                    warn!("Partial send: {amount} of {}", p.bytes.len());
                                }
                            }
                            Err(e) => {
                                if e.kind() == ErrorKind::WouldBlock {
                                    unsent.push_back(p);
                                } else {
                                    error!("Failed to send packet: {e:?}");
                                }
                            }
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
            poll_thread_handle: Some(poll_thread_handle),
            send_thread_handle: Some(send_thread_handle)
        })
    }

    pub(crate) fn local_addr(&self) -> Result<SocketAddr, AppError> {
        Ok(self.local_addr)
    }
}

impl Drop for ServerPollThread {
    fn drop(&mut self) {
        self.poll_thread_handle.take().map(JoinHandle::join);
        self.send_thread_handle.take().map(JoinHandle::join);
    }
}
