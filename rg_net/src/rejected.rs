use crate::{net_rw::{NetReader, NetWriter}, protocol::ProtocolError, Rejected, RejectionReason};


pub fn write_rejected<W>(writer: &mut W, reason: RejectionReason) -> Result<(), ProtocolError>
where
    W: NetWriter,
{
    writer.write_u8(reason.into())
}

pub fn read_rejected<'a, R>(reader: &mut R) -> Result<Rejected, ProtocolError>
where
    R: NetReader<'a>,
{
    Ok(Rejected { 
        reason: reader.read_u8_enum::<RejectionReason>()?
    })
}