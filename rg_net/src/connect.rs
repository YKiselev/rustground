use crate::{
    net_rw::{NetBufWriter, NetReader, NetWriter, WithPosition},
    protocol::{Connect, PacketKind, ProtocolError, ServerInfo},
};

///
/// Layout:
/// u8 packet kind (Connect)
/// u8 * N name
/// u8 * K password
///
pub fn write_connect(buf: &mut [u8], name: &str, password: &str) -> Result<usize, ProtocolError> {
    let mut writer = NetBufWriter::new(buf);
    writer.write_u8(PacketKind::Connect.into())?;
    writer.write_str(name)?;
    writer.write_str(password)?;
    Ok(writer.pos())
}

pub fn read_connect<'a, R>(reader: &mut R) -> Result<Connect<'a>, ProtocolError>
where
    R: NetReader<'a>,
{
    let name = reader.read_str()?;
    let password = reader.read_str()?;
    Ok(Connect { name, password })
}

#[cfg(test)]
mod tests {
    use crate::{
        connect::read_connect,
        net_rw::{NetBufReader, NetReader},
        protocol::{PacketKind, ProtocolError},
    };

    use super::write_connect;

    #[test]
    fn test_write_read() {
        let buf = &mut [0u8; 20];
        let name = "StarLord";
        let password = "12345";
        assert_eq!(18, write_connect(buf, name, password).unwrap());
        let mut reader = NetBufReader::new(buf);
        assert_eq!(PacketKind::Connect, reader.read_u8_enum().unwrap());
        let c = read_connect(&mut reader).unwrap();
        assert_eq!(name, c.name);
        assert_eq!(password, c.password);
    }
}
