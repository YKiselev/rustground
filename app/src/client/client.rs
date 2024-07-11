use io::ErrorKind::WouldBlock;
use std::borrow::Cow;
use std::io;
use std::io::ErrorKind::UnexpectedEof;
use std::io::Read;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use log::{error, info, warn};
use rsa::pkcs8::DecodePublicKey;
use rsa::RsaPublicKey;

use common::arguments::Arguments;

use crate::client::pub_key::PublicKey;
use crate::net::{Endpoint, MAX_DATAGRAM_SIZE, Message, process_messages};
use crate::net::Message::{Accepted, Hello, Ping, Pong, ServerInfo};

trait ClientState {
    // DISCONNECTED,
    // CONNECTING,
    // CONNECTED,

}

pub(crate) struct Client {
    endpoint: Endpoint,
    server_addr: SocketAddr,
    server_key: Option<PublicKey>,
    state: ClientState,
    last_seen: Option<Instant>,
    last_send: Option<Instant>,
}

impl Client {
    const MAX_LAST_SEEN: Duration = Duration::from_secs(3);
    const CONN_RETRY_INTERVAL: Duration = Duration::from_secs(3);

    fn send(&mut self, msg: &Message) {
        match self.endpoint.send(msg) {
            Ok(n) => {
                self.last_send = Some(Instant::now());
                info!("Sent {n} bytes to server!");
            }
            Err(ref e) => {
                error!("Failed to send data to the server: {e:?}");
            }
        }
    }

    fn process_message(&mut self, msg: &Message) -> anyhow::Result<()> {
        match msg {
            Accepted => {
                self.state = ClientState::CONNECTED;
                info!("Connected to server!");
            }
            ServerInfo { key } => {
                let key = bitcode::deserialize::<RsaPublicKey>(key)?;
                self.server_key = Some(PublicKey::new(key));
                info!("Got server's public key!");
                self.send_connect_message();
            }
            Pong { time } => {
                info!("Ping to server is {:.6} sec.", Instant::now().elapsed().as_secs_f64() - time);
            }
            Ping { time } => {
                self.send(&Pong { time: *time });
            }
            m => {
                warn!("Unsupported message from server: {m:?}");
            }
        }
        Ok(())
    }

    fn receive_from_server(&mut self) {
        let mut buf: Vec<u8> = Vec::with_capacity(MAX_DATAGRAM_SIZE);
        buf.resize(MAX_DATAGRAM_SIZE, 0);
        match self.endpoint.receive(&mut buf) {
            Ok(Some((amount, addr))) => {
                if self.server_addr != addr {
                    info!("Ignored message from {addr:?}");
                    return;
                }
                info!("Handling server message ({amount} bytes) from {addr:?}");
                self.last_seen = Some(Instant::now());
                buf.truncate(amount);
                process_messages(buf.as_slice(), |m| self.process_message(m)).unwrap();
            }
            Ok(None) => {}
            Err(ref e) => {
                error!("Failed to receive from server: {e:?}");
            }
        }
    }

    fn send_connect_message(&mut self) {
        let key = self.server_key.as_ref().unwrap();
        let encoded = key.encode_str("123456").unwrap();
        self.send(&Message::Connect {
            name: "Test",
            password: encoded,
        })
    }

    pub(crate) fn frame_start(&mut self) {
        self.endpoint.clear_buffers();
        match self.endpoint.take_error() {
            Ok(Some(error)) => error!("Socket error: {error:?}"),
            Ok(None) => {}
            Err(error) => error!("Unable to take error: {error:?}"),
        }
    }

    pub(crate) fn update(&mut self) {
        self.receive_from_server();
        let elapsed = self.last_send.map_or_else(|| Self::CONN_RETRY_INTERVAL, |v| v.elapsed());
        match self.state {
            ClientState::DISCONNECTED => {
                self.send(&Hello);
                self.state = ClientState::CONNECTING;
            }
            ClientState::CONNECTING => {
                if elapsed >= Self::CONN_RETRY_INTERVAL {
                    if !self.server_key.is_some() {
                        self.send(&Hello);
                    } else {
                        self.send_connect_message();
                    };
                }
            }
            ClientState::CONNECTED => {
                if elapsed >= Duration::from_secs(2) {
                    for i in 0..10 {
                        self.send(&Ping { time: Instant::now().elapsed().as_secs_f64() });
                    }
                }
            }
        }
    }

    pub(crate) fn frame_end(&mut self) {
        self.endpoint.flush().expect("Flush failed!");
    }

    pub(crate) fn new(args: &Arguments, server_addr: SocketAddr) -> Self {
        info!("Starting client...");
        let endpoint = Endpoint::new().expect("Unable to create client socket!");
        endpoint.connect(&server_addr).expect("Unable to set server address on client socket!");
        Client {
            endpoint,
            server_addr,
            server_key: None,
            state: ClientState::DISCONNECTED,
            last_seen: None,
            last_send: None,
        }
    }
}
