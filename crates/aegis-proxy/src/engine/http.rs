//! HTTP inspection and parsing for protocol detection and Slowloris protection.
//!
//! This module provides HTTP request parsing with strict timeout and size
//! enforcement to detect and reject:
//! 1. HTTP traffic to an MQTT broker (protocol mismatch)
//! 2. Slowloris attacks (slow HTTP header transmission)
//!
//! ## HTTP Request Format
//! ```text
//! GET /path HTTP/1.1\r\n
//! Host: example.com\r\n
//! User-Agent: curl/7.68.0\r\n
//! \r\n
//! ```
//!
//! ## Detection Strategy
//! - Parse request line incrementally
//! - Parse headers one by one
//! - Enforce timeouts at each stage (request line, per-header, total)
//! - Enforce size limits (total headers, per-header, header count)
//! - Reject if any limit exceeded

use std::io;
use std::time::Duration;
use tokio::io::AsyncRead;
use tokio::time::timeout;

/// HTTP request methods we recognize
const HTTP_METHODS: &[&str] = &[
    "GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "PATCH", "CONNECT", "TRACE",
];

/// Maximum size of request line (method + URI + version)
const MAX_REQUEST_LINE_SIZE: usize = 8192;

/// Result of HTTP inspection
#[derive(Debug, PartialEq)]
pub enum HttpInspectionResult {
    /// Valid HTTP request detected (should be rejected - wrong protocol)
    HttpDetected,
    /// Not HTTP traffic
    NotHttp,
    /// Slowloris attack detected (timeout or size limit exceeded)
    SlowlorisDetected(String),
}

/// Parsed HTTP request line
#[derive(Debug, PartialEq)]
struct RequestLine {
    method: String,
    uri: String,
    version: String,
}

/// Header struct removed — it was unused. Kept out-of-band to avoid dead_code warning.

/// Inspects incoming data to detect HTTP protocol and Slowloris attacks.
///
/// # Arguments
/// * `reader` - The connection to inspect
/// * `request_timeout` - Total timeout for request line + all headers
/// * `idle_timeout` - Idle timeout between bytes
/// * `max_header_size` - Maximum total size of all headers
/// * `max_header_count` - Maximum number of headers
/// * `max_header_line_size` - Maximum size of a single header line
///
/// # Returns
/// * `HttpInspectionResult` indicating detection outcome
pub async fn inspect_http<R>(
    reader: &mut R,
    request_timeout: Duration,
    idle_timeout: Duration,
    max_header_size: usize,
    max_header_count: usize,
    max_header_line_size: usize,
) -> io::Result<HttpInspectionResult>
where
    R: AsyncRead + Unpin,
{
    // Try to parse with total timeout
    match timeout(
        request_timeout,
        parse_http_request(
            reader,
            idle_timeout,
            max_header_size,
            max_header_count,
            max_header_line_size,
        ),
    )
    .await
    {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(e),
        Err(_) => Ok(HttpInspectionResult::SlowlorisDetected(
            "total request timeout exceeded".to_string(),
        )),
    }
}

/// Parses HTTP request with size and timeout limits.
async fn parse_http_request<R>(
    reader: &mut R,
    idle_timeout: Duration,
    max_header_size: usize,
    max_header_count: usize,
    max_header_line_size: usize,
) -> io::Result<HttpInspectionResult>
where
    R: AsyncRead + Unpin,
{
    // Parse request line
    let _request_line = match parse_request_line(reader, idle_timeout).await? {
        Some(line) => line,
        None => return Ok(HttpInspectionResult::NotHttp),
    };

    // Parse headers
    let mut total_header_bytes = 0;
    let mut header_count = 0;

    loop {
        // Check header count limit
        if header_count >= max_header_count {
            return Ok(HttpInspectionResult::SlowlorisDetected(
                "max header count exceeded".to_string(),
            ));
        }

        // Parse next header line
        let line = match read_line_with_timeout(reader, idle_timeout, max_header_line_size).await? {
            Some(line) => line,
            None => {
                return Ok(HttpInspectionResult::SlowlorisDetected(
                    "incomplete headers (EOF)".to_string(),
                ))
            }
        };

        total_header_bytes += line.len() + 2; // +2 for \r\n

        // Check total header size
        if total_header_bytes > max_header_size {
            return Ok(HttpInspectionResult::SlowlorisDetected(
                "max header size exceeded".to_string(),
            ));
        }

        // Empty line indicates end of headers
        if line.is_empty() {
            break;
        }

        // Validate header format (must contain ':')
        if !line.contains(':') {
            return Ok(HttpInspectionResult::SlowlorisDetected(
                "malformed header line".to_string(),
            ));
        }

        header_count += 1;
    }

    // Valid HTTP request detected
    Ok(HttpInspectionResult::HttpDetected)
}

