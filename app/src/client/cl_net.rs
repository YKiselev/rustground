use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use bytes::{Bytes, BytesMut};
use rg_common::{App, Plugin};
use rg_net::write_ping;
use rg_net::write_with_header;
use rg_net::{MAX_DATAGRAM_SIZE, write_connect};
use rg_net::{NetBufReader, NetBufWriter, NetReader, try_write};
use rg_net::{PacketKind, ProtocolError};
use rg_net::{read_accepted, read_rejected};
use rg_net::{read_pong, read_server_info};
use rg_net::{write_client_info, write_hello};
use tracing::{debug, error, info, warn};
use x25519_dalek::{EphemeralSecret, PublicKey};

use crate::application::async_runtime::ClientChannel;
use crate::cipher::Cipher;
use crate::client;
use crate::error::{AppError, to_illegal_state};

const BUF_ALLOCATOR_SIZE: usize = 8 * MAX_DATAGRAM_SIZE;

#[derive(Eq, PartialEq, PartialOrd, Clone, Copy, Debug)]
enum ClientState {
    Disconnected,
    Connecting,
    AwaitingAcceptance,
    Accepted,
}

#[derive(Debug, Default)]
struct ServerProps {
    addr: Option<SocketAddr>,
    key: Option<PublicKey>,
    password: Option<String>,
}

#[derive()]
pub(super) struct ClientNetwork {
    channel: ClientChannel,
    buf_allocator: BytesMut,
    send_bufs: VecDeque<BytesMut>,
    server_props: ServerProps,
    state: ClientState,
    last_seen: Option<Instant>,
    last_send: Option<Instant>,
    secret: Option<EphemeralSecret>,
    cipher: Option<Cipher>,
}

impl ClientNetwork {
    const MAX_LAST_SEEN: Duration = Duration::from_secs(3);
    const CONN_RETRY_INTERVAL: Duration = Duration::from_secs(3);

