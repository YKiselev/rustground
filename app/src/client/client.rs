use io::ErrorKind::WouldBlock;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

use log::{error, info};
use rsa::pkcs8::DecodePublicKey;
use rsa::RsaPublicKey;

use core::arguments::Arguments;

use crate::net::{ConnectData, Endpoint, Message};

pub(crate) struct Client {
    endpoint: Endpoint,
    buffer: [u8; 512],
    server_addr: SocketAddr,
    connected: bool,
    server_key: Option<RsaPublicKey>,
}

impl Client {
    fn send(&mut self, msg: &Message) {
        match self.endpoint.send(msg) {
            Ok(n) => {
                info!("Sent {n} bytes to server!");
            }
            Err(ref e) => {
                error!("Failed to send data to the server: {e:?}");
            }
        }
    }

    pub(crate) fn update(&mut self) {
        self.endpoint.clear_buffers();
        match self.endpoint.receive(&mut self.buffer) {
            Ok(Some((amount, addr))) => {
                info!("Handling server message ({amount} bytes) from {addr:?}");
                let msg: Message = rmp_serde::from_slice(&self.buffer[..amount])
                    .expect("Unable to deserialize server message!");
                match msg {
                    Message::Accepted => {
                        self.connected = true;
                        info!("Connected to server!");
                    }
                    Message::ServerInfo(data) => {
                        self.server_key = Some(RsaPublicKey::from_public_key_pem(&data.key).expect("Unable to import server's key!"));
                        info!("Got server's public key: {}", data.key);
                    }
                    m => {
                        info!("Unsupported message from server: {m:?}");
                    }
                }
            }
            Ok(None) => {}
            Err(ref e) => {
                info!("Failed to receive from server: {e:?}");
            }
        }
        if !self.server_key.is_some() {
            self.send(&Message::Hello);
        } else if !self.connected {
            let to_send = Message::Connect(ConnectData {
                name: String::from("Test"),
                password: String::from("123456"),
            });
            self.send(&to_send);
        } else {
            self.send(&Message::KeepAlive);
        }
        match self.endpoint.take_error() {
            Ok(Some(error)) => error!("UdpSocket error: {error:?}"),
            Ok(None) => {}
            Err(error) => error!("UdpSocket.take_error failed: {error:?}"),
        }
        self.endpoint.flush().expect("Flush failed!");
    }

    pub(crate) fn new(args: &Arguments, server_addr: SocketAddr) -> Self {
        info!("Starting client...");
        let endpoint = Endpoint::new().expect("Unable to create client socket!");
        endpoint.connect(server_addr).expect("Unable to set server address on client socket!");
        Client {
            endpoint,
            buffer: [0; 512],
            server_addr,
            connected: false,
            server_key: None,
        }
    }
}