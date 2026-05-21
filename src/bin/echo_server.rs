use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::watch;
use tokio_lab::run_echo_server;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    info!("server in ascolto su 127.0.0.1:8080");

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let server = tokio::spawn(run_echo_server(listener, shutdown_rx));

    signal::ctrl_c().await?;
    info!("Ctrl+C ricevuto — shutdown in corso...");
    let _ = shutdown_tx.send(true);

    server.await?;
    info!("server fermato");
    Ok(())
}
