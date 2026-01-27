use aegis_common::Config;
use aegis_proxy::engine::connection::handle_connection;
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
    let target_addr = config.proxy.target_address.clone();
    let master_token = CancellationToken::new();

    if config.metrics.enabled {
        let port = config.metrics.port;
        tokio::spawn(async move {
            run_metrics_server(port).await;
        });
    }

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

    let listener = TcpListener::bind(&config.proxy.listen_address).await?;
    info!(listen_addr = %config.proxy.listen_address, "AegisGate started");

    loop {
        tokio::select! {
            res = listener.accept() => {
                if let Ok((socket, addr)) = res {
                    let l_cfg = Arc::clone(&limit_cfg);
                    let target = target_addr.clone();

                    if check_rate_limit(addr.ip(), &l_cfg) {
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(socket, target).await {
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
