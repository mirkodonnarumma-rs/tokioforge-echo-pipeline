use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::watch;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    // Canale watch: il valore `false` diventa `true` quando si vuole lo shutdown
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    println!("Echo server su :8080 — Ctrl+C per shutdown");

    loop {
        tokio::select! {
            // Ramo 1: nuova connessione
            result = listener.accept() => {
                let (mut socket, addr) = result?;
                let mut rx = shutdown_rx.clone(); // clone del receiver (economico)

                tokio::spawn(async move {
                    let mut buf = [0u8; 1024];
                    loop {
                        tokio::select! {
                            // Ramo A: dati dal client
                            result = socket.read(&mut buf) => {
                                match result {
                                    Ok(0) | Err(_) => return,
                                    Ok(n) => {
                                        if socket.write_all(&buf[..n]).await.is_err() {
                                            return;
                                        }
                                    }
                                }
                            }
                            // Ramo B: segnale di shutdown
                            _ = rx.changed() => {
                                println!("{addr}: shutdown ricevuto");
                                return; // il drop di `socket` chiude la connessione
                            }
                        }
                    }
                });
            }
            // Ramo 2: segnale Ctrl+C
            _ = signal::ctrl_c() => {
                println!("\nShutdown in corso...");
                let _ = shutdown_tx.send(true);
                break; // esci dal loop di accept
            }
        }
    }
    // A questo punto i task vedranno il segnale e termineranno.
    // Per un shutdown più pulito si può usare JoinSet e attendere tutti i task.
    Ok(())
}