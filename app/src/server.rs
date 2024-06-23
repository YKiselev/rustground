use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::Instant;

use log::{error, info, warn};

use core::arguments::Arguments;

use crate::net::Message;

#[derive(Debug, Eq, PartialEq, Hash)]
struct ClientId(SocketAddr);

#[derive(Debug)]
struct Client {
    name: String,
    last_seen: Instant,
    //send_buf: Vec<Message>,
}

impl Client {
    fn send(&self, msg: Message) {}
}

pub(crate) struct Server {
    socket: UdpSocket,
    buffer: [u8; 512],
    clients: HashMap<ClientId, Client>,
}

impl Server {
    pub(crate) fn update(&mut self) {
        self.listen()
    }

    pub fn local_address(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    pub fn new(args: &Arguments) -> Self {
        info!("Starting server...");
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).expect("Unable to bind server socket!");
        socket.set_nonblocking(true).expect("Unable to set non-blocking mode!");
        Server {
            socket,
            buffer: [0; 512],
            clients: HashMap::new(),
        }
    }

    fn process_message(&mut self, amount: usize, addr: &SocketAddr) {
        let buf = &self.buffer[..amount];
        let msg: Message = serde_json::from_slice(buf).expect("Unable to deserialize message!");
        let key = ClientId(*addr);
        match msg {
            Message::Connect { name, .. } => {
                // todo - check password
                match self.clients.entry(key) {
                    Entry::Vacant(v) => {
                        v.insert(Client {
                            name,
                            last_seen: Instant::now(),
                        });
                        let to_send = serde_json::to_vec(&Message::Accepted).expect("Unable to serialize!");
                        self.socket.send_to(&to_send, &addr).expect("Unable to send data back!");
                    }
                    Entry::Occupied(ref mut o) => {
                        o.get_mut().last_seen = Instant::now();
                    }
                }
            }
            _ => {
                if let Entry::Occupied(ref mut o) = self.clients.entry(key) {
                    o.get_mut().last_seen = Instant::now();
                }
            }
        }
    }

    pub fn listen(&mut self) {
        for i in 0..10 {
            match self.socket.recv_from(&mut self.buffer) {
                Ok((amount, addr)) => {
                    info!("Handling client from {}", &addr);
                    self.process_message(amount, &addr);
                }
                Err(ref e) => if e.kind() == io::ErrorKind::WouldBlock {
                    // no-op
                } else {
                    info!("Got {e:?}");
                }
            }
            match self.socket.take_error() {
                Ok(Some(error)) => error!("UdpSocket error: {error:?}"),
                Ok(None) => {}
                Err(error) => error!("UdpSocket.take_error failed: {error:?}"),
            }
        }
    }
}
