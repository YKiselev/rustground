use std::collections::VecDeque;
use std::io::{ErrorKind, Read};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::usize::MAX;

use log::{debug, error, info, warn};
use mio::net::UdpSocket;
use nom::Err;
use rg_common::app::App;
use rg_net::connect::write_connect;
use rg_net::header::{read_header, write_with_header};
use rg_net::hello::write_hello;
use rg_net::net_rw::{try_write, NetBufReader, NetBufWriter, NetReader, NetWriter, WithPosition};
use rg_net::ping::write_ping;
use rg_net::protocol::{
    Header, PacketKind, ProtocolError, MAX_DATAGRAM_SIZE, MIN_HEADER_SIZE, NET_BUF_SIZE,
};
use rg_net::server_info::read_server_info;
use rg_net::{process_buf, read_accepted, read_rejected};

use crate::client::cl_pub_key::PublicKey;
use crate::error::AppError;

use super::cl_net::receive_data;

#[derive(Eq, PartialEq, Clone, Copy)]
enum ClientState {
    Disconnected,
    AwaitingAcceptance,
    Accepted,
}

#[derive(Debug, Default)]
struct ServerProps {
    addr: Option<SocketAddr>,
    key: Option<PublicKey>,
    password: Option<String>,
}

pub(crate) struct Client {
    socket: UdpSocket,
    send_bufs: VecDeque<Vec<u8>>,
    server_props: ServerProps,
    state: ClientState,
    last_seen: Option<Instant>,
    last_send: Option<Instant>,
}

impl Client {
    const MAX_LAST_SEEN: Duration = Duration::from_secs(3);
    const CONN_RETRY_INTERVAL: Duration = Duration::from_secs(3);

    pub(crate) fn new(app: &Arc<App>) -> Result<Self, AppError> {
        info!("Starting client...");
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0).into())
            .expect("Unable to create client socket!");
        //let _ = app.config().lock()?;
        Ok(Client {
            socket,
            send_bufs: VecDeque::new(),
            server_props: ServerProps::default(),
            state: ClientState::Disconnected,
            last_seen: None,
            last_send: None,
        })
    }

    fn write_to_send_buf<H>(&mut self, mut handler: H) -> Result<(), ProtocolError>
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

    fn send_hello(&mut self) -> Result<(), AppError> {
        Ok(self
            .write_to_send_buf(|w| write_with_header(w, PacketKind::Hello, |w| write_hello(w)))?)
    }

    fn send_connect(&mut self) -> Result<(), AppError> {
        if let Some(key) = self.server_props.key.as_ref() {
            let encoded = key.encode_str("123456")?;
            Ok(self.write_to_send_buf(|w| {
                write_with_header(w, PacketKind::Connect, |w| {
                    write_connect(w, "user1", encoded.as_slice())
                })
            })?)
        } else {
            Err(AppError::IllegalState {
                message: "No server key to encode data!".to_owned(),
            })
        }
    }

    fn send_ping(&mut self) -> Result<(), AppError> {
        Ok(self.write_to_send_buf(|w| {
            write_with_header(w, PacketKind::Connect, |w| {
                write_ping(w)
            })
        })?)
    }

    fn on_server_info<'a, R>(&mut self, reader: &mut R) -> Result<(), AppError>
    where
        R: NetReader<'a>,
    {
        let info = read_server_info(reader)?;
        let public_key = PublicKey::from_der(info.key)?;
        self.server_props.key = Some(public_key);
        info!("Got server key");
        if self.state == ClientState::AwaitingAcceptance {
            self.send_connect()
        } else {
            Ok(())
        }
    }

    fn on_accepted<'a, R>(&mut self, reader: &mut R) -> Result<(), AppError>
    where
        R: NetReader<'a>,
    {
        let _ = read_accepted(reader)?;
        self.state = ClientState::Accepted;
        info!("Accepted by the server");
        Ok(())
    }

    fn on_rejected<'a, R>(&mut self, reader: &mut R) -> Result<(), AppError>
    where
        R: NetReader<'a>,
    {
        let rejected = read_rejected(reader)?;
        error!("Server rejected connection: {:?}", rejected.reason);
        self.state = ClientState::Disconnected;
        Ok(())
    }

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
                    let _ = process_buf(&mut reader, |header, reader| {
                        debug!("Got server packet {:?} from {}", header, addr);

                        match header.kind {
                            PacketKind::ServerInfo => self.on_server_info(reader),
                            PacketKind::Accepted => self.on_accepted(reader),
                            PacketKind::Rejected => self.on_rejected(reader),
                            //PacketKind::Ping => reader.skip(header.size),
                            //PacketKind::Pong => reader.skip(header.size),
                            _ => Ok(()),
                        }
                        .inspect_err(|e| error!("Failed to process: {:?}", e))
                        .is_ok()
                    })
                    .inspect_err(|e| error!("Failed to process: {:?}", e));
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
            loop {
                let state = self.state;
                match state {
                    ClientState::Disconnected => {
                        if let Ok(cfg_guard) = app.config.lock() {
                            if let Some(addr) = cfg_guard.server.bound_to.as_ref() {
                                if let Ok(addr) = addr.parse::<SocketAddr>() {
                                    match self.socket.connect(addr) {
                                        Ok(_) => {
                                            info!("Client socket connected to {}", addr);
                                            self.state = ClientState::AwaitingAcceptance;
                                            self.server_props.addr = Some(addr);
                                            self.server_props.password =
                                                cfg_guard.server.password.clone();
                                        }
                                        Err(e) => {
                                            error!("Unable to connect socket: {}", e);
                                        }
                                    }
                                } else {
                                    warn!("Unable to parse socket address: {}", addr);
                                }
                            } else {
                                warn!("Server not bound yet?");
                            }
                        } else {
                            warn!("Unable to lock configuration!");
                        }
                    }
                    ClientState::AwaitingAcceptance => {
                        let _ = if !self.server_props.key.is_some() {
                            self.send_hello()
                        } else {
                            self.send_connect()
                        }
                        .inspect_err(|e| error!("Failed to send: {:?}", e));
                    }
                    ClientState::Accepted => {
                        if app.elapsed().as_secs() >= 10 {
                            let _ = app
                                .commands
                                .execute("quit")
                                .inspect_err(|e| error!("Quit failed: {:?}", e));
                        } else {
                            self.send_ping();
                        }
                    }
                }
                if state == self.state {
                    break;
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
