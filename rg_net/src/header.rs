use crate::{
    net_rw::{write_u16, NetReader, NetWriter},
    protocol::{Header, PacketKind, ProtocolError},
};

pub fn write_with_header<W, F>(
    writer: &mut W,
    kind: PacketKind,
    block: F,
) -> Result<(), ProtocolError>
where
    W: NetWriter,
    F: FnOnce(&mut W) -> Result<(), ProtocolError>,
{
    let size_offset = write_header_placeholder(writer, kind)?;
    let start = writer.pos();
    block(writer)?;
    let size = writer.pos().saturating_sub(start);
    update_header_size(writer, size_offset, size)
}

#[inline(always)]
pub fn write_header_placeholder<W>(writer: &mut W, kind: PacketKind) -> Result<usize, ProtocolError>
where
    W: NetWriter,
{
    writer.write_u8(kind.into())?;
    let result = writer.pos();
    writer.write_u16(0)?;
    Ok(result)
}

#[inline(always)]
pub fn update_header_size<W>(
    writer: &mut W,
    offset: usize,
    size: usize,
) -> Result<(), ProtocolError>
where
    W: NetWriter,
{
    if size > u16::MAX as usize {
        return Err(ProtocolError::ValueTooBig);
    }
    writer.write_u16_at(offset, size as u16)
}

pub fn read_header<'a, R>(reader: &mut R) -> Result<Header, ProtocolError>
where
    R: NetReader<'a>,
{
    Ok(Header {
        kind: reader.read_u8_enum::<PacketKind>()?,
        size: reader.read_u16()?,
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        hello::{read_hello, write_hello},
        net_rw::{NetBufReader, NetBufWriter},
        protocol::{PacketKind, PROTOCOL_VERSION},
    };

    use super::{read_header, write_with_header};

    #[test]
    fn test_write_read() {
        let mut buf = [0; 16];
        {
            let mut writer = NetBufWriter::new(buf.as_mut_slice());

            write_with_header(&mut writer, PacketKind::Hello, { |w| write_hello(w) }).unwrap()
        }
        let mut reader = NetBufReader::new(buf.as_slice());
        let header = read_header(&mut reader).unwrap();
        assert_eq!(PacketKind::Hello, header.kind);
        assert_eq!(2, header.size);
        let hello = read_hello(&mut reader).unwrap();
        assert_eq!(PROTOCOL_VERSION.0, hello.version.0);
        assert_eq!(PROTOCOL_VERSION.1, hello.version.1);
    }
}
