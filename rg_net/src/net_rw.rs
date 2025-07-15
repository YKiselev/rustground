use byteorder::{ByteOrder, LittleEndian};
use musli_zerocopy::{Endian, OwnedBuf, ZeroCopy};

use crate::protocol::{Header, PacketKind, ProtocolError};

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
    ///
    /// Bytes layout is 2 bytes prefix (data length) (u16) and then data itself
    /// Of course you can't write more than u16::MAX bytes.
    ///
    fn write_bytes(&mut self, value: &[u8]) -> Result<(), ProtocolError>;
    ///
    /// String length is limited to u16::MAX
    ///
    fn write_str(&mut self, value: &str) -> Result<(), ProtocolError>;
}

pub trait NetReader: WithPosition {
    fn read_u8(&mut self) -> Result<u8, ProtocolError>;
    fn read_u16(&mut self) -> Result<u16, ProtocolError>;
    ///
    /// Bytes layout is 2 bytes prefix (data length) (u16) and then data itself
    /// Of course you can't write more than u16::MAX bytes.
    ///
    fn read_bytes(&mut self) -> Result<&[u8], ProtocolError>;
    ///
    /// String length is limited to u16::MAX
    ///
    fn read_str(&mut self) -> Result<&str, ProtocolError>;
}

pub struct NetBufWriter<'a> {
    buf: &'a mut [u8],
    offset: usize,
}

impl<'a> NetBufWriter<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self {
            buf: buf,
            offset: 0,
        }
    }

    fn buf(&mut self) -> &mut [u8] {
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

    fn write_str(&mut self, value: &str) -> Result<(), ProtocolError> {
        self.write_bytes(value.as_bytes())
    }
}

pub struct NetBufReader<'a> {
    buf: &'a [u8],
    offset: usize,
}

impl<'a> NetBufReader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self {
            buf: buf,
            offset: 0,
        }
    }

    fn buf(&self) -> &[u8] {
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

impl NetReader for NetBufReader<'_> {
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

    fn read_bytes(&mut self) -> Result<&[u8], ProtocolError> {
        let len = self.read_u16()? as usize;
        let offset = self.offset;
        let result = &self.buf[offset..offset + len];
        self.offset += len;
        Ok(result)
    }

    fn read_str(&mut self) -> Result<&str, ProtocolError> {
        self.read_bytes()
            .and_then(|bytes| std::str::from_utf8(bytes).map_err(|_| ProtocolError::BadString))
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
    if buf.len() < offset + 1 {
        return Err(ProtocolError::BufferUnderflow);
    }
    Ok(u8::from_be_bytes(buf[offset..offset + 1].try_into()?))
}

#[inline(always)]
pub fn read_u16(buf: &[u8], offset: usize) -> Result<u16, ProtocolError> {
    if buf.len() < offset + 2 {
        return Err(ProtocolError::BufferUnderflow);
    }
    Ok(u16::from_be_bytes(buf[offset..offset + 2].try_into()?))
}

#[cfg(test)]
mod tests {
    use musli_zerocopy::{buf, endian, Endian, OwnedBuf, Ref, ZeroCopy};

    use crate::{
        net_rw::{padding, WithPosition},
        protocol::{Header, PacketKind},
    };

    use super::{NetBufReader, NetBufWriter, NetReader, NetWriter};

    #[derive(ZeroCopy, Debug)]
    #[repr(C)]
    struct MusliPacket {
        name: Ref<str, endian::Big>,
        map: Ref<str, endian::Big>,
        players: Endian<u32, endian::Big>,
        max_players: Endian<u32, endian::Big>,
        time: Endian<u64, endian::Big>,
        version: u8,
    }

    #[test]
    fn test_musli() {
        let mut buf = OwnedBuf::new().with_byte_order::<endian::Big>();
        let version = 123;
        let name = "Super-Duper-Server";
        let map = "rabid-frog";
        let players = 400;
        let max_players = 300000;
        let time = 12345678901234567890u64;
        let alignment = dbg!(std::mem::align_of::<MusliPacket>());
        for i in 1..=3 {
            //buf.clear();
            {
                let header_ref = buf.store_uninit::<Header>();
                let packet_ref = buf.store_uninit::<MusliPacket>();
                println!(
                    "header at {}, packet at {}",
                    header_ref.offset(),
                    packet_ref.offset()
                );
                let packet = MusliPacket {
                    version: version,
                    name: buf.store_unsized(&name),
                    map: buf.store_unsized(&map),
                    players: Endian::new(players),
                    max_players: Endian::new(max_players),
                    time: Endian::new(time),
                };
                buf.load_uninit_mut(packet_ref).write(&packet);
                let size = buf.len() - packet_ref.offset();
                let header = Header {
                    size: Endian::new(size as u16),
                    kind: PacketKind::ServerInfo,
                };
                buf.load_uninit_mut(header_ref).write(&header);
                println!("Written {} bytes, buf.len()={}", size, buf.len());
            }
            if true {
                let slice = buf.as_slice();
                println!("Slice lenght is {}", slice.len());
                let buf = buf::aligned_buf::<Header>(slice);
                let mut offset = 0usize;
                while offset + 4 < slice.len() {
                    let header = buf.load_at::<Header>(offset).unwrap();
                    assert_eq!(PacketKind::ServerInfo, header.kind);
                    offset += 4;
                    let padding = padding(offset, alignment);
                    offset += padding;
                    let size = header.size.to_ne() as usize;
                    println!("offset={}, padding={}, size={}", offset, padding, size);
                    let decoded = MusliPacket::from_bytes(&slice[offset..offset + size]).unwrap();
                    assert_eq!(version, decoded.version);
                    assert_eq!(name, buf.load(decoded.name).unwrap());
                    assert_eq!(map, buf.load(decoded.map).unwrap());
                    assert_eq!(players, decoded.players.to_ne());
                    assert_eq!(max_players, decoded.max_players.to_ne());
                    assert_eq!(time, decoded.time.to_ne());
                    offset += size;
                }
            }
        }
    }

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
