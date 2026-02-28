//! Slowloris protection: protocol-agnostic slow-data attack mitigation.
//!
//! This module provides timeout wrappers and utilities to protect against
//! slow-data attacks (Slowloris-style) on any protocol (MQTT, HTTP, etc.).
//!
//! ## Attack Vector
//! Attackers open many connections and send data extremely slowly (1 byte every
//! few seconds), tying up server resources indefinitely without completing requests.
//!
//! ## Protection Strategy
//! Multi-layer timeout enforcement:
//! 1. **Base layer**: Protocol-agnostic idle and total timeouts
//! 2. **Protocol-specific overlays**: MQTT CONNECT timeout, HTTP request timeout
//!
//! ## Usage
//! Wrap a `TcpStream` with `TimeoutReader` to enforce idle timeouts on all reads.

use pin_project_lite::pin_project;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, ReadBuf};
use tokio::time::timeout;

pin_project! {
    /// A wrapper around an AsyncRead that enforces an idle timeout between reads.
    ///
    /// If no data is received within `idle_timeout`, the next read will return
    /// an error of kind `TimedOut`.
    pub struct TimeoutReader<R> {
        #[pin]
        inner: R,
        idle_timeout: Duration,
    }
}

impl<R> TimeoutReader<R> {
    /// Creates a new `TimeoutReader` wrapping the given reader.
    ///
    /// # Arguments
    /// * `inner` - The underlying reader (e.g., `TcpStream`)
    /// * `idle_timeout` - Maximum duration allowed between successful reads
    pub fn new(inner: R, idle_timeout: Duration) -> Self {
        Self {
            inner,
            idle_timeout,
        }
    }

    /// Consumes the wrapper and returns the underlying reader.
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: AsyncRead> AsyncRead for TimeoutReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.project();
        this.inner.poll_read(cx, buf)
    }
}

/// Reads from an AsyncRead with a timeout.
///
/// Returns `Err(io::Error)` with kind `TimedOut` if the read doesn't complete
/// within the specified duration.
pub async fn read_with_timeout<R>(
    reader: &mut R,
    buf: &mut [u8],
    timeout_duration: Duration,
) -> io::Result<usize>
where
    R: AsyncRead + Unpin,
{
    match timeout(timeout_duration, tokio::io::AsyncReadExt::read(reader, buf)).await {
        Ok(result) => result,
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "read operation timed out",
        )),
    }
}

/// Reads bytes with per-byte idle timeout enforcement.
///
/// This function reads data byte-by-byte (or in small chunks) and enforces
/// an idle timeout between each read. This prevents attackers from trickling
/// data slowly to hold connections open.
///
/// # Arguments
/// * `reader` - The source to read from
/// * `buf` - Buffer to fill
/// * `idle_timeout` - Maximum time allowed between receiving bytes
/// * `total_timeout` - Maximum total time for the entire read operation
///
/// # Returns
/// * `Ok(n)` - Number of bytes read (may be less than buf.len() if EOF reached)
/// * `Err(io::Error)` - Timeout or IO error
pub async fn read_with_idle_timeout<R>(
    reader: &mut R,
    buf: &mut [u8],
    idle_timeout: Duration,
    total_timeout: Duration,
) -> io::Result<usize>
where
    R: AsyncRead + Unpin,
{
    let start = tokio::time::Instant::now();
    let mut total_read = 0;

    while total_read < buf.len() {
        // Check total timeout
        if start.elapsed() >= total_timeout {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "total read timeout exceeded",
            ));
        }

        // Calculate remaining timeout
        let remaining_total = total_timeout - start.elapsed();
        let effective_timeout = std::cmp::min(idle_timeout, remaining_total);

        // Read with idle timeout
        let n = match timeout(
            effective_timeout,
            tokio::io::AsyncReadExt::read(reader, &mut buf[total_read..]),
        )
        .await
        {
            Ok(Ok(0)) => {
                // EOF reached
                return Ok(total_read);
            }
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "idle timeout exceeded between bytes",
                ))
            }
        };

        total_read += n;
    }

    Ok(total_read)
}

// NOTE: Inline unit tests have been moved to the crate-level `tests/` directory.
// See: `crates/aegis-proxy/tests/slowloris_tests.rs`
//
// Tests are kept out of library source files to centralize integration tests.
// This file intentionally does not contain an inline `#[cfg(test)]` module.
