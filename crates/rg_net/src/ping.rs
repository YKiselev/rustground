use crate::{NetReader, NetWriter, Ping, ProtocolError};

pub fn write_ping<W>(writer: &mut W, time: f64) -> Result<(), ProtocolError>
where
    W: NetWriter,
{
    writer.write_f64(time)
}

pub fn read_ping<'a, R>(reader: &mut R) -> Result<Ping, ProtocolError>
where
    R: NetReader<'a>,
{
    Ok(Ping {
        time: reader.read_f64()?,
    })
}
