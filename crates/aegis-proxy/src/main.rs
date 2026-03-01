use aegis_common::Config;
use aegis_proxy::engine::connection::{handle_connection, ConnectionConfig};
use aegis_proxy::engine::limiter::{check_rate_limit, start_cleanup_task};
use aegis_proxy::metrics;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server, StatusCode,
};
use std::convert::Infallible;
use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn init_production_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json().with_target(true))
        .init();

    info!("Production structured logging initialized (JSON)");
}

/// Handle simple HTTP endpoints for liveness and metrics.
async fn metrics_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match req.uri().path() {
        "/health" => Ok(Response::new(Body::from("OK"))),
        "/metrics" => Ok(Response::new(Body::from(metrics::render_metrics()))),
        _ => {
            let mut not_found = Response::new(Body::from("Not Found"));
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

async fn run_metrics_server(port: u16) {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    metrics::register_metrics();

    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(metrics_handler)) });

    let server = Server::bind(&addr).serve(make_svc);

    info!(port = port, "Observability server online");

    if let Err(e) = server.await {
        error!(error = %e, "Observability server failed");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_production_logging();

    let config_data = fs::read_to_string("config/aegis_config.yaml")?;
    let config: Config = serde_yaml::from_str(&config_data)?;

    let limit_cfg = Arc::new(config.limit.clone());
    let slowloris_cfg = Arc::new(config.slowloris_protection.clone());
    let target_addr = config.proxy.target_address.clone();
    // Configure maximum Remaining Length (bytes) allowed for full CONNECT inspection.
    // If the YAML omits this value, fall back to a safe default of 64 KiB.
    let max_connect_remaining = config.proxy.max_connect_remaining.unwrap_or(64 * 1024);
    let master_token = CancellationToken::new();
    let features = config.features.clone();

    if config.metrics.enabled {
        let port = config.metrics.port;
        tokio::spawn(async move {
            run_metrics_server(port).await;
        });
    }

    if features.enable_rate_limiter {
        let janitor_cfg = Arc::clone(&limit_cfg);
        let janitor_token = master_token.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = start_cleanup_task(janitor_cfg) => {},
                _ = janitor_token.cancelled() => {
                    info!("Janitor task shutting down");
                }
            }
        });
    }

    let listener = TcpListener::bind(&config.proxy.listen_address).await?;
    info!(listen_addr = %config.proxy.listen_address, "AegisGate started");

    loop {
        tokio::select! {
            res = listener.accept() => {
                if let Ok((socket, addr)) = res {
                    let l_cfg = Arc::clone(&limit_cfg);
                    let sl_cfg = Arc::clone(&slowloris_cfg);
                    let target = target_addr.clone();
                    let rate_limiter_enabled = features.enable_rate_limiter;

                    let allowed = !rate_limiter_enabled || check_rate_limit(addr.ip(), &l_cfg);

                    if allowed {
                        let conn_config = ConnectionConfig {
                            mqtt_inspect: features.enable_mqtt_inspection,
                            mqtt_full_inspect: features.enable_mqtt_full_inspection,
                            http_inspect: features.enable_http_inspection,
                            slowloris_protect: features.enable_slowloris_protection,
                            max_connect_remaining,
                            slowloris_config: (*sl_cfg).clone(),
                        };
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(
                                socket,
                                target,
                                conn_config,
                            ).await {
                                error!(client_ip = %addr.ip(), error = %e, "Connection error");
                            }
                        });
                    } else {
                        if config.metrics.enabled {
                            metrics::REJECTED_CONNECTIONS.inc();
                        }
                        warn!(client_ip = %addr.ip(), "Rate limit exceeded");
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Shutdown signal received");
                master_token.cancel();
                break;
            }
        }
    }
    Ok(())
}
