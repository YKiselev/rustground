use snafu::Snafu;

//#[allow(dead_code)]
mod _private {
    
}

#[cfg(test)]
mod tests {

    use std::{
        array::TryFromSliceError,
        i32,
        io::{BufRead, Cursor, Error, Read, Seek, Write},
        ops::Deref,
        str::FromStr,
        u32,
    };

    use byteorder::{BigEndian, ByteOrder, ReadBytesExt};
    use cookie_factory::GenError;
    use musli_zerocopy::buf;
    use nom::{bytes::take, error::ErrorKind, number::be_u32, Err, Parser};
    use snafu::Snafu;

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

    fn read_unsized<T: AsRef<[u8]>>(
        c: &mut Cursor<T>,
    ) -> std::io::Result<std::borrow::Cow<'_, str>> {
        let mut buf = [0; 2];
        c.read_exact(&mut buf)?;
        let length = dbg!(u16::from_be_bytes(buf));
        let pos = c.position() as usize;
        c.seek_relative(length as i64)?;
        Ok(String::from_utf8_lossy(
            &c.get_ref().as_ref()[pos..pos + length as usize],
        ))
    }

    #[derive(Debug, Snafu)]
    enum SliceError {
        #[snafu(display("Buffer overflow"))]
        BufferOverflow,
        #[snafu(display("Buffer underflow"))]
        BufferUnderflow,
    }

    struct Slice<T> {
        pub data: T,
    }

    impl<T> Slice<T> {
        pub fn new(data: T) -> Self {
            Self { data }
        }
    }

    impl<T> Slice<T>
    where
        T: AsRef<[u8]>,
    {
        fn buf<const N: usize>(self: &Self, offset: usize) -> Result<[u8; N], SliceError> {
            let buf = self.data.as_ref();
            if buf.len() >= offset + N {
                buf[offset..offset + N]
                    .try_into()
                    .map_err(|_| SliceError::BufferUnderflow)
            } else {
                Err(SliceError::BufferUnderflow)
            }
        }

        pub fn read_u8(self: &Self, offset: usize) -> Result<u8, SliceError> {
            self.buf::<1>(offset).map(|buf| buf[0])
        }

        pub fn read_u16(self: &Self, offset: usize) -> Result<u16, SliceError> {
            self.buf::<2>(offset).map(|buf| u16::from_be_bytes(buf))
        }

        pub fn read_u32(self: &Self, offset: usize) -> Result<u32, SliceError> {
            self.buf::<4>(offset).map(|buf| u32::from_be_bytes(buf))
        }

        pub fn read_u64(self: &Self, offset: usize) -> Result<u64, SliceError> {
            self.buf::<8>(offset).map(|buf| u64::from_be_bytes(buf))
        }
    }

    impl<T> Slice<T>
    where
        T: AsMut<[u8]>,
    {
        fn mut_buf<const N: usize>(
            self: &mut Self,
            offset: usize,
        ) -> Result<&mut [u8; N], SliceError> {
            let buf = self.data.as_mut();
            if buf.len() >= offset + N {
                buf[offset..offset + N]
                    .as_mut()
                    .try_into()
                    .map_err(|_| SliceError::BufferOverflow)
            } else {
                Err(SliceError::BufferOverflow)
            }
        }

        pub fn write_u8(self: &mut Self, offset: usize, value: u8) -> Result<(), SliceError> {
            self.mut_buf::<1>(offset).map(|buf| buf[0] = value)
        }

        pub fn write_u16(self: &mut Self, offset: usize, value: u16) -> Result<(), SliceError> {
            self.mut_buf::<2>(offset)
                .map(|buf| buf.copy_from_slice(&value.to_be_bytes()))
        }
    }

    #[test]
    fn test_slice() {
        use WriteString;

        let mut buf = [0u8; 64];
        let s = buf.as_mut_slice();
        dbg!(s.len());
        let mut w = Slice::new(s);

        w.write_u16(0, 1).unwrap();
        w.write_u16(2, 123).unwrap();
        w.write_u16(4, 2).unwrap();

        assert_eq!(123, w.read_u16(2).unwrap());
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

    #[test]
    fn test_nom_and_cookie_factory() {
        fn write_data(dest: &mut [u8]) -> Result<(), GenError>{
            use cookie_factory::{
                bytes::be_u32, combinator::string, gen, SerializeFn, WriteContext,
            };
        
            let s = "Hello, World!";
            let (rest, pos) = gen(be_u32(s.len() as u32), dest)?;
            let (rest, pos) = gen(string(s), rest)?;
            Ok(())
        }

        fn read_packet(src:&[u8]) -> Result<(), SliceError>{
            let (src, len) = be_u32::<_,(_,ErrorKind)>().parse(src)?;
            let (_, str) = take::<_, _, (_, ErrorKind)>(len).parse(src)?;
            let mut iter = str.utf8_chunks();
            let s = if let Some(v) = iter.next() {
                if v.invalid().is_empty() {
                    v.valid()
                } else {
                    ""
                }
            } else {
                ""
            };
            if s == "Hello, World!" {
            Ok(())
            } else {
                Err(SliceError::BufferUnderflow)
            }
        }

        let mut buf = [9u8; 200];
        {
            write_data(&mut buf).unwrap();
        }
        {
            read_packet(&buf).unwrap();
        }
    }
}
