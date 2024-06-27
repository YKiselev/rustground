use std::io;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

use log::{error, info};

use core::arguments::Arguments;

use crate::net::{ConnectData, Message};

pub(crate) struct Client {
    socket: UdpSocket,
    buffer: [u8; 512],
    server_addr: SocketAddr,
    connected: bool,
}

impl Client {
    pub(crate) fn update(&mut self) {
        match self.socket.recv_from(&mut self.buffer) {
            Ok((amount, addr)) => {
                info!("Handling server message from {addr:?}");
                let msg: Message = rmp_serde::from_slice(&self.buffer[..amount])
                    .expect("Unable to deserialize server message!");
                match msg {
                    Message::Accepted => {
                        self.connected = true;
                        info!("Connected to server!");
                    }
                    m => {
                        info!("Unsupported message from server: {m:?}");
                    }
                }
                //self.socket.send_to(&self.buffer, &addr).expect("Unable to send data back!");
            }
            Err(ref e) => if e.kind() == io::ErrorKind::WouldBlock {
                // no-op
            } else {
                info!("Failed to receive from server: {e:?}");
            }
        }
        if !self.connected {
            let to_send = rmp_serde::to_vec(&Message::Connect(ConnectData {
                name: String::from("Test"),
                password: String::from("123456"),
            })).expect("Unable to serialize!");
            match self.socket.send_to(&to_send, &self.server_addr) {
                Ok(n) => {
                    info!("Sent {n} bytes to server!");
                }
                Err(ref e) => {
                    error!("Failed to send data to the server: {e:?}");
                }
            }
        }
        match self.socket.take_error() {
            Ok(Some(error)) => error!("UdpSocket error: {error:?}"),
            Ok(None) => {}
            Err(error) => error!("UdpSocket.take_error failed: {error:?}"),
        }
    }

    pub(crate) fn new(args: &Arguments, server_addr: SocketAddr) -> Self {
        info!("Starting client...");
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).expect("Unable to bind client");
        socket.set_nonblocking(true).expect("Unable to set non-blocking mode!");
        Client {
            socket,
            buffer: [0; 512],
            server_addr,
            connected: false,
        }
    }
}