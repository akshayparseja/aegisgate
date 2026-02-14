use crate::parser::mqtt::{self, MqttPacketType};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use tracing::{debug, warn};

pub static ACTIVE_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);

struct ConnectionGuard;

impl ConnectionGuard {
    fn new() -> Self {
        ACTIVE_CONNECTIONS.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Handle a single client connection.
///
/// The `enable_mqtt_inspection` flag controls whether the handler performs
/// lightweight MQTT CONNECT validation (peek + basic packet type check).
/// When disabled, the handler acts as a transparent TCP proxy for the connection.
pub async fn handle_connection(
    mut source: TcpStream,
    target_addr: String,
    mqtt_inspect: bool,
    mqtt_full_inspect: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _guard = ConnectionGuard::new();
    // Buffer to hold any bytes already consumed from the client (e.g. CONNECT fixed header,
    // Remaining Length bytes, and the CONNECT payload) so we can forward them to the backend
    // after establishing the backend connection.
    let mut initial_bytes: Vec<u8> = Vec::new();

    // If MQTT inspection is enabled, perform validation. There are two modes:
    // - lightweight inspection (only peek the first byte to check packet type)
    // - full inspection (read fixed header, decode Remaining Length, read payload,
    //   and validate minimal CONNECT variable header fields)
    //
    // If inspection is disabled, act as a transparent TCP proxy.
    if mqtt_inspect {
        if mqtt_full_inspect {
            // Full CONNECT validation path.

            // 1) Read fixed header (1 byte) with timeout
            let mut fixed = [0u8; 1];
            match timeout(Duration::from_secs(3), source.read_exact(&mut fixed)).await {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    warn!("Error reading fixed header from client: {}", e);
                    // Increment protocol rejection metric
                    crate::metrics::PROTOCOL_REJECTIONS.inc();
                    return Ok(());
                }
                Err(_) => {
                    warn!("Timeout waiting for MQTT fixed header");
                    crate::metrics::PROTOCOL_REJECTIONS.inc();
                    return Ok(());
                }
            }

            // Buffer the fixed header byte so it can be forwarded to the backend after connect
            initial_bytes.push(fixed[0]);

            // Quick packet-type check (high nibble)
            let packet_type = mqtt::inspect_packet(&fixed);
            if packet_type != MqttPacketType::Connect {
                warn!("Dropped: Expected CONNECT, detected {:?}", packet_type);
                crate::metrics::PROTOCOL_REJECTIONS.inc();
                return Ok(());
            }

            // 2) Read Remaining Length bytes (up to 4). Read one byte at a time until decoder completes.
            let mut rl_bytes: Vec<u8> = Vec::with_capacity(4);
            let mut rl_complete = false;
            let mut remaining_len: usize = 0;

            for _ in 0..4 {
                let mut b = [0u8; 1];
                match timeout(Duration::from_secs(1), source.read_exact(&mut b)).await {
                    Ok(Ok(_)) => {
                        rl_bytes.push(b[0]);
                        match mqtt::decode_remaining_length(&rl_bytes) {
                            Ok((v, _used)) => {
                                remaining_len = v;
                                rl_complete = true;
                                break;
                            }
                            Err("Incomplete") => {
                                // need to read more bytes in next iteration
                                continue;
                            }
                            Err(_) => {
                                warn!("Malformed Remaining Length received from client");
                                crate::metrics::PROTOCOL_REJECTIONS.inc();
                                return Ok(());
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        warn!("Error reading Remaining Length byte: {}", e);
                        crate::metrics::PROTOCOL_REJECTIONS.inc();
                        return Ok(());
                    }
                    Err(_) => {
                        warn!("Timeout reading Remaining Length bytes");
                        crate::metrics::PROTOCOL_REJECTIONS.inc();
                        return Ok(());
                    }
                }
            }

            if !rl_complete {
                // After up to 4 bytes, if remaining length is still not parsed, drop the connection
                warn!("Incomplete or malformed Remaining Length after 4 bytes");
                crate::metrics::PROTOCOL_REJECTIONS.inc();
                return Ok(());
            }

            // Append Remaining Length bytes to the initial buffer so they can be forwarded
            initial_bytes.extend_from_slice(&rl_bytes);

            // 3) Read the rest of the CONNECT packet (variable header + payload) using remaining_len
            let mut payload = vec![0u8; remaining_len];
            if remaining_len > 0 {
                match timeout(Duration::from_secs(5), source.read_exact(&mut payload)).await {
                    Ok(Ok(_)) => {}
                    Ok(Err(e)) => {
                        warn!("Error reading CONNECT payload: {}", e);
                        crate::metrics::PROTOCOL_REJECTIONS.inc();
                        return Ok(());
                    }
                    Err(_) => {
                        warn!("Timeout reading CONNECT payload");
                        crate::metrics::PROTOCOL_REJECTIONS.inc();
                        return Ok(());
                    }
                }
            }

            // Append payload to buffer to forward to backend
            if !payload.is_empty() {
                initial_bytes.extend_from_slice(&payload);
            }

            // 4) Minimal CONNECT validation:
            // The CONNECT variable header typically begins with a two-byte length (0x00 0x04) followed by "MQTT"
            // Ensure payload has at least 6 bytes: 2 length + 4 bytes ("MQTT")
            if payload.len() < 6 {
                warn!("Malformed CONNECT: variable header too short");
                crate::metrics::PROTOCOL_REJECTIONS.inc();
                return Ok(());
            }
            // Use a proper byte-string literal for comparison
            if !(payload[0] == 0x00 && payload[1] == 0x04 && &payload[2..6] == b"MQTT") {
                warn!("Malformed CONNECT: invalid protocol name/version");
                crate::metrics::PROTOCOL_REJECTIONS.inc();
                return Ok(());
            }

            debug!("Verified full MQTT CONNECT frame (fixed header + remaining length + variable header). Forwarding to {}", target_addr);
        } else {
            // Lightweight inspection: peek the first byte without consuming it
            let mut buffer = [0u8; 1];
            let peek_res = timeout(Duration::from_secs(3), source.peek(&mut buffer)).await;
            if peek_res.is_err() {
                warn!("Connection timed out waiting for MQTT data");
                crate::metrics::PROTOCOL_REJECTIONS.inc();
                return Ok(());
            }

            let packet_type = mqtt::inspect_packet(&buffer);
            if packet_type != MqttPacketType::Connect {
                warn!("Dropped: Expected CONNECT, detected {:?}", packet_type);
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

    // Connect to backend broker with timeout (with additional debug logging)
    let client_peer = source
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "<unknown>".to_string());
    debug!(
        "Attempting backend connect to {} for client {}",
        target_addr, client_peer
    );
    let target = match timeout(Duration::from_secs(5), TcpStream::connect(&target_addr)).await {
        Ok(stream) => {
            // `stream` is Result<TcpStream, io::Error> so use `?` to propagate any inner error
            let s = stream?;
            debug!(
                "Successfully connected to backend {} for client {}",
                target_addr, client_peer
            );
            s
        }
        Err(_) => {
            warn!(
                "Could not connect to backend at {} (connect timeout) for client {}",
                target_addr, client_peer
            );
            return Ok(());
        }
    };

    // Split streams for proxying
    let (mut source_read, mut source_write) = source.into_split();
    let (mut target_read, mut target_write) = target.into_split();

    // If we have already consumed bytes from the client (e.g., CONNECT fixed header + RL + payload),
    // forward them first to the backend so the backend sees the complete initial packet.
    if !initial_bytes.is_empty() {
        // Provide a short hex preview and length for debugging
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
            target_write.write_all(&initial_bytes),
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
            }
            Ok(Err(e)) => {
                warn!(
                    "Error writing initial CONNECT bytes to backend {} for client {}: {}",
                    target_addr, client_peer, e
                );
                // Provide a debug dump of error context (if available) before bailing out
                debug!(
                    "Initial bytes length: {}, preview: {}",
                    initial_bytes.len(),
                    preview
                );
                return Ok(());
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
                return Ok(());
            }
        }
    }

    let _ = tokio::select! {
        res = io::copy(&mut source_read, &mut target_write) => res,
        res = io::copy(&mut target_read, &mut source_write) => res,
    };

    debug!("Connection closed.");
    Ok(())
}
