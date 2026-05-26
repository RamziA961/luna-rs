use std::io;

/// Wraps any reader and logs the number of bytes read to Prometheus.
pub struct InstrumentedReader<R: io::Read, F: FnMut(usize)> {
    inner: R,
    on_read: F,
}

impl<R: io::Read, F: FnMut(usize)> InstrumentedReader<R, F> {
    pub fn new(inner: R, on_read: F) -> Self {
        Self { inner, on_read }
    }
}

impl<R: io::Read, F: FnMut(usize)> io::Read for InstrumentedReader<R, F> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n > 0 {
            (self.on_read)(n);
        }
        Ok(n)
    }
}
