use std::env;
use std::io::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let port = get_port();
    let listener = TcpListener::bind(format!("[::]:{}", port)).await?;
    println!("Servidor iniciado na porta: {}", port);
    start_proxy(listener).await;
    Ok(())
}

async fn start_proxy(listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((client_stream, addr)) => {
                println!("Nova conexão de: {}", addr);
                tokio::spawn(async move {
                    if let Err(e) = handle_client(client_stream).await {
                        eprintln!("Erro ao processar cliente {}: {}", addr, e);
                    }
                });
            }
            Err(e) => eprintln!("Erro ao aceitar conexão: {}", e),
        }
    }
}

async fn handle_client(mut client_stream: TcpStream) -> Result<(), Error> {
    let status = get_status();
    client_stream.write_all(format!("HTTP/1.1 101 {}\r\n\r\n", status).as_bytes()).await?;

    let addr_proxy = detect_service(&mut client_stream).await.unwrap_or("0.0.0.0:22");

    let server_stream = TcpStream::connect(addr_proxy).await?;

    bidirectional_transfer(client_stream, server_stream).await;

    Ok(())
}

async fn detect_service(stream: &mut TcpStream) -> Result<&'static str, Error> {
    let mut buffer = vec![0; 1024];
    let _ = timeout(Duration::from_secs(1), stream.read(&mut buffer)).await;
    let data = String::from_utf8_lossy(&buffer);

    if data.contains("SSH") {
        Ok("0.0.0.0:22")
    } else {
        Ok("0.0.0.0:1194")
    }
}

async fn bidirectional_transfer(mut client_stream: TcpStream, mut server_stream: TcpStream) {
    let (mut client_read, mut client_write) = client_stream.into_split();
    let (mut server_read, mut server_write) = server_stream.into_split();

    let client_to_server = async {
        let mut buffer = vec![0; 8192];
        while let Ok(bytes_read) = client_read.read(&mut buffer).await {
            if bytes_read == 0 {
                break;
            }
            if server_write.write_all(&buffer[..bytes_read]).await.is_err() {
                break;
            }
        }
    };

    let server_to_client = async {
        let mut buffer = vec![0; 8192];
        while let Ok(bytes_read) = server_read.read(&mut buffer).await {
            if bytes_read == 0 {
                break;
            }
            if client_write.write_all(&buffer[..bytes_read]).await.is_err() {
                break;
            }
        }
    };

    tokio::join!(client_to_server, server_to_client);
}

fn get_port() -> u16 {
    env::args().nth(2).unwrap_or_else(|| "80".to_string()).parse().unwrap_or(80)
}

fn get_status() -> String {
    env::args().nth(4).unwrap_or_else(|| "@RustyManager".to_string())
}
