use std::error::Error;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let target = "127.0.0.1:8080";
    let total_requests = 20;
    let mut handles = vec![];

    println!("🚀 Starting Rust-based stress test against {}", target);

    for i in 1..=total_requests {
        let handle = tokio::spawn(async move {
            match TcpStream::connect(target).await {
                Ok(mut stream) => {
                    let _ = stream.write_all(&[0x10]).await;
                    println!("[{}] ✅ Connection successful", i);
                }
                Err(_) => {
                    println!("[{}] ❌ Connection refused (Rate Limited)", i);
                }
            }
        });
        handles.push(handle);

        if i % 5 == 0 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    for handle in handles {
        let _ = handle.await;
    }

    println!("\n✨ Stress test complete.");
    Ok(())
}
