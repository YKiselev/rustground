use crate::{
    ClientInfo,
    net_rw::{NetReader, NetWriter},
    protocol::ProtocolError,
};

///
/// Layout:
/// u8 * N public key
///
#[allow(dead_code)]
pub fn write_client_info<W>(writer: &mut W, key: &[u8]) -> Result<(), ProtocolError>
where
    W: NetWriter,
{
    writer.write_bytes(key)
}

#[allow(dead_code)]
pub fn read_client_info<'a, R>(reader: &mut R) -> Result<ClientInfo<'a>, ProtocolError>
where
    R: NetReader<'a>,
{
    let key = reader.read_bytes()?;
    Ok(ClientInfo { key })
}

#[cfg(test)]
mod tests {
    use crate::{
        client_info::read_client_info,
        net_rw::{NetBufReader, NetBufWriter},
    };

    use super::write_client_info;

    #[test]
    fn write_read() {
        let buf = &mut [0u8; 16];
        let key = &[1u8; 10];
        let mut writer = NetBufWriter::new(buf);
        write_client_info(&mut writer, key).unwrap();
        let mut reader = NetBufReader::new(buf);
        let info = read_client_info(&mut reader).unwrap();
        assert_eq!(key, info.key);
    }
}
