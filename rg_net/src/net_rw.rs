use log::error;
use num_enum::TryFromPrimitive;

use crate::{
    protocol::{check_bounds, ProtocolError, NET_BUF_SIZE},
    read_header, Header, MIN_HEADER_SIZE,
};

///
/// align is expected to be power of two
///
#[inline(always)]
pub fn padding(offset: usize, align: usize) -> usize {
    let mask = align - 1;
    (align - (offset & mask)) & mask
}

pub trait WithPosition {
    fn pos(&self) -> usize;
    fn set_pos(&mut self, pos: usize) -> Result<usize, ProtocolError>;
}

pub trait NetWriter: WithPosition {
    fn write_u8(&mut self, value: u8) -> Result<(), ProtocolError>;
    fn write_u16(&mut self, value: u16) -> Result<(), ProtocolError>;
    fn write_u16_at(&mut self, offset: usize, value: u16) -> Result<(), ProtocolError>;
    ///
    /// Bytes layout is 2 bytes prefix (data length) (u16) and then data itself
    /// Of course you can't write more than u16::MAX bytes.
    ///
    fn write_bytes(&mut self, value: &[u8]) -> Result<(), ProtocolError>;
    ///
    /// String length is limited to u16::MAX
    ///
    fn write_str(&mut self, value: &str) -> Result<(), ProtocolError> {
        self.write_bytes(value.as_bytes())
    }
}

pub trait NetReader<'a>: WithPosition {
    fn available(&self) -> usize;
    fn skip(&mut self, amount: usize) -> Result<usize, ProtocolError> {
        let p = self.pos();
        self.set_pos(p + amount)
    }
    fn read_u8(&mut self) -> Result<u8, ProtocolError>;
    fn read_u16(&mut self) -> Result<u16, ProtocolError>;
    ///
    /// Bytes layout is 2 bytes prefix (data length) (u16) and then data itself
    /// Of course you can't write more than u16::MAX bytes.
    ///
    fn read_bytes(&mut self) -> Result<&'a [u8], ProtocolError>;
    ///
    /// String length is limited to u16::MAX
    ///
    fn read_str(&mut self) -> Result<&'a str, ProtocolError> {
        self.read_bytes()
            .and_then(|bytes| std::str::from_utf8(bytes).map_err(|_| ProtocolError::BadString))
    }

    fn read_u8_enum<E>(&mut self) -> Result<E, ProtocolError>
    where
        E: TryFromPrimitive<Primitive = u8>,
    {
        self.read_u8()
            .and_then(|v| E::try_from_primitive(v).map_err(|_| ProtocolError::BadEnumTag))
    }
}

pub struct NetBufWriter<'a> {
    buf: &'a mut [u8],
    offset: usize,
}

impl<'a> NetBufWriter<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self {
            buf: buf,
            offset: 0,
        }
    }

    pub fn buf(&mut self) -> &mut [u8] {
        self.buf
    }
}

impl WithPosition for NetBufWriter<'_> {
    fn pos(&self) -> usize {
        self.offset
    }

    fn set_pos(&mut self, pos: usize) -> Result<usize, ProtocolError> {
        if pos > self.buf.len() {
            return Err(ProtocolError::BufferOverflow);
        }
        self.offset = pos;
        Ok(pos)
    }
}

impl NetWriter for NetBufWriter<'_> {
    fn write_u8(&mut self, value: u8) -> Result<(), ProtocolError> {
        write_u8(self.buf, self.offset, value)?;
        self.offset += 1;
        Ok(())
    }

    fn write_u16(&mut self, value: u16) -> Result<(), ProtocolError> {
        write_u16(self.buf, self.offset, value)?;
        self.offset += 2;
        Ok(())
    }

    fn write_bytes(&mut self, value: &[u8]) -> Result<(), ProtocolError> {
        let len = value.len();
        if self.offset + len > self.buf.len() {
            return Err(ProtocolError::BufferOverflow);
        }
        if len > u16::MAX as usize {
            return Err(ProtocolError::ValueTooBig);
        }
        self.write_u16(len as u16)?;
        let data_offset = self.offset;
        self.buf[data_offset..data_offset + len].copy_from_slice(value);
        self.offset = data_offset + len;
        Ok(())
    }

    fn write_u16_at(&mut self, offset: usize, value: u16) -> Result<(), ProtocolError> {
        write_u16(self.buf, offset, value)
    }
}

pub struct NetBufReader<'a> {
    buf: &'a [u8],
    offset: usize,
}

impl<'a> NetBufReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self {
            buf: buf,
            offset: 0,
        }
    }

    pub fn buf(&self) -> &[u8] {
        &self.buf
    }
}

impl WithPosition for NetBufReader<'_> {
    fn pos(&self) -> usize {
        self.offset
    }

    fn set_pos(&mut self, pos: usize) -> Result<usize, ProtocolError> {
        if pos > self.buf.len() {
            return Err(ProtocolError::BufferOverflow);
        }
        self.offset = pos;
        Ok(pos)
    }
}

