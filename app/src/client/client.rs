use std::collections::VecDeque;
use std::io::{ErrorKind, Read};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::usize::MAX;

use log::{error, info, warn};
use mio::net::UdpSocket;
use rg_net::header::{read_header, write_with_header};
use rg_net::hello::write_hello;
use rg_net::net_rw::{try_write, NetBufReader, NetBufWriter, NetReader, NetWriter, WithPosition};
use rg_net::protocol::{Header, PacketKind, ProtocolError, MAX_DATAGRAM_SIZE, MIN_HEADER_SIZE, NET_BUF_SIZE};
use rg_net::server_info::read_server_info;
use rsa::RsaPublicKey;

use crate::app::App;
use crate::client::cl_pub_key::PublicKey;
use crate::error::AppError;

use super::cl_net::receive_data;

#[derive(Eq, PartialEq)]
enum ClientState {
    Disconnected,
    Connected,
    AwaitingAcceptance,
    Accepted,
}

pub(crate) struct Client {
    socket: UdpSocket,
    send_bufs: VecDeque<Vec<u8>>,
    server_addr: Option<SocketAddr>,
    server_key: Option<PublicKey>,
    state: ClientState,
    last_seen: Option<Instant>,
    last_send: Option<Instant>,
}

impl Client {
    const MAX_LAST_SEEN: Duration = Duration::from_secs(3);
    const CONN_RETRY_INTERVAL: Duration = Duration::from_secs(3);

