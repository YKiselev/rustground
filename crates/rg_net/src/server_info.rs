use crate::{
    net_rw::{NetReader, NetWriter},
    protocol::{ProtocolError, ServerInfo},
    version::{read_protocol_version, write_protocol_version},
};

///
/// Layout:
/// u8 proto_version_hi
/// u8 proto_version_lo
/// u8 * N public key
///
pub fn write_server_info<W>(writer: &mut W, key: &[u8]) -> Result<(), ProtocolError>
where
    W: NetWriter,
{
    write_protocol_version(writer)?;
    writer.write_bytes(key)
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
        net_rw::{NetBufReader, NetBufWriter},
        protocol::PROTOCOL_VERSION,
        server_info::read_server_info,
    };

    use super::write_server_info;

    #[test]
    fn write_read() {
        let buf = &mut [0u8; 16];
        let key = &[1u8; 10];
        let mut writer = NetBufWriter::new(buf);
        write_server_info(&mut writer, key).unwrap();
        let mut reader = NetBufReader::new(buf);
        let info = read_server_info(&mut reader).unwrap();
        assert_eq!(PROTOCOL_VERSION, info.version);
        assert_eq!(key, info.key);
    }
}
