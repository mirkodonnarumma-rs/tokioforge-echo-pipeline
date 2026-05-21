use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::time::timeout;
use tokio_lab::run_echo_server;

/// Porta 0 = l'OS assegna una porta libera: zero conflitti tra test paralleli.

#[tokio::test]
async fn test_echo_basic() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let (_tx, rx) = watch::channel(false);
    tokio::spawn(run_echo_server(listener, rx));

    let mut client = TcpStream::connect(format!("127.0.0.1:{port}")).await.unwrap();
    client.write_all(b"hello").await.unwrap();

    let mut buf = [0u8; 5];
    timeout(Duration::from_secs(2), client.read_exact(&mut buf))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(&buf, b"hello");
}

#[tokio::test]
async fn test_echo_multiple_clients_concurrent() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let (_tx, rx) = watch::channel(false);
    tokio::spawn(run_echo_server(listener, rx));

    let addr1 = format!("127.0.0.1:{port}");
    let addr2 = addr1.clone();

    let t1 = tokio::spawn(async move {
        let mut s = TcpStream::connect(addr1).await.unwrap();
        s.write_all(b"aaa").await.unwrap();
        let mut buf = [0u8; 3];
        timeout(Duration::from_secs(2), s.read_exact(&mut buf))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&buf, b"aaa");
    });

    let t2 = tokio::spawn(async move {
        let mut s = TcpStream::connect(addr2).await.unwrap();
        s.write_all(b"bbb").await.unwrap();
        let mut buf = [0u8; 3];
        timeout(Duration::from_secs(2), s.read_exact(&mut buf))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&buf, b"bbb");
    });

    t1.await.unwrap();
    t2.await.unwrap();
}

#[tokio::test]
async fn test_graceful_shutdown_completes_server() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let server = tokio::spawn(run_echo_server(listener, shutdown_rx));

    // Verifica che l'echo funzioni prima di inviare lo shutdown
    let mut client = TcpStream::connect(format!("127.0.0.1:{port}")).await.unwrap();
    client.write_all(b"ping").await.unwrap();
    let mut buf = [0u8; 4];
    timeout(Duration::from_secs(2), client.read_exact(&mut buf))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(&buf, b"ping");

    // Segnale di shutdown: il server deve completare entro 2 secondi
    shutdown_tx.send(true).unwrap();
    timeout(Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap();
}
