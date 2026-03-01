use crate::engine::http::{inspect_http, looks_like_http, HttpInspectionResult};
use crate::engine::slowloris::read_with_idle_timeout;
use crate::parser::mqtt::{self, MqttPacketType};
use aegis_common::SlowlorisConfig;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{tcp::OwnedWriteHalf, TcpStream};
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

pub static ACTIVE_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);

/// Configuration for connection handling behavior.
pub struct ConnectionConfig {
    pub mqtt_inspect: bool,
    pub mqtt_full_inspect: bool,
    pub http_inspect: bool,
    pub slowloris_protect: bool,
    pub max_connect_remaining: usize,
    pub slowloris_config: SlowlorisConfig,
}

struct ProxyConnectionGuard;

impl ProxyConnectionGuard {
    fn new() -> Self {
        ACTIVE_CONNECTIONS.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

impl Drop for ProxyConnectionGuard {
    fn drop(&mut self) {
        ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Read one byte (fixed header) from the client with timeout.
async fn read_fixed_header(
    source: &mut TcpStream,
) -> Result<u8, Box<dyn std::error::Error + Send + Sync>> {
    let mut fixed = [0u8; 1];
    match timeout(Duration::from_secs(3), source.read_exact(&mut fixed)).await {
        Ok(Ok(_)) => Ok(fixed[0]),
        Ok(Err(e)) => {
            crate::metrics::PROTOCOL_REJECTIONS.inc();
            Err(Box::new(e))
        }
        Err(_) => {
            crate::metrics::PROTOCOL_REJECTIONS.inc();
            Err("timeout reading fixed header".into())
        }
    }
}

/// Read Remaining Length bytes from the client, returning the bytes and decoded length.
/// The caller provides `max_allowed` to guard against excessively large Remaining Lengths
/// (prevents large allocations during CONNECT inspection).
async fn read_remaining_length(
    source: &mut TcpStream,
    max_allowed: usize,
) -> Result<(Vec<u8>, usize), Box<dyn std::error::Error + Send + Sync>> {
    let mut rl_bytes: Vec<u8> = Vec::with_capacity(4);
    for _ in 0..4 {
        let mut b = [0u8; 1];
        match timeout(Duration::from_secs(1), source.read_exact(&mut b)).await {
            Ok(Ok(_)) => {
                rl_bytes.push(b[0]);
                match mqtt::decode_remaining_length(&rl_bytes) {
                    Ok((v, _used)) => {
                        // Enforce maximum allowed remaining length for CONNECT inspection.
                        if v > max_allowed {
                            crate::metrics::PROTOCOL_REJECTIONS.inc();
                            warn!(
                                "Rejected CONNECT: remaining length {} exceeds max allowed {}",
                                v, max_allowed
                            );
                            return Err("remaining length too large".into());
                        }
                        return Ok((rl_bytes, v));
                    }
                    Err("Incomplete") => continue,
                    Err(_) => {
                        crate::metrics::PROTOCOL_REJECTIONS.inc();
                        return Err("malformed remaining length".into());
                    }
                }
            }
            Ok(Err(e)) => {
                crate::metrics::PROTOCOL_REJECTIONS.inc();
                return Err(Box::new(e));
            }
            Err(_) => {
                crate::metrics::PROTOCOL_REJECTIONS.inc();
                return Err("timeout reading remaining length".into());
            }
        }
    }
    crate::metrics::PROTOCOL_REJECTIONS.inc();
    Err("incomplete remaining length".into())
}

/// Read `len` bytes of payload from the client with timeout.
async fn read_payload(
    source: &mut TcpStream,
    len: usize,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    if len == 0 {
        return Ok(Vec::new());
    }
    let mut payload = vec![0u8; len];
    match timeout(Duration::from_secs(5), source.read_exact(&mut payload)).await {
        Ok(Ok(_)) => Ok(payload),
        Ok(Err(e)) => {
            crate::metrics::PROTOCOL_REJECTIONS.inc();
            Err(Box::new(e))
        }
        Err(_) => {
            crate::metrics::PROTOCOL_REJECTIONS.inc();
            Err("timeout reading payload".into())
        }
    }
}

/// Minimal CONNECT variable-header validation.
fn validate_connect_variable_header(payload: &[u8]) -> bool {
    payload.len() >= 6 && payload[0] == 0x00 && payload[1] == 0x04 && &payload[2..6] == b"MQTT"
}

/// Connect to backend broker with timeout.
async fn connect_backend(
    target_addr: &str,
    client_peer: &str,
) -> Result<TcpStream, Box<dyn std::error::Error + Send + Sync>> {
    debug!(
        "Attempting backend connect to {} for client {}",
        target_addr, client_peer
    );
    match timeout(Duration::from_secs(5), TcpStream::connect(target_addr)).await {
        Ok(stream) => {
            let s = stream?;
            debug!(
                "Successfully connected to backend {} for client {}",
                target_addr, client_peer
            );
            Ok(s)
        }
        Err(_) => {
            warn!(
                "Could not connect to backend at {} (connect timeout) for client {}",
                target_addr, client_peer
            );
            Err("backend connect timeout".into())
        }
    }
}

/// Forward initial bytes (already-consumed CONNECT frame) to backend.
async fn forward_initial_bytes(
    target_write: &mut OwnedWriteHalf,
    initial_bytes: &[u8],
    target_addr: &str,
    client_peer: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if initial_bytes.is_empty() {
        return Ok(());
    }
    let preview: String = initial_bytes
        .iter()
        .take(16)
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ");
    debug!(
        "Forwarding {} initial bytes to backend {} for client {} (preview: {})",
        initial_bytes.len(),
        target_addr,
        client_peer,
        preview
    );
    match timeout(
        Duration::from_secs(3),
        target_write.write_all(initial_bytes),
    )
    .await
    {
        Ok(Ok(_)) => {
            debug!(
                "Successfully forwarded {} initial bytes to backend {} for client {}",
                initial_bytes.len(),
                target_addr,
                client_peer
            );
            Ok(())
        }
        Ok(Err(e)) => {
            warn!(
                "Error writing initial CONNECT bytes to backend {} for client {}: {}",
                target_addr, client_peer, e
            );
            debug!(
                "Initial bytes length: {}, preview: {}",
                initial_bytes.len(),
                preview
            );
            Err(Box::new(e))
        }
        Err(_) => {
            warn!(
                "Timeout writing initial CONNECT bytes to backend {} for client {}",
                target_addr, client_peer
            );
            debug!(
                "Initial bytes length: {}, preview: {}",
                initial_bytes.len(),
                preview
            );
            Err("timeout writing initial bytes".into())
        }
    }
}

/// Handle a single client connection. Supports optional MQTT inspection (lightweight or full),
/// HTTP inspection, and Slowloris protection.
pub async fn handle_connection(
    mut source: TcpStream,
    target_addr: String,
    config: ConnectionConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client_peer = source
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "<unknown>".to_string());

    let mut initial_bytes: Vec<u8> = Vec::new();

    if config.slowloris_protect {
        let first_packet_timeout =
            Duration::from_millis(config.slowloris_config.first_packet_timeout_ms);
        let mut peek_buf = [0u8; 16];
        let n = match timeout(first_packet_timeout, source.peek(&mut peek_buf)).await {
            Ok(Ok(n)) if n > 0 => n,
            Ok(Ok(_)) => {
                warn!(client = %client_peer, "Connection closed before sending data");
                crate::metrics::SLOWLORIS_REJECTIONS.inc();
                return Ok(());
            }
            Ok(Err(e)) => {
                warn!(client = %client_peer, error = %e, "Error peeking first packet");
                crate::metrics::SLOWLORIS_REJECTIONS.inc();
                return Ok(());
            }
            Err(_) => {
                warn!(client = %client_peer, "First packet timeout - no data received within {}ms",
                    config.slowloris_config.first_packet_timeout_ms);
                crate::metrics::SLOWLORIS_REJECTIONS.inc();
                return Ok(());
            }
        };

        debug!(client = %client_peer, "Received first {} bytes within timeout", n);

        if config.http_inspect && looks_like_http(&peek_buf[..n]) {
            info!(client = %client_peer, "HTTP protocol detected - inspecting for Slowloris");

            let http_timeout =
                Duration::from_millis(config.slowloris_config.http_request_timeout_ms);
            let idle_timeout =
                Duration::from_millis(config.slowloris_config.packet_idle_timeout_ms);

            match inspect_http(
                &mut source,
                http_timeout,
                idle_timeout,
                config.slowloris_config.max_http_header_size,
                config.slowloris_config.max_http_header_count,
                8192,
            )
            .await
            {
                Ok(HttpInspectionResult::HttpDetected) => {
                    info!(client = %client_peer, "Valid HTTP request detected - rejecting (wrong protocol for MQTT broker)");
                    crate::metrics::HTTP_REJECTIONS.inc();
                    return Ok(());
                }
                Ok(HttpInspectionResult::SlowlorisDetected(reason)) => {
                    warn!(client = %client_peer, reason = %reason, "Slowloris attack detected on HTTP");
                    crate::metrics::SLOWLORIS_REJECTIONS.inc();
                    return Ok(());
                }
                Ok(HttpInspectionResult::NotHttp) => {
                    debug!(client = %client_peer, "Quick HTTP check was false positive, proceeding");
                }
                Err(e) => {
                    warn!(client = %client_peer, error = %e, "Error during HTTP inspection");
                    crate::metrics::SLOWLORIS_REJECTIONS.inc();
                    return Ok(());
                }
            }
        }
    }

    // MQTT-specific overlay
    if config.mqtt_inspect {
        if config.mqtt_full_inspect {
            // Apply MQTT CONNECT timeout if Slowloris protection enabled
            let connect_timeout = if config.slowloris_protect {
                Duration::from_millis(config.slowloris_config.mqtt_connect_timeout_ms)
            } else {
                Duration::from_secs(30) // Default fallback
            };

            let idle_timeout = if config.slowloris_protect {
                Duration::from_millis(config.slowloris_config.packet_idle_timeout_ms)
            } else {
                Duration::from_secs(10) // Default fallback
            };

            // Read fixed header with idle timeout
            let fixed_byte = if config.slowloris_protect {
                let mut buf = [0u8; 1];
                match read_with_idle_timeout(&mut source, &mut buf, idle_timeout, connect_timeout)
                    .await
                {
                    Ok(1) => buf[0],
                    Ok(_) => {
                        warn!(client = %client_peer, "EOF while reading MQTT fixed header");
                        crate::metrics::PROTOCOL_REJECTIONS.inc();
                        return Ok(());
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                        warn!(client = %client_peer, "Timeout reading MQTT fixed header (Slowloris)");
                        crate::metrics::SLOWLORIS_REJECTIONS.inc();
                        return Ok(());
                    }
                    Err(_) => {
                        crate::metrics::PROTOCOL_REJECTIONS.inc();
                        return Ok(());
                    }
                }
            } else {
                match read_fixed_header(&mut source).await {
                    Ok(b) => b,
                    Err(_) => return Ok(()),
                }
            };
            initial_bytes.push(fixed_byte);

            let packet_type = mqtt::inspect_packet(&[fixed_byte]);
            if packet_type != MqttPacketType::Connect {
                warn!(client = %client_peer, "Dropped: Expected CONNECT, detected {:?}", packet_type);
                crate::metrics::PROTOCOL_REJECTIONS.inc();
                return Ok(());
            }

            // Read remaining length (pass configured cap)
            let (rl_bytes, remaining_len) =
                match read_remaining_length(&mut source, config.max_connect_remaining).await {
                    Ok(v) => v,
                    Err(_) => return Ok(()),
                };
            initial_bytes.extend_from_slice(&rl_bytes);

            // Read payload
            let payload = match read_payload(&mut source, remaining_len).await {
                Ok(p) => p,
                Err(_) => return Ok(()),
            };
            if !payload.is_empty() {
                initial_bytes.extend_from_slice(&payload);
            }

            // Validate minimal CONNECT variable header
            if !validate_connect_variable_header(&payload) {
                warn!(client = %client_peer, "Malformed CONNECT: invalid protocol name/version or too short");
                crate::metrics::PROTOCOL_REJECTIONS.inc();
                return Ok(());
            }

            debug!(
                "Verified full MQTT CONNECT frame. Forwarding to {}",
                target_addr
            );
        } else {
            // Lightweight inspection: peek the first byte
            let mut buffer = [0u8; 1];
            let peek_res = timeout(Duration::from_secs(3), source.peek(&mut buffer)).await;
            if peek_res.is_err() {
                warn!(client = %client_peer, "Connection timed out waiting for MQTT data");
                crate::metrics::PROTOCOL_REJECTIONS.inc();
                return Ok(());
            }
            let packet_type = mqtt::inspect_packet(&buffer);
            if packet_type != MqttPacketType::Connect {
                warn!(client = %client_peer, "Dropped: Expected CONNECT, detected {:?}", packet_type);
                crate::metrics::PROTOCOL_REJECTIONS.inc();
                return Ok(());
            }
            debug!(
                "Verified MQTT CONNECT (initial check). Proceeding to backend connect: {}",
                target_addr
            );
        }
    } else {
        debug!(
            "MQTT inspection disabled; forwarding connection to {}",
            target_addr
        );
    }

    // client_peer already captured earlier for logging at inspection-time

    // Connect to backend
    let target = match connect_backend(&target_addr, &client_peer).await {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    let _guard = ProxyConnectionGuard::new();

    let (mut source_read, mut source_write) = source.into_split();
    let (mut target_read, mut target_write) = target.into_split();

    // Forward initial bytes if present
    if let Err(e) = forward_initial_bytes(
        &mut target_write,
        &initial_bytes,
        &target_addr,
        &client_peer,
    )
    .await
    {
        warn!(client = %client_peer, reason = %e, "Failed forwarding initial bytes to backend");
        return Ok(());
    }

    // Start bidirectional copying between client and backend
    let _ = tokio::select! {
        res = io::copy(&mut source_read, &mut target_write) => res,
        res = io::copy(&mut target_read, &mut source_write) => res,
    };

    debug!("Connection closed.");
    Ok(())
}
