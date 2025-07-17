use std::{array::TryFromSliceError, fmt::Debug};

use num_enum::{FromPrimitive, IntoPrimitive, TryFromPrimitive};
use snafu::Snafu;

pub const MAX_DATAGRAM_SIZE: usize = 65507;
pub const NET_BUF_SIZE: usize = 65536;
pub const MIN_HEADER_SIZE: usize = 3;
pub const PROTOCOL_VERSION: Version = Version(1, 0);

#[derive(Debug, Snafu, PartialEq)]
pub enum ProtocolError {
    #[snafu(display("Index {index} is out of range 0..{size}"))]
    BufferUnderflow { index: usize, size: usize },
    #[snafu(display("Buffer overflow"))]
    BufferOverflow,
    #[snafu(display("Value is too big"))]
    ValueTooBig,
    #[snafu(display("Bad string"))]
    BadString,
    #[snafu(display("Bad enum tag"))]
    BadEnumTag,
}

impl ProtocolError {
    pub fn underflow(index: usize, size: usize) -> ProtocolError {
        ProtocolError::BufferUnderflow { index, size }
    }
}
impl From<TryFromSliceError> for ProtocolError {
    fn from(_: TryFromSliceError) -> Self {
        ProtocolError::underflow(0, 0)
    }
}

///
/// Packet kinds
///
#[derive(Debug, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum PacketKind {
    Hello,
    ServerInfo,
    Connect,
    Accepted,
    Rejected,
    Ping,
    Pong,
}

///
/// Protocol version
///
#[derive(Debug, PartialEq)]
pub struct Version(pub u8, pub u8);

///
///
///
#[derive(Debug, PartialEq)]
pub struct Header {
    pub kind: PacketKind,
    pub size: u16,
}

///
/// Hello packet
///
#[derive(Debug, PartialEq)]
pub struct Hello {
    pub version: Version,
}

///
/// ServerInfo packet. Sent by server in response to Hello packet from client
///
#[derive(Debug, PartialEq)]
pub struct ServerInfo<'a> {
    pub version: Version,
    pub key: &'a [u8],
}

///
/// Connect packet. Sent by client
///
#[derive(Debug, PartialEq)]
pub struct Connect<'a> {
    pub name: &'a str,
    pub password: &'a str,
}

#[derive(Debug, PartialEq)]
pub struct Accepted {

}

#[derive(Debug, PartialEq)]
pub struct Rejected<'a> {
    pub reason: &'a str
}

///
/// Ping packet
///
#[derive(Debug, PartialEq)]
pub struct Ping {
    pub time: f64,
}

///
/// Pong packet
///
#[derive(Debug, PartialEq)]
pub struct Pong {
    pub time: f64,
}

#[inline(always)]
pub fn check_bounds(offset: usize, size: usize) -> Result<(), ProtocolError> {
    if offset > size {
        Err(ProtocolError::underflow(offset, size))
    } else {
        Ok(())
    }
}
