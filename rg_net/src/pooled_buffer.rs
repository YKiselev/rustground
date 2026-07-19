use std::str::FromStr;

use log::{error, info};

///
///
///
#[derive(Debug)]
pub struct PooledBuffer {
    data: Vec<u8>,
}

impl PooledBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn resize(&mut self, size: usize, value: u8) {
        self.data.resize(size, value);
    }

    pub fn truncate(&mut self, size: usize) {
        self.data.truncate(size);
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

pub struct BufferPool {
    name: String,
    buf_capacity: usize,
    pool: Vec<PooledBuffer>,
    create_count: u64,
    aquire_count: u64,
    release_count: u64,
}

impl BufferPool {
    pub fn new(buf_capacity: usize, name: &str) -> Self {
        Self {
            name: name.to_string(),
            buf_capacity,
            pool: Vec::new(),
            create_count: 0,
            aquire_count: 0,
            release_count: 0,
        }
    }

    pub fn aquire_buffer(&mut self) -> PooledBuffer {
        error!(
            "[{}] created: {}, aquired: {}, released: {}",
            self.name, self.create_count, self.aquire_count, self.release_count
        );
        if let Some(mut buf) = self.pool.pop() {
            buf.truncate(0);
            self.aquire_count += 1;
            return buf;
        }
        self.create_count += 1;
        PooledBuffer::new(self.buf_capacity)
    }

    pub fn release_buffer(&mut self, buf: PooledBuffer) {
        self.pool.push(buf);
        self.release_count += 1;
    }
}
