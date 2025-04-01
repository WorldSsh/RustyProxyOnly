use std::env;
use std::io::Error;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let port = env::args().nth(1).unwrap_or_else(|| "80".to_string());
    let port: u16 = port.parse().unwrap_or(80);
    
    let listener = TcpListener::bind(format!("[::]:{}", port)).await?;
    println!("Servidor iniciado na porta: {}", port);
    
    loop {
        match listener.accept().await {
            Ok((client_stream, addr)) => {
                println!("Nova conexão de: {}", addr);
                tokio::spawn(handle_client(client_stream, addr));
            }
            Err(e) => eprintln!("Erro ao aceitar conexão: {}", e),
        }
    }
}

async fn handle_client(mut client_stream: TcpStream, addr: SocketAddr) {
    if let Err(e) = process_client(&mut client_stream).await {
        eprintln!("Erro ao processar cliente {}: {}", addr, e);
    }
}

async fn process_client(client_stream: &mut TcpStream) -> Result<(), Error> {
    let status = env::args().nth(2).unwrap_or_else(|| "@RustyManager".to_string());
    
    client_stream
        .write_all(format!("HTTP/1.1 101 {}

", status).as_bytes())
        .await?;
    
    let mut buffer = [0; 4096];
    if client_stream.read(&mut buffer).await? == 0 {
        return Ok(());
    }
    
    client_stream
        .write_all(format!("HTTP/1.1 200 {}

", status).as_bytes())
        .await?;
    
    let addr_proxy = match timeout(Duration::from_secs(5), peek_stream(client_stream)).await {
        Ok(Ok(data)) if data.contains("SSH") => "127.0.0.1:22",
        Ok(_) => "127.0.0.1:1194",
        Err(_) | Ok(Err(_)) => "127.0.0.1:22",
    };

    let server_stream = match TcpStream::connect(addr_proxy).await {
        Ok(stream) => stream,
        Err(_) => {
            eprintln!("Erro ao conectar ao servidor proxy em {}", addr_proxy);
            return Ok(());
        }
    };

    let (mut client_read, mut client_write) = client_stream.split();
    let (mut server_read, mut server_write) = server_stream.split();

    let client_to_server = tokio::spawn(async move {
        let mut buffer = vec![0; 4096];
        while let Ok(n) = client_read.read(&mut buffer).await {
            if n == 0 || server_write.write_all(&buffer[..n]).await.is_err() {
                break;
            }
        }
    });

    let server_to_client = tokio::spawn(async move {
        let mut buffer = vec![0; 4096];
        while let Ok(n) = server_read.read(&mut buffer).await {
            if n == 0 || client_write.write_all(&buffer[..n]).await.is_err() {
                break;
            }
        }
    });

    let _ = tokio::try_join!(client_to_server, server_to_client);
    Ok(())
}

async fn peek_stream(stream: &mut TcpStream) -> Result<String, Error> {
    let mut buffer = vec![0; 4096];
    let bytes_peeked = stream.peek(&mut buffer).await?;
    Ok(String::from_utf8_lossy(&buffer[..bytes_peeked]).to_string())
}
