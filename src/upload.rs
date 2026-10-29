//! Count source reads without buffering or claiming network delivery.
use std::{
    io::{self, Read},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

pub struct Counted<R> {
    pub reader: R,
    pub count: Arc<AtomicU64>,
}
impl<R: Read> Read for Counted<R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let n = self.reader.read(bytes)?;
        self.count.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_actual_reads_and_preserves_errors() {
        let count = Arc::new(AtomicU64::new(0));
        let mut reader = Counted {
            reader: &b"abc"[..],
            count: count.clone(),
        };
        let mut byte = [0];
        assert_eq!(count.load(Ordering::Relaxed), 0);
        for expected in 1..=3 {
            assert_eq!(reader.read(&mut byte).unwrap(), 1);
            assert_eq!(count.load(Ordering::Relaxed), expected);
        }
        assert_eq!(reader.read(&mut byte).unwrap(), 0);
        struct Failed;
        impl Read for Failed {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::ErrorKind::Interrupted.into())
            }
        }
        let mut failed = Counted {
            reader: Failed,
            count: count.clone(),
        };
        assert_eq!(
            failed.read(&mut byte).unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
        assert_eq!(count.load(Ordering::Relaxed), 3);
    }
}
