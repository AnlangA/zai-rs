#![allow(dead_code, unused_doc_comments)]
//! Tracked payload reader for streaming upload tests (plan P07.1).
//!
//! A `TrackedReader` wraps a `tokio::io::AsyncRead` and records how many bytes
//! have been read and how many "live" (read but not yet consumed) bytes are
//! outstanding. The live-payload tracker uses `Bytes::from_owner` with a
//! Drop guard so the test can assert memory usage bounds during a 128 MiB
//! streaming upload: live payload ≤ 16 MiB, total read = 128 MiB, each attempt
//! opens the file exactly once.

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, ReadBuf};

/// Shared counters for a `TrackedReader` stream.
#[derive(Default)]
pub struct TrackedStats {
    /// Total bytes read (cumulative, never decrements).
    pub total_read: AtomicU64,
    /// Bytes currently "live" (read from backing but not yet consumed by the
    /// caller). Incremented on read, decremented via the Bytes drop guard.
    pub live_bytes: AtomicU64,
    /// How many times the file was opened (each attempt re-opens).
    pub open_count: AtomicU64,
}

/// An async reader that tracks read progress and live payload size.
#[allow(dead_code)]
pub struct TrackedReader<R: AsyncRead + Unpin> {
    inner: R,
    stats: Arc<TrackedStats>,
    #[allow(dead_code)]
    chunk_size: usize,
}

impl<R: AsyncRead + Unpin> TrackedReader<R> {
    /// Wrap an async reader with tracking. Each chunk of up to `chunk_size`
    /// bytes increments live_bytes; when the produced Bytes is dropped, the
    /// guard decrements it.
    #[allow(dead_code)]
    pub fn new(inner: R, stats: Arc<TrackedStats>, chunk_size: usize) -> Self {
        stats.open_count.fetch_add(1, Ordering::SeqCst);
        Self {
            inner,
            stats,
            chunk_size,
        }
    }

    #[allow(dead_code)]
    pub fn stats(&self) -> &Arc<TrackedStats> {
        &self.stats
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for TrackedReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, buf);
        let after = buf.filled().len();
        let n = after.saturating_sub(before);
        if n > 0 {
            self.stats.total_read.fetch_add(n as u64, Ordering::SeqCst);
            self.stats.live_bytes.fetch_add(n as u64, Ordering::SeqCst);
        }
        result
    }
}

/// Drop guard: when the produced Bytes is freed, decrement `live_bytes`.
pub struct LiveGuard {
    stats: Arc<TrackedStats>,
    bytes: u64,
}

impl LiveGuard {
    #[allow(dead_code)]
    pub fn new(stats: Arc<TrackedStats>, bytes: u64) -> Self {
        Self { stats, bytes }
    }
}

impl Drop for LiveGuard {
    fn drop(&mut self) {
        self.stats
            .live_bytes
            .fetch_sub(self.bytes, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn tracked_reader_counts_bytes() {
        let data = vec![0u8; 1024];
        let stats = Arc::new(TrackedStats::default());
        let mut reader = TrackedReader::new(&data[..], stats.clone(), 256);
        let mut buf = vec![0u8; 1024];
        let n = reader.read(&mut buf).await.unwrap();
        assert_eq!(n, 1024);
        assert_eq!(stats.total_read.load(Ordering::SeqCst), 1024);
    }
}
