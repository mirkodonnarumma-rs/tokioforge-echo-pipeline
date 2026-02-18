#[tokio::test]
async fn echo_works() {
    use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};

    let mut stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    stream.write_all(b"hello").await.unwrap();

    let mut buf = [0u8; 5];
    stream.read_exact(&mut buf).await.unwrap();

    assert_eq!(&buf, b"hello");
}
