use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::JoinSet;
use tracing::{info, warn};

async fn accept_or_shutdown(
    listener: &TcpListener,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> Option<(TcpStream, SocketAddr)> {
    tokio::select! {
        result = listener.accept() => result.ok(),
        _ = shutdown_rx.changed() => None,
    }
}

async fn handle_client(mut socket: TcpStream, addr: SocketAddr, mut shutdown_rx: watch::Receiver<bool>) {
    info!(%addr, "client connesso");
    let mut buf = [0u8; 1024];
    loop {
        tokio::select! {
            result = socket.read(&mut buf) => {
                match result {
                    Ok(0) => {
                        info!(%addr, "client disconnesso (EOF)");
                        return;
                    }
                    Ok(n) => {
                        if let Err(e) = socket.write_all(&buf[..n]).await {
                            warn!(%addr, "errore scrittura: {e}");
                            return;
                        }
                    }
                    Err(e) => {
                        warn!(%addr, "errore lettura: {e}");
                        return;
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                info!(%addr, "shutdown ricevuto");
                return;
            }
        }
    }
}

/// Avvia l'echo server sull'ascoltatore fornito.
/// Termina quando shutdown_rx riceve `true` o il listener restituisce errore.
/// Aspetta che tutti i task client attivi completino prima di tornare.
pub async fn run_echo_server(listener: TcpListener, mut shutdown_rx: watch::Receiver<bool>) {
    let mut join_set: JoinSet<()> = JoinSet::new();

    while let Some((socket, addr)) = accept_or_shutdown(&listener, &mut shutdown_rx).await {
        let rx = shutdown_rx.clone();
        join_set.spawn(handle_client(socket, addr, rx));
    }

    info!("accept loop chiuso — attendo task attivi...");
    while join_set.join_next().await.is_some() {}
}
