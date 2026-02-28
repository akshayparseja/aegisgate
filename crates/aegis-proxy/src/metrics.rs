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
        "Total number of connections rejected by rate limiting"
    )
    .expect("metric can be created");
    /// Count of connections rejected due to protocol validation (malformed packets, invalid CONNECT)
    pub static ref PROTOCOL_REJECTIONS: IntCounter = IntCounter::new(
        "aegis_protocol_rejections_total",
        "Total number of connections rejected by protocol (MQTT) validation"
    )
    .expect("metric can be created");
    /// Count of connections rejected due to HTTP detection (wrong protocol)
    pub static ref HTTP_REJECTIONS: IntCounter = IntCounter::new(
        "aegis_http_rejections_total",
        "Total number of connections rejected due to HTTP protocol detection"
    )
    .expect("metric can be created");
    /// Count of connections rejected due to Slowloris attack detection
    pub static ref SLOWLORIS_REJECTIONS: IntCounter = IntCounter::new(
        "aegis_slowloris_rejections_total",
        "Total number of connections rejected due to Slowloris attack detection"
    )
    .expect("metric can be created");
}

pub fn register_metrics() {
    let _ = REGISTRY.register(Box::new(CONNECTION_GAUGE.clone()));
    let _ = REGISTRY.register(Box::new(REJECTED_CONNECTIONS.clone()));
    let _ = REGISTRY.register(Box::new(PROTOCOL_REJECTIONS.clone()));
    let _ = REGISTRY.register(Box::new(HTTP_REJECTIONS.clone()));
    let _ = REGISTRY.register(Box::new(SLOWLORIS_REJECTIONS.clone()));
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
