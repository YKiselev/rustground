use crate::{
    net_rw::{NetReader, NetWriter},
    protocol::{ProtocolError, Version, PROTOCOL_VERSION},
};

///
///
///

#[inline(always)]
pub fn write_protocol_version<W>(writer: &mut W) -> Result<(), ProtocolError>
where
    W: NetWriter,
{
    writer.write_u8(PROTOCOL_VERSION.0)?;
    writer.write_u8(PROTOCOL_VERSION.1)?;
    Ok(())
}

pub fn read_protocol_version<'a, R>(reader: &mut R) -> Result<Version, ProtocolError>
where
    R: NetReader<'a>,
{
    Ok(Version(reader.read_u8()?, reader.read_u8()?))
}
