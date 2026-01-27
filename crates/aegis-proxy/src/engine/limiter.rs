use aegis_common::LimitConfig;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

pub struct TokenBucket {
    pub tokens: f64,
    pub last_refill: Instant,
}

pub static IP_TRACKER: Lazy<DashMap<IpAddr, TokenBucket>> = Lazy::new(DashMap::new);

pub fn check_rate_limit(addr: IpAddr, config: &LimitConfig) -> bool {
    let mut entry = IP_TRACKER.entry(addr).or_insert_with(|| TokenBucket {
        tokens: config.max_tokens,
        last_refill: Instant::now(),
    });

    let now = Instant::now();
    let elapsed = now.duration_since(entry.last_refill).as_secs_f64();

    let old_tokens = entry.tokens;
    entry.tokens = (entry.tokens + elapsed * config.refill_rate).min(config.max_tokens);
    entry.last_refill = now;

    if entry.tokens >= 1.0 {
        entry.tokens -= 1.0;
        debug!(
            "IP {}: {:.2} -> {:.2} (Allowed)",
            addr, old_tokens, entry.tokens
        );
        true
    } else {
        warn!(
            "IP {}: Rate limit hit. Tokens: {:.2} (Dropped)",
            addr, entry.tokens
        );
        false
    }
}

/// The Janitor now takes the global config to know its schedule
pub async fn start_cleanup_task(config: Arc<LimitConfig>) {
    let mut interval = tokio::time::interval(Duration::from_secs(config.cleanup_interval_secs));
    let timeout = Duration::from_secs(config.ip_idle_timeout_secs);

    loop {
        interval.tick().await;
        let now = Instant::now();
        let initial_size = IP_TRACKER.len();

        IP_TRACKER.retain(|_, bucket| now.duration_since(bucket.last_refill) < timeout);

        let final_size = IP_TRACKER.len();
        if initial_size != final_size {
            info!(
                "Cleanup: GC removed {} inactive IPs.",
                initial_size - final_size
            );
        }
    }
}
