use crate::{
    net_rw::{NetReader, NetWriter},
    protocol::{Hello, ProtocolError},
    version::{read_protocol_version, write_protocol_version},
};

///
/// Writes Hello message
/// Layout:
/// u8 proto_version_hi
/// u8 proto_version_lo
///
pub fn write_hello<W>(writer: &mut W) -> Result<(), ProtocolError>
where
    W: NetWriter,
{
    write_protocol_version(writer)
}

pub fn read_hello<'a, R>(reader: &mut R) -> Result<Hello, ProtocolError>
where
    R: NetReader<'a>,
{
    Ok(Hello {
        version: read_protocol_version(reader)?,
    })
}
