use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<String>(4);

    let producer = tokio::spawn(async move {
        for i in 0..20u32 {
            let msg = format!("msg-{i:02}");
            println!("[P] Invio: {msg}");
            tx.send(msg).await.unwrap();
        }
        println!("[P] Fine — sender droppato, canale si chiuderà.");
    });

    let consumer = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            println!("[C] Ricevuto: {msg}");
            sleep(Duration::from_millis(200)).await;
        }
        println!("[C] Canale chiuso — consumer terminato.");
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
            // tx droppato qui: rx.recv() tornerà None dopo l'ultimo messaggio
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
