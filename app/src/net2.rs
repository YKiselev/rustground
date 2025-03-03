use snafu::Snafu;

#[derive(Debug, Snafu)]
enum Net2Error {
    #[snafu(display("Buffer underflow"))]
    BufferUnderflow,
    #[snafu(display("Buffer overflow"))]
    BufferOverflow,
}

struct HeaderAdapter<'a> {
    buf: &'a [u8],
}

impl HeaderAdapter<'_> {
    fn new<'a>(buf: &'a [u8]) -> Self {
        unimplemented!()
    }
}

macro_rules! define_write {
    ($t:ty) => {
        ::paste::paste! {
            #[inline]
            pub(super) fn [<write_ $t>] (buf: &mut [u8], offset: usize, value: $t) -> Result<usize, Net2Error> {
                let size = std::mem::size_of::<$t>();
                let new_offset = offset + size;
                if new_offset > buf.len() {
                    Err(Net2Error::BufferOverflow)
                } else {
                    let mut pos = offset;
                    let mut shift = (size - 1) * 8;
                    while pos < new_offset {
                        buf[pos] = (value >> shift) as u8;
                        pos += 1;
                        shift = shift.saturating_sub(8);
                    }
                    Ok(new_offset)
                }
            }
        }
    };
}

macro_rules! define_read {
    ($t:ty) => {
        ::paste::paste! {
            #[inline]
            pub(super) fn [<read_ $t>](buf: &[u8], offset: usize) -> Result<$t, Net2Error> {
                let size = std::mem::size_of::<$t>();
                if offset + size > buf.len() {
                    Err(Net2Error::BufferUnderflow)
                } else {
                    let mut result = 0;
                    let mut shift = (size - 1) * 8;
                    for i in 0..size {
                        result += (buf[offset + i] as $t) << shift;
                        shift = shift.saturating_sub(8);
                    }
                    Ok(result)
                }
            }
        }
    };
}

macro_rules! define_zz {
    ($t:ty, $r:ty) => {
        ::paste::paste! {
            #[inline]
            pub(crate) fn [<zz_encode_ $t>] (value: $t) -> $t {
                (value >> ($t::BITS - 1)) ^ (value << 1)
            }

            #[inline]
            pub(crate) fn [<zz_decode_ $t>] (value: $t) -> $t {
                (value >> 1) ^ -(value & 1)
            }
        }
    };
}

//#[allow(dead_code)]
mod _private {
    use super::Net2Error;

    define_write!(u8);
    define_write!(u16);
    define_write!(u32);
    define_write!(u64);
    define_write!(i8);
    define_write!(i16);
    define_write!(i32);
    define_write!(i64);

    define_read!(u8);
    define_read!(u16);
    define_read!(u32);
    define_read!(u64);
    define_read!(i8);
    define_read!(i16);
    define_read!(i32);
    define_read!(i64);

    define_zz!(i8, u8);
    define_zz!(i16, u16);
    define_zz!(i32, u32);
    define_zz!(i64, u64);
}

#[cfg(test)]
mod tests {

    use std::{
        i32, io::{BufRead, Cursor, Read, Seek, Write}, ops::Deref, str::FromStr, u32
    };

    use byteorder::{BigEndian, ByteOrder, ReadBytesExt};
    use rsa::rand_core::le;

    use crate::net2::_private::*;

    use super::Net2Error;

    #[test]
    fn test_u16() {
        let buf = &mut [0u8; 8];

        let offset = write_u16(buf, 0, 0xaabb).unwrap();
        assert_eq!(2, offset);
        assert_eq!(&buf[..2], &[0xaau8, 0xbbu8]);

        assert_eq!(0xaabb, read_u16(buf, 0).unwrap());
    }

    #[test]
    fn test_i16() {
        let buf = &mut [0u8; 8];

        write_i16(buf, 0, i16::MAX).unwrap();
        write_i16(buf, 2, i16::MIN).unwrap();

        assert_eq!(i16::MAX, read_i16(buf, 0).unwrap());
        assert_eq!(i16::MIN, read_i16(buf, 2).unwrap());
    }

    #[test]
    fn test_u32() {
        #[cfg(target_endian = "big")]
        {}
        #[cfg(not(target_endian = "big"))]
        {}

        let buf = &mut [0u8; 8];

        let offset = write_u32(buf, 0, 0xaabbccdd).unwrap();
        assert_eq!(4, offset);
        assert_eq!(&buf[..4], &[0xaau8, 0xbbu8, 0xccu8, 0xddu8]);

        assert_eq!(0xaabbccdd, read_u32(buf, 0).unwrap());
    }

