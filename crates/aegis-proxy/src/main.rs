use aegis_common::Config;
use aegis_proxy::engine::connection::handle_connection;
use aegis_proxy::engine::limiter::{check_rate_limit, start_cleanup_task};
use std::fs;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let config_data = fs::read_to_string("config/aegis_config.yaml")?;
    let config: Config = serde_yaml::from_str(&config_data)?;

    let limit_cfg = Arc::new(config.limit.clone());
    let target_addr = config.proxy.target_address.clone();
    let master_token = CancellationToken::new();

    // Pass the global config to the Janitor
    let janitor_cfg = Arc::clone(&limit_cfg);
    let janitor_token = master_token.clone();
    tokio::spawn(async move {
        tokio::select! {
            _ = start_cleanup_task(janitor_cfg) => {},
            _ = janitor_token.cancelled() => {
                info!("Janitor shutting down.");
            }
        }
    });

    let listener = TcpListener::bind(&config.proxy.listen_address).await?;
    info!("AegisGate running on {}", config.proxy.listen_address);

    loop {
        tokio::select! {
            res = listener.accept() => {
                if let Ok((socket, addr)) = res {
                    let l_cfg = Arc::clone(&limit_cfg);
                    let target = target_addr.clone();

                    if check_rate_limit(addr.ip(), &l_cfg) {
                        tokio::spawn(async move {
                            let _ = handle_connection(socket, target).await;
                        });
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                master_token.cancel();
                break;
            }
        }
    }
    Ok(())
}
