use crate::parser::mqtt::{self, MqttPacketType};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use tracing::{debug, warn};

pub static ACTIVE_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);

pub async fn handle_connection(
    source: TcpStream,
    target_addr: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ACTIVE_CONNECTIONS.fetch_add(1, Ordering::SeqCst);

    // 1. Peek to validate protocol
    let mut buffer = [0u8; 1];
    let peek_res = timeout(Duration::from_secs(3), source.peek(&mut buffer)).await;

    if peek_res.is_err() {
        warn!("Connection timed out waiting for MQTT data");
        ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::SeqCst);
        return Ok(());
    }

    let packet_type = mqtt::inspect_packet(&buffer);

    if packet_type != MqttPacketType::Connect {
        warn!("Dropped: Expected CONNECT, detected {:?}", packet_type);
        ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::SeqCst);
        return Ok(());
    }

    debug!("Verified MQTT CONNECT. Forwarding to {}", target_addr);

    let target = match timeout(Duration::from_secs(5), TcpStream::connect(&target_addr)).await {
        Ok(stream) => stream?,
        Err(_) => {
            warn!("Could not connect to EMQX at {}", target_addr);
            ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::SeqCst);
            return Ok(());
        }
    };

    let (mut source_read, mut source_write) = source.into_split();
    let (mut target_read, mut target_write) = target.into_split();

    let _ = tokio::select! {
        _ = io::copy(&mut source_read, &mut target_write) => {},
        _ = io::copy(&mut target_read, &mut source_write) => {},
    };

    ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::SeqCst);
    debug!("Connection closed.");
    Ok(())
}
