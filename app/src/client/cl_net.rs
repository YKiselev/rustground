use std::{io::ErrorKind, net::SocketAddr};

use log::info;
use mio::net::UdpSocket;
use rg_net::protocol::NET_BUF_SIZE;

use crate::error::AppError;


pub(crate) fn receive_data(socket: &UdpSocket, buf: &mut Vec<u8>) -> Result<Option<(usize, SocketAddr)>, AppError> {
    buf.resize(NET_BUF_SIZE, 0);
    match socket.recv_from(buf.as_mut_slice()) {
        Ok((amount, addr)) => {
            if amount > 0 {
                buf.truncate(amount);
                Ok(Some((amount, addr)))
            } else {
                Ok(None)
            }
        }
        Err(e) => {
            return if e.kind() == ErrorKind::WouldBlock {
                Ok(None) // no data yet
            } else {
                Err(AppError::IoError { kind: e.kind() })
            };
        }
    }
}
