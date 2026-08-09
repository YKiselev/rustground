
use crate::{NetReader, NetWriter, Pong, ProtocolError};


pub fn write_pong<W>(writer: &mut W, t: f64) -> Result<(), ProtocolError>
where
    W: NetWriter,
{
    writer.write_f64(t)
}

pub fn read_pong<'a, R>(reader: &mut R) -> Result<Pong, ProtocolError>
where
    R: NetReader<'a>,
{
    Ok(Pong {
        time: reader.read_f64()?,
    })
}