use crate::engine::connection::ACTIVE_CONNECTIONS;
use lazy_static::lazy_static;
use prometheus::{Encoder, Gauge, IntCounter, Registry, TextEncoder};
use std::sync::atomic::Ordering;

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref CONNECTION_GAUGE: Gauge = Gauge::new(
        "aegis_active_connections",
        "Number of currently active MQTT proxy connections"
    )
    .expect("metric can be created");
    pub static ref REJECTED_CONNECTIONS: IntCounter = IntCounter::new(
        "aegis_rejected_connections_total",
        "Total number of connections rejected by protocol validation"
    )
    .expect("metric can be created");
}

pub fn register_metrics() {
    let _ = REGISTRY.register(Box::new(CONNECTION_GAUGE.clone()));
    let _ = REGISTRY.register(Box::new(REJECTED_CONNECTIONS.clone()));
}

fn update_metrics() {
    let count = ACTIVE_CONNECTIONS.load(Ordering::SeqCst) as f64;
    CONNECTION_GAUGE.set(count);
}

pub fn render_metrics() -> String {
    update_metrics();

    let metric_families = REGISTRY.gather();
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();

    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        return format!("# Error encoding metrics: {}", e);
    }

    String::from_utf8(buffer).unwrap_or_else(|_| "# Error: Invalid UTF8".to_string())
}
