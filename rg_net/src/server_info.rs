use crate::{
    net_rw::{NetBufReader, NetBufWriter, NetReader, NetWriter, WithPosition},
    protocol::{PacketKind, ProtocolError, ServerInfo},
    version::{read_protocol_version, write_protocol_version},
};

///
/// Layout:
/// u8 packet kind (ServerInfo)
/// u8 proto_version_hi
/// u8 proto_version_lo
/// u8 * N public key
///
pub fn write_server_info(buf: &mut [u8], key: &[u8]) -> Result<usize, ProtocolError> {
    let mut writer = NetBufWriter::new(buf);
    writer.write_u8(PacketKind::ServerInfo.into())?;
    write_protocol_version(&mut writer)?;
    writer.write_bytes(key)?;
    Ok(writer.pos())
}

pub fn read_server_info<'a, R>(reader: &mut R) -> Result<ServerInfo<'a>, ProtocolError>
where
    R: NetReader<'a>,
{
    let version = read_protocol_version(reader)?;
    let key = reader.read_bytes()?;
    Ok(ServerInfo { version, key })
}

#[cfg(test)]
mod tests {
    use crate::{
        net_rw::{NetBufReader, NetReader},
        protocol::{PacketKind, PROTOCOL_VERSION},
        server_info::read_server_info,
    };

    use super::write_server_info;

    #[test]
    fn write_read() {
        let buf = &mut [0u8; 16];
        let key = &[1u8; 10];
        assert_eq!(15, write_server_info(buf, key).unwrap());
        let mut reader = NetBufReader::new(buf);
        assert_eq!(PacketKind::ServerInfo, reader.read_u8_enum().unwrap());
        let info = read_server_info(&mut reader).unwrap();
        assert_eq!(PROTOCOL_VERSION, info.version);
        assert_eq!(key, info.key);
    }
}