/// Parses HTTP request line (e.g., "GET /path HTTP/1.1")
///
/// Returns:
/// * `Some(RequestLine)` if valid HTTP request line detected
/// * `None` if not HTTP (doesn't start with known method)
async fn parse_request_line<R>(
    reader: &mut R,
    idle_timeout: Duration,
) -> io::Result<Option<RequestLine>>
where
    R: AsyncRead + Unpin,
{
    let line = match read_line_with_timeout(reader, idle_timeout, MAX_REQUEST_LINE_SIZE).await? {
        Some(line) => line,
        None => return Ok(None),
    };

    // Parse "METHOD URI VERSION"
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 3 {
        return Ok(None);
    }

    let method = parts[0];
    let uri = parts[1];
    let version = parts[2];

    // Check if method is valid HTTP method
    if !HTTP_METHODS.contains(&method) {
        return Ok(None);
    }

    // Check if version starts with "HTTP/"
    if !version.starts_with("HTTP/") {
        return Ok(None);
    }

    Ok(Some(RequestLine {
        method: method.to_string(),
        uri: uri.to_string(),
        version: version.to_string(),
    }))
}

/// Reads a line (terminated by \r\n) with timeout and size limit.
///
/// Returns:
/// * `Some(String)` - Line without \r\n terminator
/// * `None` - EOF reached before reading anything
async fn read_line_with_timeout<R>(
    reader: &mut R,
    idle_timeout: Duration,
    max_line_size: usize,
) -> io::Result<Option<String>>
where
    R: AsyncRead + Unpin,
{
    let mut line = Vec::new();
    let mut prev_byte = 0u8;

    loop {
        // Check size limit
        if line.len() >= max_line_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "line size limit exceeded",
            ));
        }

        // Read one byte with timeout
        let mut byte = [0u8; 1];
        let _n = match timeout(
            idle_timeout,
            tokio::io::AsyncReadExt::read(reader, &mut byte),
        )
        .await
        {
            Ok(Ok(0)) => {
                // EOF
                if line.is_empty() {
                    return Ok(None);
                } else {
                    // Incomplete line
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "incomplete line",
                    ));
                }
            }
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "idle timeout reading line",
                ))
            }
        };

        let current_byte = byte[0];

        // Check for \r\n
        if prev_byte == b'\r' && current_byte == b'\n' {
            // Remove the \r from line
            line.pop();
            break;
        }

        line.push(current_byte);
        prev_byte = current_byte;
    }

    String::from_utf8(line)
        .map(Some)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid UTF-8 in line"))
}

/// Quick check if first few bytes look like HTTP.
///
/// This is a fast pre-check before full parsing.
pub fn looks_like_http(buf: &[u8]) -> bool {
    if buf.len() < 4 {
        return false;
    }

    // Check for common HTTP methods
    for method in HTTP_METHODS {
        if buf.starts_with(method.as_bytes()) {
            // Next byte should be space
            if buf.len() > method.len() && buf[method.len()] == b' ' {
                return true;
            }
        }
    }

    false
}

// NOTE: Inline unit tests have been moved to the crate-level `tests/` directory.
// See: `crates/aegis-proxy/tests/http_tests.rs` (create this file and move the tests here).
// Keeping tests in the `tests/` directory ensures they run as integration tests and
// follows the project policy of centralizing test files.
