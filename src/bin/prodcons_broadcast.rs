use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let (tx, _) = broadcast::channel::<String>(8);

    let mut handles = Vec::new();
    for (id, delay_ms) in [(0usize, 10u64), (1, 50), (2, 200)] {
        let mut rx = tx.subscribe();
        handles.push(tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        info!(consumer = id, msg, "ricevuto");
                        sleep(Duration::from_millis(delay_ms)).await;
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(consumer = id, persi = n, "buffer overflow — messaggi persi");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!(consumer = id, "canale chiuso — uscita");
                        break;
                    }
                }
            }
        }));
    }

    for i in 0..20u32 {
        let msg = format!("msg-{i:02}");
        info!(msg, "producer: invio");
        tx.send(msg).unwrap();
        sleep(Duration::from_millis(10)).await;
    }
    drop(tx);

    for h in handles {
        h.await.unwrap();
    }
    info!("tutti i consumer terminati");
}
