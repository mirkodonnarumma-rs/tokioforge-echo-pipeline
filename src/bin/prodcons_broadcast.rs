use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel::<String>(8);

    let mut handles = Vec::new();
    for (id, delay_ms) in [(0usize, 10u64), (1, 50), (2, 200)] {
        let mut rx = tx.subscribe();
        handles.push(tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        println!("[C{id}] Ricevuto: {msg}");
                        sleep(Duration::from_millis(delay_ms)).await;
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        println!("[C{id}] WARNING: persi {n} messaggi (buffer overflow)");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        println!("[C{id}] Canale chiuso — uscita.");
                        break;
                    }
                }
            }
        }));
    }

    for i in 0..20u32 {
        let msg = format!("msg-{i:02}");
        println!("[P] Invio: {msg}");
        tx.send(msg).unwrap();
        sleep(Duration::from_millis(10)).await;
    }
    drop(tx);

    for h in handles {
        h.await.unwrap();
    }
    println!("[P] Tutti i consumer terminati.");
}
