use crate::{
    net_rw::{NetReader, NetWriter},
    protocol::{Connect, ProtocolError},
};

///
/// Layout:
/// u8 * N name
/// u8 * K password
///
pub fn write_connect<W>(writer: &mut W, name: &str, password: &[u8]) -> Result<(), ProtocolError>
where
    W: NetWriter,
{
    writer.write_str(name)?;
    writer.write_bytes(password)
}

pub fn read_connect<'a, R>(reader: &mut R) -> Result<Connect<'a>, ProtocolError>
where
    R: NetReader<'a>,
{
    let name = reader.read_str()?;
    let password = reader.read_bytes()?;
    Ok(Connect { name, password })
}

#[cfg(test)]
mod tests {
    use crate::{
        connect::read_connect,
        net_rw::{NetBufReader, NetBufWriter},
    };

    use super::write_connect;

    #[test]
    fn test_write_read() {
        let buf = &mut [0u8; 20];
        let name = "StarLord";
        let password = "12345";
        let mut writer = NetBufWriter::new(buf);
        write_connect(&mut writer, name, password.as_bytes()).unwrap();
        let mut reader = NetBufReader::new(buf);
        let c = read_connect(&mut reader).unwrap();
        assert_eq!(name, c.name);
        assert_eq!(password.as_bytes(), c.password);
    }
}
