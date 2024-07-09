use io::ErrorKind::WouldBlock;
use std::borrow::Cow;
use std::io;
use std::io::ErrorKind::UnexpectedEof;
use std::io::Read;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use log::{error, info, warn};
use rmp_serde::decode::Error::InvalidMarkerRead;
use rmp_serde::Deserializer;
use rsa::pkcs8::DecodePublicKey;
use rsa::RsaPublicKey;
use serde::Deserialize;

use core::arguments::Arguments;

use crate::client::pub_key::PublicKey;
use crate::net::{ConnectData, Endpoint, MAX_DATAGRAM_SIZE, Message, process_messages, TimeData};
use crate::net::Message::{Accepted, Hello, Ping, Pong, ServerInfo};

enum ClientState {
    DISCONNECTED,
    CONNECTING,
    CONNECTED,
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
            ServerInfo(data) => {
                self.server_key = Some(PublicKey::new(data.key.clone()));
                info!("Got server's public key!");
                let m = build_connect_message(self.server_key.as_ref())?;
                self.send(&m);
            }
            Pong(td) => {
                info!("Ping to server is {:.6} sec.", Instant::now().elapsed().as_secs_f64() - td.time);
            }
            Ping(td) => {
                self.send(&Pong(TimeData { time: td.time }));
            }
            m => {
                warn!("Unsupported message from server: {m:?}");
            }
        }
        Ok(())
    }

    fn process_messages(&mut self, buf: &[u8]) -> anyhow::Result<()> {
        let mut des = Deserializer::from_read_ref(buf);
        loop {
            match Message::deserialize(&mut des) {
                Ok(msg) => {
                    self.process_message(&msg)?;
                }
                Err(InvalidMarkerRead(io_err)) => {
                    if io_err.kind() == UnexpectedEof {
                        break;
                    } else {
                        return Err(anyhow::Error::from(io_err));
                    }
                }
                Err(e) => {
                    return Err(anyhow::Error::from(e));
                }
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
                process_messages(&buf.as_slice(), |m| self.process_message(m)).expect("AAAAAAAAAAaa!");
            }
            Ok(None) => {}
            Err(ref e) => {
                error!("Failed to receive from server: {e:?}");
            }
        }
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
                    let to_send = if !self.server_key.is_some() {
                        Hello
                    } else {
                        build_connect_message(self.server_key.as_ref()).expect("Failed to create connect message!")
                    };
                    self.send(&to_send);
                }
            }
            ClientState::CONNECTED => {
                if elapsed >= Duration::from_secs(2) {
                    self.send(&Ping(TimeData { time: Instant::now().elapsed().as_secs_f64() }));
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

fn build_connect_message<'a>(key: Option<&PublicKey>) -> anyhow::Result<Message<'a>> {
    let encoded = key.ok_or_else(|| anyhow::Error::msg("Server key is not present!"))
        .and_then(|k| k.encode_str("123456"))?;
    Ok(Message::Connect(ConnectData {
        name: Cow::from("Test"),
        password: Cow::from(encoded),
    }))
}
