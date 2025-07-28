use std::time::Instant;

use crate::{NetReader, NetWriter, Ping, ProtocolError};


pub fn write_ping<W>(writer: &mut W) -> Result<(), ProtocolError>
where
    W: NetWriter,
{
    writer.write_f64(Instant::now().elapsed().as_secs_f64())
}

pub fn read_ping<'a, R>(reader: &mut R) -> Result<Ping, ProtocolError>
where
    R: NetReader<'a>,
{
    Ok(Ping {
        time: reader.read_f64()?,
    })
}