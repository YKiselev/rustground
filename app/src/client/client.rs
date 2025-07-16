use std::io::{ErrorKind, Read};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{error, info, warn};
use mio::net::UdpSocket;
use rg_net::header::read_header;
use rg_net::net_rw::{NetBufReader, NetReader};
use rg_net::protocol::{PacketKind, MIN_HEADER_SIZE, NET_BUF_SIZE};
use rsa::RsaPublicKey;

use crate::app::App;
use crate::client::cl_pub_key::PublicKey;

use super::cl_net::receive_data;

#[derive(Eq, PartialEq)]
enum ClientState {
    Init,
    Connected,
    AwaitingAcceptance,
    Accepted,
}

pub(crate) struct Client {
    socket: UdpSocket,
    //recv_buf: Option<Vec<u8>>,
    send_bufs: Vec<Vec<u8>>,
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
        //endpoint.connect(&server_addr).expect("Unable to set server address on client socket!");
        Client {
            socket,
            //recv_buf: Some(Vec::with_capacity(MAX_DATAGRAM_SIZE)),
            send_bufs: Vec::new(),
            server_addr: None,
            server_key: None,
            state: ClientState::Init,
            last_seen: None,
            last_send: None,
        }
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
        let mut buf = Vec::new(); // self.recv_buf.take().unwrap_or_else(|| Vec::new());
        loop {
            match receive_data(&self.socket, buf.as_mut()) {
                Ok(Some((amount, addr))) => {
                    let mut reader = NetBufReader::new(buf.as_slice());
                    while reader.available() > MIN_HEADER_SIZE {
                        //self.process_message(m).unwrap();
                        match read_header(&mut reader) {
                            Ok(header) => {
                                let amount = header.size as usize;
                                if let Err(e) = reader.skip(amount) {
                                    error!("Failed to skip packet: {e:?}");
                                }
                                // match header.kind {
                                //     PacketKind::Hello => reader.skip(header.size)?,
                                //     PacketKind::ServerInfo => reader.skip(header.size)?,
                                //     PacketKind::Connect => reader.skip(header.size)?,
                                //     PacketKind::Accepted => reader.skip(header.size)?,
                                //     PacketKind::Rejected => reader.skip(header.size),
                                //     PacketKind::Ping => reader.skip(header.size),
                                //     PacketKind::Pong => reader.skip(header.size),
                                // }
                            }
                            Err(e) => {
                                error!("Failed to read packet: {e:?}");
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
        //self.recv_buf.replace(buf);
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
        // self.endpoint.clear_buffers();
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
                ClientState::Init => {
                    if let Some(addr) = app.config().lock().unwrap().server.bound_to.as_ref() {
                        match self
                            .socket
                            .connect(addr.parse().expect("Unable to parse server address!"))
                        {
                            Ok(_) => {
                                info!("Client socket connected to {}", addr);
                                self.state = ClientState::Connected;
                            }
                            Err(e) => {
                                error!("Unable to connect socket: {}", e);
                            }
                        }
                    }
                }
                ClientState::Connected => {
                    //self.send(&Hello);
                    self.state = ClientState::AwaitingAcceptance;
                }
                ClientState::AwaitingAcceptance => {
                    if !self.server_key.is_some() {
                        //self.send(&Hello);
                    } else {
                        //self.send_connect_message();
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
        // todo
        self.send_bufs.clear();
        // if let Err(e) = self.endpoint.flush() {
        //     if self.state == ClientState::INIT {
        //         error!("Flush failed: {}", e);
        //     }
        // }
    }
}
