use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target = "127.0.0.1:8080";
    let max_connections = 10_000;
    let mut handles = vec![];

    println!(
        "Starting stress test: Attempting {} connections to {}",
        max_connections, target
    );

    for i in 0..max_connections {
        let handle = tokio::spawn(async move {
            match TcpStream::connect(target).await {
                Ok(_stream) => {
                    // Keep the connection alive to test concurrency limits
                    sleep(Duration::from_secs(60)).await;
                }
                Err(e) => {
                    if i % 100 == 0 {
                        eprintln!("Connection {} failed: {}", i, e);
                    }
                }
            }
        });
        handles.push(handle);

        // Small delay to prevent local OS socket exhaustion during setup
        if i % 100 == 0 {
            sleep(Duration::from_millis(10)).await;
        }
    }

    println!("All connection attempts spawned. Monitoring...");

    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}