    pub(crate) fn new(_app: &Arc<App>, channel: ClientChannel) -> Result<Self, AppError> {
        info!("Creating client network...");
        Ok(ClientNetwork {
            channel,
            buf_allocator: BytesMut::with_capacity(BUF_ALLOCATOR_SIZE),
            send_bufs: VecDeque::new(),
            server_props: ServerProps::default(),
            state: ClientState::Connecting,
            last_seen: None,
            last_send: None,
            secret: None,
            cipher: None,
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

            if !self.buf_allocator.try_reclaim(MAX_DATAGRAM_SIZE) {
                warn!("Unable to reclaim {} bytes", MAX_DATAGRAM_SIZE);
            }

            let rest = self.buf_allocator.split_off(MAX_DATAGRAM_SIZE);
            let new_buf = std::mem::replace(&mut self.buf_allocator, rest);
            self.send_bufs.push_back(new_buf);
        }
        Ok(())
    }

    fn send_hello(&mut self) -> Result<(), AppError> {
        Ok(self
            .write_to_send_buf(|w| write_with_header(w, PacketKind::Hello, |w| write_hello(w)))?)
    }

    fn send_client_info(&mut self) -> Result<(), AppError> {
        info!("Sending client info...");

        if let Some(my_secret) = self.secret.as_ref() {
            let my_public = PublicKey::from(my_secret);

            self.write_to_send_buf(|w| {
                write_with_header(w, PacketKind::ClientInfo, |w| {
                    write_client_info(w, my_public.as_bytes())
                })
            })?;
        } else {
            self.write_to_send_buf(|w| {
                write_with_header(w, PacketKind::ClientInfo, |w| {
                    write_client_info(w, &[0u8; 0])
                })
            })?;
        }

        if let Some(secret) = self.secret.take() {
            if let Some(public) = self.server_props.key.as_ref() {
                let cipher = Cipher::new(secret, public)?;
                self.cipher = Some(cipher);
                info!("Cipher initialized");
            } else {
                self.cipher = None;
                warn!("Missing server public key!");
            }
        }

        Ok(())
    }

    fn send_connect(&mut self) -> Result<(), AppError> {
        info!("Sending connect...");
        if let Some(cipher) = self.cipher.as_mut() {
            let encoded = cipher.encode_str("123456")?;
            Ok(self.write_to_send_buf(|w| {
                write_with_header(w, PacketKind::Connect, |w| {
                    write_connect(w, "user1", &encoded)
                })
            })?)
        } else {
            Err(to_illegal_state("No cipher to encode data!"))
        }
    }

    fn send_ping(&mut self) -> Result<(), AppError> {
        Ok(self.write_to_send_buf(|w| {
            write_with_header(w, PacketKind::Ping, |w| write_ping(w, get_ping_seconds()))
        })?)
    }

    fn on_server_info<'a, R>(&mut self, reader: &mut R) -> Result<(), AppError>
    where
        R: NetReader<'a>,
    {
        if self.state == ClientState::AwaitingAcceptance {
            info!("Got server info");
            let info = read_server_info(reader)?;
            let server_key: Result<[u8; 32], _> = info.key.try_into();

            // If server don't use encryption
            if server_key.is_err() {
                debug!("There is no server key");

                // Clear out our secret
                self.secret.take();
                self.server_props.key = None;
            } else {
                debug!("Got server key");

                self.server_props.key = Some(PublicKey::from(server_key.unwrap()));

                if self.secret.is_none() {
                    let secret = EphemeralSecret::random();
                    self.secret = Some(secret);
                }
            };

            let _ = self.send_client_info()?;
            let _ = self.send_connect()?;
        }
        Ok(())
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

    fn on_pong<'a, R>(&mut self, reader: &mut R) -> Result<(), AppError>
    where
        R: NetReader<'a>,
    {
        let pong = read_pong(reader)?;
        let now = get_ping_seconds();
        let roundtrip_time = 1000.0 * 0.5 * (now - pong.time);
        info!("Server ping: {:.1} millis.", roundtrip_time.abs());
        Ok(())
    }

    fn read_from_channel(&mut self, app: &Arc<App>) {
        let rx = self.channel.rx.clone();
        for response in rx.try_iter() {
            match response {
                client::Response::DatagramReceived { bytes, address } => {
                    self.process_datagram(address, bytes);
                }
                client::Response::Connected(addr) => {
                    info!("Client socket connected to {}", addr);
                    self.state = ClientState::AwaitingAcceptance;
                    self.server_props.addr = Some(addr);
                    // debug
                    self.server_props.password = app.vars.try_get_value("server::password");
                }
                client::Response::Error(e) => {
                    warn!("Async runtime reports error: {:?}", e);
                }
            }
        }
    }

    fn process_datagram(&mut self, address: SocketAddr, bytes: Bytes) {
        self.last_seen = Some(Instant::now());

        let mut reader = NetBufReader::new(&bytes);

        debug!("Got {} bytes from async runtime", bytes.len());

        while let Some((header, mut payload)) = reader.read_next_packet() {
            debug!("Got server packet {:?} from {}", header, address);

            let _ = match header.kind {
                PacketKind::ServerInfo => self.on_server_info(&mut payload),
                PacketKind::Accepted => self.on_accepted(&mut payload),
                PacketKind::Rejected => self.on_rejected(&mut payload),
                //PacketKind::Ping => reader.skip(header.size),
                PacketKind::Pong => self.on_pong(&mut payload),
                other => {
                    warn!("Unexpected packet: {:?}", other);
                    Ok(())
                }
            }
            .inspect_err(|e| error!("Failed to process: {:?}", e));
        }
    }

    fn is_time_to_resend(&self) -> bool {
        Self::CONN_RETRY_INTERVAL
            <= self
                .last_send
                .map_or_else(|| Self::CONN_RETRY_INTERVAL, |v| v.elapsed())
    }

    pub(super) fn frame_start(&mut self, _app: &Arc<App>) {}

    pub(super) fn update(&mut self, app: &Arc<App>) {
        self.read_from_channel(app);
        if self.is_time_to_resend() {
            loop {
                let state = self.state;
                match state {
                    ClientState::Disconnected => {}
                    ClientState::Connecting => {
                        if let Some(addr) = app.vars.try_get_value("server::bound_to") {
                            if let Ok(addr) = addr.parse::<SocketAddr>() {
                                if let Err(e) =
                                    self.channel.tx.send(client::Request::NetworkConnect(addr))
                                {
                                    warn!("Unable to send message to async runtime: {}", e);
                                }
                            } else {
                                warn!("Unable to parse socket address: {}", addr);
                            }

                            self.last_send = Some(Instant::now());
                        } else {
                            warn!("Server not bound yet?");
                        }
                    }
                    ClientState::AwaitingAcceptance => {
                        let _ = if !self.cipher.is_some() {
                            self.send_hello()
                        } else {
                            self.send_connect()
                        }
                        .inspect_err(|e| error!("Failed to send: {:?}", e));
                    }
                    ClientState::Accepted => {
                        let _ = self
                            .send_ping()
                            .inspect_err(|e| error!("Failed to send: {:?}", e));
                    }
                }
                if state == self.state {
                    break;
                }
            }
        }
    }

    pub(super) fn frame_end(&mut self, _app: &Arc<App>) {
        let bufs = &mut self.send_bufs;
        let mut sends = 0;
        while let Some(bytes) = bufs.pop_front() {
            if let Err(_) = self.channel.tx.send(client::Request::SendDatagram {
                bytes: bytes.freeze(),
            }) {
                debug!("Send channel is closed!");
            }
            sends += 1;
        }
        if sends > 0 {
            self.last_send = Some(Instant::now());
        }
    }
}

fn get_ping_seconds() -> f64 {
    static TIME: OnceLock<Instant> = OnceLock::new();
    let time = TIME.get_or_init(|| Instant::now());
    time.elapsed().as_secs_f64()
}
