use byteorder::{ByteOrder, LittleEndian};

use crate::protocol::{Hello, ProtocolError, Version};



impl Hello {
    pub fn new(buf: &mut [u8]) -> Self {
        unimplemented!()
    }

    pub fn version(&self) -> Result<Version, ProtocolError> {
        unimplemented!()
    }
}