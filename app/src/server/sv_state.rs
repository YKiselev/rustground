use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use bytes::{Bytes, BytesMut};
use rg_common::App;
use rg_net::{
    NET_BUF_SIZE, NetBufReader, PacketKind, read_client_info, read_connect, read_hello, read_ping,
};
use tracing::{debug, info, warn};

use crate::{
    application::async_runtime::ServerChannel,
    error::AppError,
    server::{
        self,
        server::ServerConfig,
        sv_clients::{ClientId, Clients},
        sv_guests::Guests,
        sv_security::ServerSecurity,
    },
};

const BUF_ALLOCATOR_CAPACITY: usize = 16 * NET_BUF_SIZE;

#[derive()]
pub(super) struct ServerState {
    config: Arc<RwLock<ServerConfig>>,
    clients: Clients,
    guests: Guests,
    security: ServerSecurity,
    channel: ServerChannel,
    buf_allocator: BytesMut,
}

impl ServerState {
    pub fn new(
        _app: &App,
        config: &Arc<RwLock<ServerConfig>>,
        channel: ServerChannel,
    ) -> Result<Self, AppError> {
        info!("Starting server...");
        let cfg = config.read()?;
        let addr: SocketAddr = cfg.address.parse()?;
        let _ = channel
            .tx
            .send(server::Request::StartNetworkLoop(addr))
            .map_err(|e| AppError::ChannelError(e.to_string()))?;

        let security = ServerSecurity::new(cfg.password.to_owned())?;
        let buf_allocator = BytesMut::with_capacity(BUF_ALLOCATOR_CAPACITY);

        drop(cfg);

        Ok(ServerState {
            config: Arc::clone(config),
            clients: Clients::new(),
            guests: Guests::new(),
            security,
            channel,
            buf_allocator,
        })
    }

    pub fn shutdow(self) {
        if let Err(_) = self.channel.tx.send(server::Request::StopNetworkLoop) {
            warn!("Unable to send shutdown signal to async workers!");
        }
    }

    pub fn update(&mut self) -> Result<(), AppError> {
        let rx = self.channel.rx.clone();
        for p in rx.try_iter() {
            match p {
                server::Response::Error(e) => {
                    warn!("Async runtime reports error: {}", e);
                }
                server::Response::NetworkLoopStarted(socket_addr) => {
                    let mut cfg = self.config.write()?;
                    cfg.bound_to = Some(socket_addr.to_string());
                }
                server::Response::DatagramReceived { bytes, address } => {
                    self.process_network_datagram(address, bytes);
                }
            }
        }

        self.clients.flush(&self.channel.tx);
        self.guests.flush(&self.channel.tx);

        Ok(())
    }

    fn process_network_datagram(&mut self, address: SocketAddr, bytes: Bytes) {
        let clients = &mut self.clients;
        let guests = &mut self.guests;
        let security = &self.security;

        let client_id = ClientId::new(address);
        let mut reader = NetBufReader::new(&bytes);

        while let Some((header, mut payload)) = reader.read_next_packet() {
            debug!("Got {:?} from client {}", header, address);

            match header.kind {
                PacketKind::Hello => {
                    if !clients.exists(&client_id) {
                        match read_hello(&mut payload) {
                            Ok(ref hello) => {
                                guests.on_hello(&client_id, hello, &mut self.buf_allocator)
                            }
                            Err(e) => {
                                warn!("Failed to parse: {:?}", e)
                            }
                        }
                    }
                }

                PacketKind::ClientInfo => {
                    if !clients.exists(&client_id) {
                        match read_client_info(&mut payload) {
                            Ok(ref info) => guests.on_client_info(&client_id, info),
                            Err(e) => {
                                warn!("Failed to parse: {:?}", e)
                            }
                        }
                    }
                }

                PacketKind::Connect => match read_connect(&mut payload) {
                    Ok(ref connect) => match guests.on_connect(
                        &client_id,
                        connect,
                        security,
                        &mut self.buf_allocator,
                    ) {
                        Ok(cipher) => {
                            if let Some(cipher) = cipher {
                                clients.add(client_id, connect.name, cipher);
                            }
                        }
                        Err(e) => {
                            warn!("Unable to connect client: {:?}", e);
                        }
                    },
                    Err(e) => {
                        warn!("Failed to parse: {}", e);
                    }
                },

                PacketKind::Ping => match read_ping(&mut payload) {
                    Ok(ref ping) => {
                        guests.on_ping(&client_id, ping, &mut self.buf_allocator);
                    }
                    Err(e) => {
                        warn!("Failed to parse: {:?}", e);
                    }
                },

                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::{Bytes, BytesMut};

    fn alloc2(buf: &mut BytesMut) -> Option<Bytes> {
        let required = 100;
        if buf.capacity() < required {
            if !buf.try_reclaim(required) {
                println!("Failed to reclaim!");
                return None;
            }
        }

        let rest = buf.split_off(required);
        let mut x = std::mem::replace(buf, rest);

        x.clear();
        x.extend_from_slice(b"O, my sweet slice of bytes!");
        Some(x.freeze())
    }

    #[test]
    fn test() {
        let cap = 1024;
        let mut buf = BytesMut::with_capacity(cap);
        // #1
        {
            let allocations: Vec<_> = (0..10)
                .map(|_| alloc2(&mut buf))
                .filter(|v| v.is_some())
                .collect();
            println!("Got {} allocations!", allocations.len());
            println!(
                "buf(len={}, cap={})",
                buf.len(),
                buf.capacity()
            );
        }
        println!(
            "After first pass: buf(len={}, cap={})",
            buf.len(),
            buf.capacity()
        );
        // #2
        {
            let allocations: Vec<_> = (0..10)
                .map(|_| alloc2(&mut buf))
                .filter(|v| v.is_some())
                .collect();
            println!("Got {} allocations!", allocations.len());
        }
        println!(
            "After second pass: buf{:p}(len={}, cap={})",
            buf.as_ptr(),
            buf.len(),
            buf.capacity()
        );
        if !buf.try_reclaim(1024) {
            println!("Oops!");
        }
        println!(
            "After reclaim: buf(len={}, cap={})",
            buf.len(),
            buf.capacity()
        );
        println!("Done!");
    }
}