    pub(crate) fn new(app: &Arc<App>) -> Self {
        info!("Starting client...");
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0).into())
            .expect("Unable to create client socket!");
        Client {
            socket,
            send_bufs: VecDeque::new(),
            server_addr: None,
            server_key: None,
            state: ClientState::Disconnected,
            last_seen: None,
            last_send: None,
        }
    }

    fn try_write<H>(&mut self, mut handler: H) -> Result<(), ProtocolError>
    where
        H: FnMut(&mut NetBufWriter) -> Result<(), ProtocolError>,
    {
        for _ in 0..2 {
            if let Some(buf) = self.send_bufs.back_mut() {
                match try_write(buf, &mut handler) {
                    Ok(flag) => {
                        if flag {
                            break;
                        }
                    }
                    Err(e) => error!("Failed to write send buffer: {}", e),
                }
            }
            self.send_bufs.push_back(Vec::new());
        }
        Ok(())
    }

    fn send_hello(&mut self) -> Result<(), ProtocolError> {
        self.try_write(|w| write_with_header(w, PacketKind::Hello, |w| write_hello(w)))
    }

    fn on_server_info<'a, R>(&mut self, header: &Header, reader: &mut R) -> Result<(), AppError>
    where
        R: NetReader<'a>,
    {
        let info = read_server_info(reader).map_err(|e| AppError::GenericError {
            message: e.to_string(),
        })?;
        self.server_key = PublicKey::from_der(info.key).ok();
        Ok(())
    }

    // fn send(&mut self, msg: &Message) {
    //     match self.endpoint.send(msg) {
    //         Ok(n) => {
    //             self.last_send = Some(Instant::now());
    //             info!("Sent {n} bytes to server!");
    //         }
    //         Err(ref e) => {
    //             error!("Failed to send data to the server: {e:?}");
    //         }
    //     }
    // }

    // fn process_message(&mut self, msg: &Message) -> Result<(), AppError> {
    //     match msg {
    //         Accepted => {
    //             self.state = ClientState::CONNECTED;
    //             info!("Connected to server!");
    //         }
    //         ServerInfo { key } => {
    //             let key = bitcode::deserialize::<RsaPublicKey>(key)
    //                 .map_err(|e| AppError::from("Unable to deserialize!"))?;
    //             self.server_key = Some(PublicKey::new(key));
    //             info!("Got server's public key!");
    //             self.send_connect_message();
    //         }
    //         Pong { time } => {
    //             info!(
    //                 "Ping to server is {:.2} ms.",
    //                 1000.0 * (Instant::now().elapsed().as_secs_f64() - time)
    //             );
    //         }
    //         Ping { time } => {
    //             self.send(&Pong { time: *time });
    //         }
    //         m => {
    //             warn!("Unsupported message from server: {m:?}");
    //         }
    //     }
    //     Ok(())
    // }

    fn receive_from_server(&mut self) {
        let mut buf = Vec::with_capacity(MAX_DATAGRAM_SIZE);
        loop {
            match receive_data(&self.socket, buf.as_mut()) {
                Ok(Some((_, addr))) => {
                    self.last_seen = Some(Instant::now());
                    let mut reader = NetBufReader::new(buf.as_slice());
                    while reader.available() >= MIN_HEADER_SIZE {
                        match read_header(&mut reader) {
                            Ok(header) => {
                                info!("Got packet {:?} from {}", header, addr);
                                let amount = header.size as usize;

                                match header.kind {
                                    PacketKind::ServerInfo => {
                                        let _ = self.on_server_info(&header, &mut reader)
                                            .inspect_err(|e| error!("Failed to process: {:?}", e));
                                    }
                                    //PacketKind::Connect => reader.skip(header.size)?,
                                    //PacketKind::Accepted => reader.skip(header.size)?,
                                    //PacketKind::Rejected => reader.skip(header.size),
                                    //PacketKind::Ping => reader.skip(header.size),
                                    //PacketKind::Pong => reader.skip(header.size),
                                    _ => {
                                        if let Err(e) = reader.skip(amount) {
                                            error!("Failed to skip packet: {e:?}");
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to read server packet: {e:?}");
                                break;
                            }
                        }
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
    }

    // fn send_connect_message(&mut self) {
    //     let key = self.server_key.as_ref().unwrap();
    //     let encoded = key.encode_str("123456").unwrap();
    //     self.send(&Message::Connect {
    //         name: "Test",
    //         password: encoded,
    //     })
    // }

    fn is_time_to_resend(&self) -> bool {
        Self::CONN_RETRY_INTERVAL
            <= self
                .last_send
                .map_or_else(|| Self::CONN_RETRY_INTERVAL, |v| v.elapsed())
    }

    pub(crate) fn frame_start(&mut self) {
        match self.socket.take_error() {
            Ok(Some(error)) => error!("Socket error: {error:?}"),
            Ok(None) => {}
            Err(error) => error!("Unable to take error: {error:?}"),
        }
    }

    pub(crate) fn update(&mut self, app: &Arc<App>) {
        self.receive_from_server();
        if self.is_time_to_resend() {
            match self.state {
                ClientState::Disconnected => {
                    if let Some(addr) = app.config().lock().unwrap().server.bound_to.as_ref() {
                        let addr: SocketAddr =
                            addr.parse().expect("Unable to parse server address!");
                        match self.socket.connect(addr) {
                            Ok(()) => {
                                info!("Client socket connected to {}", addr);
                                self.state = ClientState::Connected;
                                self.server_addr = Some(addr)
                            }
                            Err(e) => {
                                error!("Unable to connect socket: {}", e);
                            }
                        }
                    }
                }
                ClientState::Connected => {
                    let _ = self
                        .send_hello()
                        .inspect_err(|e| error!("Failed to send: {:?}", e));
                    self.state = ClientState::AwaitingAcceptance;
                }
                ClientState::AwaitingAcceptance => {
                    if !self.server_key.is_some() {
                        self.send_hello()
                            .inspect_err(|e| error!("Failed to send: {:?}", e));
                    } else {
                        self.send_connect_message();
                    };
                }
                ClientState::Accepted => {
                    // for i in 0..10 {
                    //     self.send(&Ping {
                    //         time: Instant::now().elapsed().as_secs_f64(),
                    //     });
                    // }
                }
            }
        }
    }

    pub(crate) fn frame_end(&mut self) {
        let bufs = &mut self.send_bufs;
        let socket = &self.socket;
        while let Some(b) = bufs.pop_front() {
            match socket.send(b.as_slice()) {
                Ok(amount) => {
                    if amount < b.len() {
                        warn!("Partial write!");
                    }
                    self.last_send = Some(Instant::now());
                }
                Err(e) => {
                    bufs.push_front(b);
                    if e.kind() == ErrorKind::WouldBlock {
                        break;
                    } else {
                        error!("Unable to send data: {:?}", e);
                        break;
                    }
                }
            }
        }
    }
}
