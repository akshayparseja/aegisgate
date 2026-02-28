use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub proxy: ProxyConfig,
    pub limit: LimitConfig,
    pub slowloris_protection: SlowlorisConfig,
    pub http_inspection: HttpInspectionConfig,
    pub metrics: MetricsConfig,
    pub features: FeaturesConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProxyConfig {
    pub listen_address: String,
    pub target_address: String,
    /// Optional maximum Remaining Length (in bytes) that will be accepted when
    /// performing full MQTT CONNECT inspection. If absent, callers should use a
    /// sensible default (e.g. 64 * 1024).
    pub max_connect_remaining: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LimitConfig {
    pub max_tokens: f64,
    pub refill_rate: f64,
    pub cleanup_interval_secs: u64,
    pub ip_idle_timeout_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SlowlorisConfig {
    /// Base layer: time to receive first packet after connection accepted (ms)
    pub first_packet_timeout_ms: u64,
    /// Max idle time between any bytes (ms)
    pub packet_idle_timeout_ms: u64,
    /// Total timeout for connection until first valid complete packet (ms)
    pub connection_timeout_ms: u64,

    /// MQTT-specific: max time to receive complete CONNECT packet (ms)
    pub mqtt_connect_timeout_ms: u64,
    /// MQTT-specific: max time to receive any other complete MQTT packet (ms)
    pub mqtt_packet_timeout_ms: u64,

    /// HTTP-specific: max time to receive complete HTTP request line + headers (ms)
    pub http_request_timeout_ms: u64,
    /// HTTP-specific: max total size of HTTP headers (bytes)
    pub max_http_header_size: usize,
    /// HTTP-specific: max number of HTTP headers
    pub max_http_header_count: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HttpInspectionConfig {
    /// Max size of individual HTTP header line (bytes)
    pub max_header_line_size: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub port: u16,
}

/// Feature flags to enable or disable proxy protections and subsystems.
#[derive(Debug, Deserialize, Clone)]
pub struct FeaturesConfig {
    /// Enable MQTT-level inspection and CONNECT validation.
    pub enable_mqtt_inspection: bool,
    /// Enable full MQTT Remaining Length + CONNECT validation.
    pub enable_mqtt_full_inspection: bool,
    /// Enable HTTP inspection and related protections.
    pub enable_http_inspection: bool,
    /// Enable Slowloris protection logic for HTTP.
    pub enable_slowloris_protection: bool,
    /// Enable per-IP token-bucket rate limiting.
    pub enable_rate_limiter: bool,
    /// Enable eBPF fast-path integration (if available).
    pub enable_ebpf: bool,
    /// Enable ML-based anomaly detection pipeline.
    pub enable_ml: bool,
}
