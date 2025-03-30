use std::env;
use std::io::Error;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let port = get_port();
    let listener = TcpListener::bind(format!("[::]:{}", port)).await?;
    println!("Iniciando serviço na porta: {}", port);
    
    start_proxy(listener).await;
    Ok(())
}

async fn start_proxy(listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((client_stream, addr)) => {
                println!("Nova conexão de {}", addr);
                tokio::spawn(async move {
                    if let Err(e) = handle_client(client_stream).await {
                        eprintln!("Erro ao processar cliente {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Erro ao aceitar conexão: {}", e);
            }
        }
    }
}

async fn handle_client(mut client_stream: TcpStream) -> Result<(), Error> {
    send_response(&mut client_stream, get_status()).await?;
    
    let addr_proxy = determine_proxy_address(&mut client_stream).await;
    println!("Encaminhando para: {}", addr_proxy);
    
    match TcpStream::connect(addr_proxy).await {
        Ok(server_stream) => relay_data(client_stream, server_stream).await?,
        Err(_) => eprintln!("Erro ao conectar ao proxy: {}", addr_proxy),
    }
    
    Ok(())
}

async fn relay_data(client_stream: TcpStream, server_stream: TcpStream) -> Result<(), Error> {
    let (mut client_read, mut client_write) = client_stream.into_split();
    let (mut server_read, mut server_write) = server_stream.into_split();

    let client_to_server = async {
        let mut buffer = [0; 8192];
        loop {
            match client_read.read(&mut buffer).await {
                Ok(0) => break, // Conexão fechada
                Ok(n) => {
                    println!("Cliente -> Servidor: {} bytes", n);
                    server_write.write_all(&buffer[..n]).await?;
                }
                Err(e) => {
                    eprintln!("Erro ao ler do cliente: {}", e);
                    break;
                }
            }
        }
        Ok::<(), Error>(())
    };

    let server_to_client = async {
        let mut buffer = [0; 8192];
        loop {
            match server_read.read(&mut buffer).await {
                Ok(0) => break, // Conexão fechada
                Ok(n) => {
                    println!("Servidor -> Cliente: {} bytes", n);
                    client_write.write_all(&buffer[..n]).await?;
                }
                Err(e) => {
                    eprintln!("Erro ao ler do servidor: {}", e);
                    break;
                }
            }
        }
        Ok::<(), Error>(())
    };

    tokio::try_join!(client_to_server, server_to_client)?;
    println!("Conexão encerrada");
    Ok(())
}

async fn determine_proxy_address(client_stream: &mut TcpStream) -> &'static str {
    let result = timeout(Duration::from_secs(1), peek_stream(client_stream)).await;
    match result {
        Ok(Ok(data)) if data.contains("SSH") || data.is_empty() => "0.0.0.0:22",
        Ok(_) => "0.0.0.0:1194",
        _ => "0.0.0.0:22",
    }
}

async fn send_response(client_stream: &mut TcpStream, status: String) -> Result<(), Error> {
    client_stream
        .write_all(format!("HTTP/1.1 200 {}\r\n\r\n", status).as_bytes())
        .await
}

async fn peek_stream(stream: &TcpStream) -> Result<String, Error> {
    let mut peek_buffer = vec![0; 8192];
    let bytes_peeked = stream.peek(&mut peek_buffer).await?;
    Ok(String::from_utf8_lossy(&peek_buffer[..bytes_peeked]).to_string())
}

fn get_port() -> u16 {
    env::args().nth(2).and_then(|port| port.parse().ok()).unwrap_or(80)
}

fn get_status() -> String {
    env::args()
        .nth(4)
        .unwrap_or_else(|| "@RustyManager".to_string())
}
