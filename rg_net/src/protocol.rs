use std::{array::TryFromSliceError, fmt::Debug};

use musli_zerocopy::{endian, Endian, Ref, ZeroCopy};
use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum ProtocolError {
    #[snafu(display("Buffer underflow"))]
    BufferUnderflow,
    #[snafu(display("Buffer overflow"))]
    BufferOverflow,
    #[snafu(display("Value is too big"))]
    ValueTooBig,
    #[snafu(display("Bad string"))]
    BadString,
}

impl From<TryFromSliceError> for ProtocolError {
    fn from(_: TryFromSliceError) -> Self {
        ProtocolError::BufferUnderflow
    }
}

///
/// Packet kinds
///

#[derive(Debug, ZeroCopy, PartialEq)]
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
/// 
/// 
type be_u16 = Endian<u16, endian::Big>;
type be_u32 = Endian<u32, endian::Big>;


///
/// Header
/// 
#[derive(ZeroCopy, Debug)]
#[repr(C)]
pub struct Header {
    pub size: be_u16,
    pub kind: PacketKind
}

///
/// Protocol version
/// 
#[derive(Debug, ZeroCopy, PartialEq)]
#[repr(C)]
pub struct Version(u8, u8, u8);

///
/// Hello packet
/// 
#[derive(Debug, ZeroCopy, PartialEq)]
#[repr(C)]
pub struct Hello {
    pub version: Version,
}

///
/// ServerInfo packet. Sent by server in response to Hello packet from client
/// 
#[derive(Debug, ZeroCopy, PartialEq)]
#[repr(C)]
pub struct ServerInfo {
    pub version: Version,
    pub key: Ref<[u8]>
}

///
/// Connect packet. Sent by client
/// 
#[derive(Debug, ZeroCopy, PartialEq)]
#[repr(C)]
pub struct Connect {
    pub name: Ref<str>,
    pub password: Ref<str>
}

///
/// Ping packet
/// 
#[derive(Debug, ZeroCopy, PartialEq)]
#[repr(C)]
pub struct Ping {
    pub time: f64
}

pub const PROTOCOL_VERSION: Version = Version(1, 0, 0);
