use std::env;
use std::io::Error;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::{time::{Duration}};
use tokio::time::timeout;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Iniciando o proxy
    let port = get_port();
    let listener = TcpListener::bind(format!("[::]:{}", port)).await?;
    println!("Iniciando serviço na porta: {}", port);
    start_http(listener).await;
    Ok(())
}

async fn start_http(listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((client_stream, addr)) => {
                tokio::spawn(async move {
                    if let Err(e) = handle_client(client_stream).await {
                        println!("Erro ao processar cliente {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                println!("Erro ao aceitar conexão: {}", e);
            }
        }
    }
}

async fn handle_client(mut stream: TcpStream) -> Result<(), Error> {
    let mut buffer = vec![0; 1024];
    let bytes_read = timeout(Duration::from_secs(5), stream.read(&mut buffer)).await??;
    
    if bytes_read == 0 {
        return Ok(());
    }
    
    println!("Recebido: {}", String::from_utf8_lossy(&buffer[..bytes_read]));
    
    stream.write_all(b"HTTP/1.1 200 OK\r\n\r\nHello, world!").await?;
    stream.flush().await?;
    Ok(())
}

fn get_port() -> u16 {
    env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse().unwrap_or(8080)
}
