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
    fn write_u32(&mut self, value: u32) -> Result<(), ProtocolError>;
    fn write_u64(&mut self, value: u64) -> Result<(), ProtocolError>;
    fn write_f32(&mut self, value: f32) -> Result<(), ProtocolError> {
        self.write_u32(value.to_bits())
    }

    fn write_f64(&mut self, value: f64) -> Result<(), ProtocolError> {
        self.write_u64(value.to_bits())
    }

    fn write_i8(&mut self, value: i8) -> Result<(), ProtocolError> {
        self.write_u8(value as u8)
    }
    fn write_i16(&mut self, value: i16) -> Result<(), ProtocolError> {
        self.write_u16(value as u16)
    }
    fn write_i32(&mut self, value: i32) -> Result<(), ProtocolError> {
        self.write_u32(value as u32)
    }
    fn write_i64(&mut self, value: i64) -> Result<(), ProtocolError> {
        self.write_u64(value as u64)
    }

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
    fn read_u32(&mut self) -> Result<u32, ProtocolError>;
    fn read_u64(&mut self) -> Result<u64, ProtocolError>;
    fn read_f32(&mut self) -> Result<f32, ProtocolError> {
        self.read_u32().map(|v| f32::from_bits(v))
    }
    fn read_f64(&mut self) -> Result<f64, ProtocolError> {
        self.read_u64().map(|v| f64::from_bits(v))
    }
    fn read_i8(&mut self) -> Result<i8, ProtocolError> {
        self.read_u8().map(|v| v as i8)
    }
    fn read_i16(&mut self) -> Result<i16, ProtocolError> {
        self.read_u16().map(|v| v as i16)
    }
    fn read_i32(&mut self) -> Result<i32, ProtocolError> {
        self.read_u32().map(|v| v as i32)
    }
    fn read_i64(&mut self) -> Result<i64, ProtocolError> {
        self.read_u64().map(|v| v as i64)
    }

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
        let v = self.read_u8()?;
        E::try_from_primitive(v).map_err(|e| ProtocolError::BadEnumTag { value: v as isize })
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

    fn write_u32(&mut self, value: u32) -> Result<(), ProtocolError> {
        write_u32(self.buf, self.offset, value)?;
        self.offset += 4;
        Ok(())
    }

    fn write_u64(&mut self, value: u64) -> Result<(), ProtocolError> {
        write_u64(self.buf, self.offset, value)?;
        self.offset += 8;
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

    fn read_u32(&mut self) -> Result<u32, ProtocolError> {
        let result = read_u32(self.buf, self.offset)?;
        self.offset += 4;
        Ok(result)
    }

    fn read_u64(&mut self) -> Result<u64, ProtocolError> {
        let result = read_u64(self.buf, self.offset)?;
        self.offset += 8;
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
pub fn write_u32(buf: &mut [u8], offset: usize, value: u32) -> Result<(), ProtocolError> {
    if buf.len() < offset + 4 {
        return Err(ProtocolError::BufferOverflow);
    }
    buf[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    Ok(())
}

#[inline(always)]
pub fn write_u64(buf: &mut [u8], offset: usize, value: u64) -> Result<(), ProtocolError> {
    if buf.len() < offset + 8 {
        return Err(ProtocolError::BufferOverflow);
    }
    buf[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
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

#[inline(always)]
pub fn read_u32(buf: &[u8], offset: usize) -> Result<u32, ProtocolError> {
    check_bounds(offset + 4, buf.len())?;
    Ok(u32::from_be_bytes(buf[offset..offset + 4].try_into()?))
}

#[inline(always)]
pub fn read_u64(buf: &[u8], offset: usize) -> Result<u64, ProtocolError> {
    check_bounds(offset + 8, buf.len())?;
    Ok(u64::from_be_bytes(buf[offset..offset + 8].try_into()?))
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
    fn unsigned_ints() {
        let v = &mut [0; 16];
        {
            let mut writer = NetBufWriter::new(v);
            assert_eq!(0, writer.pos());

            writer.write_u8(1u8).unwrap();
            writer.write_u16(1001u16).unwrap();
            writer.write_u32(500_005u32).unwrap();
            writer.write_u64(321_000_000_321u64).unwrap();

            assert_eq!(
                writer.buf(),
                &[1, 3, 233, 0, 7, 161, 37, 0, 0, 0, 74, 189, 23, 75, 65, 0]
            );
        }
        let mut reader = NetBufReader::new(v);
        assert_eq!(1u8, reader.read_u8().unwrap());
        assert_eq!(1001u16, reader.read_u16().unwrap());
        assert_eq!(500_005u32, reader.read_u32().unwrap());
        assert_eq!(321_000_000_321u64, reader.read_u64().unwrap());
    }

    #[test]
    fn signed_ints() {
        let v = &mut [0; 16];
        {
            let mut writer = NetBufWriter::new(v);
            writer.write_i8(-5i8).unwrap();
            writer.write_i16(-10_000i16).unwrap();
            writer.write_i32(-3i32).unwrap();
            writer.write_i64(-700_000i64).unwrap();

            assert_eq!(
                writer.buf(),
                &[251, 216, 240, 255, 255, 255, 253, 255, 255, 255, 255, 255, 245, 81, 160, 0]
            );
        }
        let mut reader = NetBufReader::new(v);
        assert_eq!(-5i8, reader.read_i8().unwrap());
        assert_eq!(-10_000i16, reader.read_i16().unwrap());
        assert_eq!(-3i32, reader.read_i32().unwrap());
        assert_eq!(-700_000i64, reader.read_i64().unwrap());
    }

    #[test]
    fn floating_point() {
        let v = &mut [0; 16];
        {
            let mut writer = NetBufWriter::new(v);
            writer.write_f32(3.1415f32).unwrap();
            writer.write_f64(0.00000456f64).unwrap();
            assert_eq!(
                writer.buf(),
                &[64, 73, 14, 86, 62, 211, 32, 67, 65, 115, 60, 228, 0, 0, 0, 0]
            );
        }
        let mut reader = NetBufReader::new(v);
        assert_eq!(3.1415f32, reader.read_f32().unwrap());
        assert_eq!(0.00000456f64, reader.read_f64().unwrap());
    }

    #[test]
    fn insized() {
        let v = &mut [0; 20];
        {
            let mut writer = NetBufWriter::new(v);
            writer.write_bytes(&[1, 0, 1, 0, 1]).unwrap();
            writer.write_str("Test string").unwrap();
            assert_eq!(
                writer.buf(),
                &[0, 5, 1, 0, 1, 0, 1, 0, 11, 84, 101, 115, 116, 32, 115, 116, 114, 105, 110, 103]
            );
        }
        let mut reader = NetBufReader::new(v);
        assert_eq!(&[1, 0, 1, 0, 1], reader.read_bytes().unwrap());
        assert_eq!("Test string", reader.read_str().unwrap());
    }
}
