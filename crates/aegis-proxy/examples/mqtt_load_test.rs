use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    let mut handles = vec![];
    let conn_count = 11;
    let proxy_addr = "127.0.0.1";
    let proxy_port = 8080;

    println!(
        "🚀 Starting load test: {} clients via AegisGate",
        conn_count
    );
    println!("⏱️ Estimated setup time: {} seconds", conn_count);

    for i in 0..conn_count {
        let mut mqttoptions = MqttOptions::new(format!("client-{}", i), proxy_addr, proxy_port);
        mqttoptions.set_keep_alive(Duration::from_secs(5));

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

        println!("[Client {}] Attempting connection...", i);

        let h = tokio::spawn(async move {
            // Start the event loop in the background
            tokio::spawn(async move {
                loop {
                    match eventloop.poll().await {
                        Ok(Event::Incoming(Packet::Publish(p))) => {
                            println!("[Client {}] Received message on topic: {}", i, p.topic);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("[Client {}] Connection Error: {:?}", i, e);
                            break;
                        }
                    }
                }
            });

            // 1. Subscribe
            if let Err(e) = client.subscribe("aegis/test/topic", QoS::AtMostOnce).await {
                println!("[Client {}] ❌ Subscription failed: {:?}", i, e);
                return;
            }

            // 2. Periodically Publish
            for msg_id in 1..=3 {
                let payload = format!("Hello #{} from client {}", msg_id, i);
                if let Err(_) = client
                    .publish("aegis/test/topic", QoS::AtMostOnce, false, payload)
                    .await
                {
                    break;
                }
                sleep(Duration::from_secs(5)).await;
            }
            println!("[Client {}] ✅ Task completed.", i);
        });

        handles.push(h);

        sleep(Duration::from_millis(500)).await;
    }

    println!("\n🔥 All clients spawned. Waiting for tasks to finish...");
    for h in handles {
        let _ = h.await;
    }
}
