use io::ErrorKind::WouldBlock;
use std::borrow::Cow;
use std::error::Error;
use std::fmt::format;
use std::io;
use std::io::ErrorKind::UnexpectedEof;
use std::io::Read;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use bitcode::__private::View;
use log::{error, info, warn};
use rsa::pkcs8::DecodePublicKey;
use rsa::RsaPublicKey;

use rg_common::arguments::Arguments;

use crate::app::App;
use crate::client::cl_pub_key::PublicKey;
use crate::net::{Endpoint, MAX_DATAGRAM_SIZE, Message, NetEndpoint};
use crate::net::Message::{Accepted, Hello, Ping, Pong, ServerInfo};

#[derive(Eq, PartialEq)]
enum ClientState {
    INIT,
    DISCONNECTED,
    CONNECTING,
    CONNECTED,
}

pub(crate) struct Client {
    endpoint: Box<dyn Endpoint>,
    recv_buf: Option<Vec<u8>>,
    server_addr: Option<SocketAddr>,
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
                info!("Ping to server is {:.2} ms.", 1000.0 * (Instant::now().elapsed().as_secs_f64() - time));
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
        let mut buf = self.recv_buf.take().unwrap_or_else(|| Vec::new());
        loop {
            match self.endpoint.receive_data(buf.as_mut()) {
                Ok(Some(mut data)) => {
                    while let Some(ref m) = data.read() {
                        self.process_message(m).unwrap();
                    }
                }

                Ok(None) => {
                    break;
                }
                Err(e) => {
                    error!("Failed to receive from server: {e:?}");
                    break;
                }
            }
        }
        self.recv_buf.replace(buf);
    }

    fn send_connect_message(&mut self) {
        let key = self.server_key.as_ref().unwrap();
        let encoded = key.encode_str("123456").unwrap();
        self.send(&Message::Connect {
            name: "Test",
            password: encoded,
        })
    }

    fn is_time_to_resend(&self) -> bool {
        Self::CONN_RETRY_INTERVAL <= self.last_send.map_or_else(|| Self::CONN_RETRY_INTERVAL, |v| v.elapsed())
    }

    pub(crate) fn frame_start(&mut self) {
        self.endpoint.clear_buffers();
        match self.endpoint.take_error() {
            Ok(Some(error)) => error!("Socket error: {error:?}"),
            Ok(None) => {}
            Err(error) => error!("Unable to take error: {error:?}"),
        }
    }

    pub(crate) fn update(&mut self, app: &mut App) {
        self.receive_from_server();
        if self.is_time_to_resend() {
            match self.state {
                ClientState::INIT => {
                    if let Ok(addr) = app.get_var("server_address") {
                        match self.endpoint.connect(addr) {
                            Ok(_) => {
                                info!("Client socket connected to {}", addr);
                                self.state = ClientState::DISCONNECTED;
                            }
                            Err(e) => {
                                error!("Unable to connect socket: {}", e);
                            }
                        }
                    }
                }
                ClientState::DISCONNECTED => {
                    self.send(&Hello);
                    self.state = ClientState::CONNECTING;
                }
                ClientState::CONNECTING => {
                    if !self.server_key.is_some() {
                        self.send(&Hello);
                    } else {
                        self.send_connect_message();
                    };
                }
                ClientState::CONNECTED => {
                    for i in 0..10 {
                        self.send(&Ping { time: Instant::now().elapsed().as_secs_f64() });
                    }
                }
            }
        }
    }

    pub(crate) fn frame_end(&mut self) {
        if let Err(e) = self.endpoint.flush() {
            if self.state == ClientState::INIT {
                error!("Flush failed: {}", e);
            }
        }
    }

    pub(crate) fn new(app: &mut App) -> Self {
        info!("Starting client...");
        let endpoint = NetEndpoint::new().expect("Unable to create client socket!");
        //endpoint.connect(&server_addr).expect("Unable to set server address on client socket!");
        Client {
            endpoint: Box::new(endpoint),
            recv_buf: Some(Vec::with_capacity(MAX_DATAGRAM_SIZE)),
            server_addr: None,
            server_key: None,
            state: ClientState::INIT,
            last_seen: None,
            last_send: None,
        }
    }
}