impl<'a> NetReader<'a> for NetBufReader<'a> {
    fn read_u8(&mut self) -> Result<u8, ProtocolError> {
        let result = read_u8(self.buf, self.offset)?;
        self.offset += 1;
        Ok(result)
    }

    fn read_u16(&mut self) -> Result<u16, ProtocolError> {
        let result = read_u16(self.buf, self.offset)?;
        self.offset += 2;
        Ok(result)
    }

    fn read_bytes(&mut self) -> Result<&'a [u8], ProtocolError> {
        let len = self.read_u16()? as usize;
        let offset = self.offset;
        check_bounds(offset + len, self.buf.len())?;
        let result = &self.buf[offset..offset + len];
        self.offset += len;
        Ok(result)
    }

    fn available(&self) -> usize {
        self.buf.len().saturating_sub(self.offset)
    }
}

#[inline(always)]
pub fn write_u8(buf: &mut [u8], offset: usize, value: u8) -> Result<(), ProtocolError> {
    if buf.len() < offset + 2 {
        return Err(ProtocolError::BufferOverflow);
    }
    buf[offset..offset + 1].copy_from_slice(&value.to_be_bytes());
    Ok(())
}

#[inline(always)]
pub fn write_u16(buf: &mut [u8], offset: usize, value: u16) -> Result<(), ProtocolError> {
    if buf.len() < offset + 2 {
        return Err(ProtocolError::BufferOverflow);
    }
    buf[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    Ok(())
}

#[inline(always)]
pub fn read_u8(buf: &[u8], offset: usize) -> Result<u8, ProtocolError> {
    check_bounds(offset + 1, buf.len())?;
    Ok(u8::from_be_bytes(buf[offset..offset + 1].try_into()?))
}

#[inline(always)]
pub fn read_u16(buf: &[u8], offset: usize) -> Result<u16, ProtocolError> {
    check_bounds(offset + 2, buf.len())?;
    Ok(u16::from_be_bytes(buf[offset..offset + 2].try_into()?))
}

///
/// Tries to append data to provided [buf].
/// In case of overflow rolls back to initial vector length and returns [Ok(false)]
///
pub fn try_write<H>(buf: &mut Vec<u8>, mut handler: H) -> Result<bool, ProtocolError>
where
    H: FnMut(&mut NetBufWriter) -> Result<(), ProtocolError>,
{
    let mark = buf.len();
    buf.resize(NET_BUF_SIZE, 0);
    let mut writer = NetBufWriter::new(buf.as_mut_slice());
    writer.set_pos(mark).expect("Unable to set mark!");
    let r = handler(&mut writer);
    let size = writer.pos();
    match r {
        Ok(_) => {
            buf.truncate(size);
            return Ok(true);
        }
        Err(e) => {
            buf.truncate(mark);
            if e == ProtocolError::BufferOverflow {
                Ok(false)
            } else {
                Err(e)
            }
        }
    }
}

pub fn process_buf<'a, H, R>(reader: &mut R, mut handler: H) -> Result<(), ProtocolError>
where
    R: NetReader<'a>,
    H: FnMut(&Header, &mut R) -> bool,
{
    while reader.available() >= MIN_HEADER_SIZE {
        match read_header(reader) {
            Ok(header) => {
                let amount = header.size as usize;
                let mark = reader.pos();
                if !handler(&header, reader) {
                    if let Err(e) = reader.set_pos(mark + amount) {
                        error!("Failed to skip packet: {e:?}");
                    }
                }
            }
            Err(e) => {
                error!("Failed to read packet: {e:?}");
                break;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::net_rw::{NetBufReader, NetBufWriter, NetReader, NetWriter, WithPosition};

    #[test]
    fn test_net_writer() {
        let mut v = vec![1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8];
        {
            let mut writer = NetBufWriter::new(v.as_mut_slice());
            assert_eq!(0, writer.pos());
            writer.write_u16(321).unwrap();
            assert_eq!(2, writer.pos());
            writer.write_u16(543).unwrap();
            assert_eq!(4, writer.pos());
            writer.write_u8(222).unwrap();
            assert_eq!(
                writer.buf(),
                &[1, 65, 2, 31, 222, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8]
            );
            assert_eq!(5, writer.pos());
            assert_eq!(0, writer.set_pos(0).unwrap());
            writer.write_u16(11222).unwrap();
            assert_eq!(2, writer.pos());
            writer.write_bytes(&[1, 1, 1, 1, 1]).unwrap();
            assert_eq!(9, writer.pos());
            writer.write_str("test").unwrap();
            assert_eq!(
                writer.buf(),
                &[
                    43, 214, 0, 5, 1, 1, 1, 1, 1, 0, 4, 't' as u8, 'e' as u8, 's' as u8, 't' as u8,
                    8
                ]
            );
        }
        let mut reader = NetBufReader::new(v.as_slice());
        assert_eq!(11222, reader.read_u16().unwrap());
        assert_eq!(&[1, 1, 1, 1, 1], reader.read_bytes().unwrap());
        assert_eq!("test", reader.read_str().unwrap());
    }
}
