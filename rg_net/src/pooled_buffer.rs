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

#[derive(Default)]
pub struct BufferPool {
    buf_capacity: usize,
    pool: Vec<PooledBuffer>,
}

impl BufferPool {
    pub fn new(buf_capacity: usize) -> Self {
        Self {
            buf_capacity,
            pool: Vec::new(),
        }
    }

    pub fn aquire_buffer(&mut self) -> PooledBuffer {
        if let Some(mut buf) = self.pool.pop() {
            buf.truncate(0);
            return buf;
        }
        PooledBuffer::new(self.buf_capacity)
    }

    pub fn release_buffer(&mut self, buf: PooledBuffer) {
        self.pool.push(buf);
    }
}
