use crate::{net_rw::{NetReader, NetWriter}, protocol::{Accepted, ProtocolError}};


pub fn write_accepted<W>(writer: &mut W) -> Result<(), ProtocolError>
where
    W: NetWriter,
{
    // no-op
    Ok(())
}

pub fn read_accepted<'a, R>(reader: &mut R) -> Result<Accepted, ProtocolError>
where
    R: NetReader<'a>,
{
    Ok(Accepted { })
}