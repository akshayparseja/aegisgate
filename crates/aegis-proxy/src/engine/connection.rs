use crate::parser::mqtt::{self, MqttPacketType};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io;
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

pub async fn handle_connection(
    source: TcpStream,
    target_addr: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _guard = ConnectionGuard::new();

    let mut buffer = [0u8; 1];
    let peek_res = timeout(Duration::from_secs(3), source.peek(&mut buffer)).await;

    if peek_res.is_err() {
        warn!("Connection timed out waiting for MQTT data");
        return Ok(());
    }

    let packet_type = mqtt::inspect_packet(&buffer);

    if packet_type != MqttPacketType::Connect {
        warn!("Dropped: Expected CONNECT, detected {:?}", packet_type);
        return Ok(());
    }

    debug!("Verified MQTT CONNECT. Forwarding to {}", target_addr);

    // 3. Connect to Backend (Wait up to 5 seconds)
    let target = match timeout(Duration::from_secs(5), TcpStream::connect(&target_addr)).await {
        Ok(stream) => stream?,
        Err(_) => {
            warn!("Could not connect to backend at {}", target_addr);
            return Ok(()); // _guard drops here -> Counter -1
        }
    };

    let (mut source_read, mut source_write) = source.into_split();
    let (mut target_read, mut target_write) = target.into_split();

    let _ = tokio::select! {
        res = io::copy(&mut source_read, &mut target_write) => res,
        res = io::copy(&mut target_read, &mut source_write) => res,
    };

    debug!("Connection closed.");
    Ok(()) // _guard drops here -> Counter -1
}
