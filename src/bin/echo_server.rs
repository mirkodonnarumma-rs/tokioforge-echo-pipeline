use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::watch;
use tokio_lab::run_echo_server;

fn log(level: &str, msg: &str) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (h, m, s) = ((secs % 86400) / 3600, (secs % 3600) / 60, secs % 60);
    println!("[{h:02}:{m:02}:{s:02} {level}] {msg}");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    log("INFO", "Server in ascolto su 127.0.0.1:8080");

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let server = tokio::spawn(run_echo_server(listener, shutdown_rx));

    signal::ctrl_c().await?;
    log("INFO", "Ctrl+C ricevuto — shutdown in corso...");
    let _ = shutdown_tx.send(true);

    server.await?;
    log("INFO", "Server fermato.");
    Ok(())
}
