use tokio::sync::mpsc;
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

    let (tx, mut rx) = mpsc::channel::<String>(4);

    let producer = tokio::spawn(async move {
        for i in 0..20u32 {
            let msg = format!("msg-{i:02}");
            info!(msg, "producer: invio");
            if tx.send(msg).await.is_err() {
                warn!("producer: receiver droppato, uscita anticipata");
                return;
            }
        }
        info!("producer: fine — sender droppato, canale si chiuderà");
    });

    let consumer = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            info!(msg, "consumer: ricevuto");
            sleep(Duration::from_millis(200)).await;
        }
        info!("consumer: canale chiuso — terminato");
    });

    let (p, c) = tokio::join!(producer, consumer);
    p.unwrap();
    c.unwrap();
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_all_messages_delivered() {
        let (tx, mut rx) = mpsc::channel(16);

        let producer = tokio::spawn(async move {
            for i in 0..5u32 {
                tx.send(i).await.unwrap();
            }
        });

        producer.await.unwrap();

        let mut results = Vec::new();
        while let Some(val) = rx.recv().await {
            results.push(val);
        }

        assert_eq!(results, vec![0u32, 1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn test_send_fails_after_receiver_dropped() {
        let (tx, rx) = mpsc::channel::<i32>(10);
        drop(rx);
        assert!(tx.send(42).await.is_err());
    }
}
