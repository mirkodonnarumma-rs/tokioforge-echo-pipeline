use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Echo server in ascolto su :8080");

    loop {
        let (mut socket, addr) = listener.accept().await?;
        println!("Connessione da {addr}");

        // `socket` viene MOVED nel task — ownership trasferita
        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(0) => return,           // EOF: client disconnesso
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("Errore lettura da {addr}: {e}");
                        return;
                    }
                };
                if let Err(e) = socket.write_all(&buf[..n]).await {
                    eprintln!("Errore scrittura verso {addr}: {e}");
                    return;
                }
            }
        });
    }
}