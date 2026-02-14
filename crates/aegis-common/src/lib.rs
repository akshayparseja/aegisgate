use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub proxy: ProxyConfig,
    pub limit: LimitConfig,
    pub metrics: MetricsConfig,
    pub features: FeaturesConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProxyConfig {
    pub listen_address: String,
    pub target_address: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LimitConfig {
    pub max_tokens: f64,
    pub refill_rate: f64,
    pub cleanup_interval_secs: u64,
    pub ip_idle_timeout_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub port: u16,
}

/// Feature toggles to enable/disable specific protections or subsystems.
/// These flags allow running AegisGate in a minimal proxy-only mode
/// or enabling individual protections (MQTT inspection, HTTP inspection,
/// Slowloris protection, rate limiting, eBPF fast-path, ML inference).
#[derive(Debug, Deserialize, Clone)]
pub struct FeaturesConfig {
    /// Toggle MQTT-level inspection and CONNECT validation - Dev done
    pub enable_mqtt_inspection: bool,
    /// Toggle full MQTT Remaining Length + CONNECT field validation
    /// When false, only lightweight inspection (or none) is performed depending on other flags. - Dev done
    pub enable_mqtt_full_inspection: bool,
    /// Toggle HTTP parsing and HTTP-specific protections (Slowloris) - Todo
    pub enable_http_inspection: bool,
    /// Toggle Slowloris protection logic (header timeouts / per-connection checks) - Todo
    pub enable_slowloris_protection: bool,
    /// Toggle per-IP token bucket rate limiting
    pub enable_rate_limiter: bool,
    /// Toggle eBPF fast-path integration (if compiled & deployed) - Todo
    pub enable_ebpf: bool,
    /// Toggle ML-based anomaly detection (inference pipeline) - Todo
    pub enable_ml: bool,
}
