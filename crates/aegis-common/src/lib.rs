use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub proxy: ProxyConfig,
    pub limit: LimitConfig,
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
