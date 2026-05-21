use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::JoinSet;

async fn accept_or_shutdown(
    listener: &TcpListener,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> Option<(TcpStream, SocketAddr)> {
    tokio::select! {
        result = listener.accept() => result.ok(),
        _ = shutdown_rx.changed() => None,
    }
}

async fn handle_client(mut socket: TcpStream, mut shutdown_rx: watch::Receiver<bool>) {
    let mut buf = [0u8; 1024];
    loop {
        tokio::select! {
            result = socket.read(&mut buf) => {
                match result {
                    Ok(0) => return,
                    Ok(n) => {
                        if socket.write_all(&buf[..n]).await.is_err() {
                            return;
                        }
                    }
                    Err(_) => return,
                }
            }
            _ = shutdown_rx.changed() => return,
        }
    }
}

/// Avvia l'echo server sull'ascoltatore fornito.
/// Termina quando shutdown_rx riceve `true` o il listener restituisce errore.
/// Aspetta che tutti i task client attivi completino prima di tornare.
pub async fn run_echo_server(listener: TcpListener, mut shutdown_rx: watch::Receiver<bool>) {
    let mut join_set: JoinSet<()> = JoinSet::new();

    while let Some((socket, addr)) = accept_or_shutdown(&listener, &mut shutdown_rx).await {
        println!("[INFO] Connessione da {addr}");
        let rx = shutdown_rx.clone();
        join_set.spawn(handle_client(socket, rx));
    }

    println!("[INFO] Accept loop chiuso — attendo task attivi...");
    while join_set.join_next().await.is_some() {}
}