    #[test]
    fn test_i32() {
        let buf = &mut [0u8; 8];

        write_i32(buf, 0, i32::MAX).unwrap();
        write_i32(buf, 4, i32::MIN).unwrap();

        assert_eq!(i32::MAX, read_i32(buf, 0).unwrap());
        assert_eq!(i32::MIN, read_i32(buf, 4).unwrap());
    }

    #[test]
    fn test_u64() {
        let buf = &mut [0u8; 8];

        let offset = write_u64(buf, 0, 0xaabbccddabacadaf).unwrap();
        assert_eq!(8, offset);
        assert_eq!(
            &buf[..8],
            &[0xaau8, 0xbbu8, 0xccu8, 0xddu8, 0xab, 0xac, 0xad, 0xaf]
        );

        assert_eq!(0xaabbccddabacadaf, read_u64(buf, 0).unwrap());
    }

    pub trait WriteString: std::io::Write {
        fn write_unsized<S: AsRef<str>>(&mut self, value: S) -> std::io::Result<usize> {
            let view = value.as_ref().as_bytes();
            let length = dbg!(view.len() as u16);
            self.write(&length.to_be_bytes())?;
            self.write(view)
        }
    }

    impl<W: std::io::Write + ?Sized> WriteString for W {}

    pub trait ReadString: std::io::Read {
        fn read_unsized(&mut self) -> std::io::Result<String> {
            let mut buf = [0; 2];
            self.read_exact(&mut buf)?;
            let length = dbg!(u16::from_be_bytes(buf) as usize);
            let mut result = Vec::with_capacity(length);
            result.resize(length, 0);
            self.read_exact(result.as_mut_slice())?;
            Ok(String::from_utf8_lossy(result.as_slice()).into_owned())
        }
    }

    impl<R: std::io::Read + ?Sized> ReadString for R {}

    #[test]
    fn should_zz_encode_decode() {
        assert_eq!(0, zz_decode_i8(dbg!(zz_encode_i8(0))));
        assert_eq!(1, zz_decode_i8(dbg!(zz_encode_i8(1))));
        assert_eq!(-1, zz_decode_i8(dbg!(zz_encode_i8(-1))));
        assert_eq!(63, zz_decode_i8(dbg!(zz_encode_i8(63))));
        assert_eq!(-63, zz_decode_i8(dbg!(zz_encode_i8(-63))));
        assert_eq!(
            i32::MAX / 2,
            zz_decode_i32(dbg!(zz_encode_i32(i32::MAX / 2)))
        );
        assert_eq!(
            i32::MIN / 2,
            zz_decode_i32(dbg!(zz_encode_i32(i32::MIN / 2)))
        );
        // assert_eq!(i32::MAX, zz_decode_i32(dbg!(zz_encode_i32(dbg!(i32::MAX)))));
        // assert_eq!(i32::MIN, zz_decode_i32(dbg!(zz_encode_i32(dbg!(i32::MIN)))));
        // assert_eq!(i32::MAX, zz_decode_i32(dbg!(zz_encode_i32(dbg!(i32::MAX)))));
        // assert_eq!(i32::MIN, zz_decode_i32(dbg!(zz_encode_i32(dbg!(i32::MIN)))));
    }

    fn read_unsized<T : AsRef<[u8]>>(c: &mut Cursor<T>) -> std::io::Result<std::borrow::Cow<'_,str>> {
        let mut buf = [0; 2];
        c.read_exact(&mut buf)?;
        let length = dbg!(u16::from_be_bytes(buf));
        let pos = c.position() as usize;
        c.seek_relative(length as i64)?;
        Ok(String::from_utf8_lossy(&c.get_ref().as_ref()[pos..pos + length as usize]))
    }

    #[test]
    fn test_byteorder() {
        use byteorder::{ReadBytesExt, WriteBytesExt};
        use WriteString;

        let mut buf = [0u8; 64];
        let mut w = Cursor::new(buf.as_mut_slice());

        w.write_u8(123).unwrap();
        w.write_u16::<BigEndian>(12345).unwrap();
        w.write_u32::<BigEndian>(123456789).unwrap();
        let abcd = "ABCD";
        w.write_unsized(abcd).unwrap();
        w.write_unsized("12345").unwrap();

        let bytes = w.position() as usize;
        let mut c = Cursor::new(&buf[..bytes]);

        assert_eq!(123u8, c.read_u8().unwrap());
        assert_eq!(12345u16, c.read_u16::<BigEndian>().unwrap());
        assert_eq!(123456789u32, c.read_u32::<BigEndian>().unwrap());
        assert_eq!("ABCD", c.read_unsized().unwrap());
        assert_eq!("12345", read_unsized(&mut c).unwrap());
    }
}
