use std::env;
use std::io::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};
use chrono::Local;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let port = get_port();
    let listener = TcpListener::bind(format!("[::]:{}", port)).await?;
    log("INFO", &format!("Iniciando serviço na porta: {}", port));
    start_http(listener).await;
    Ok(())
}

async fn start_http(listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((client_stream, addr)) => {
                tokio::spawn(async move {
                    if let Err(e) = handle_client(client_stream).await {
                        log("ERROR", &format!("Erro ao processar cliente {}: {}", addr, e));
                    }
                });
            }
            Err(e) => {
                log("ERROR", &format!("Erro ao aceitar conexão: {}", e));
            }
        }
    }
}

async fn handle_client(mut client_stream: TcpStream) -> Result<(), Error> {
    let status = get_status();
    client_stream.write_all(format!("HTTP/1.1 101 {}\r\n\r\n", status).as_bytes()).await?;
    
    let mut buffer = vec![0; 4096];
    client_stream.read(&mut buffer).await?;
    client_stream.write_all(b"HTTP/1.1 200 Connection Established\r\nProxy-Agent: RustProxy\r\nConnection: keep-alive\r\nKeep-Alive: timeout=60, max=120\r\n\r\n").await?;
    
    let addr_proxy = identify_protocol(&mut client_stream).await;
    log("INFO", &format!("Conectando ao proxy: {}", addr_proxy));
    
    let server_connect = timeout(Duration::from_secs(10), TcpStream::connect(&addr_proxy)).await;
    let server_stream = match server_connect {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) | Err(e) => {
            log("ERROR", &format!("Erro ao conectar ao proxy: {}", e));
            return Ok(());
        }
    };
    
    tokio::io::copy_bidirectional(&mut client_stream, &mut server_stream).await?;
    Ok(())
}

async fn identify_protocol(stream: &mut TcpStream) -> String {
    match timeout(Duration::from_secs(5), peek_stream(stream)).await {
        Ok(Ok(data)) if data.contains("SSH") || data.is_empty() => "0.0.0.0:22".to_string(),
        Ok(_) => "0.0.0.0:1194".to_string(),
        Err(_) | Ok(Err(_)) => "0.0.0.0:22".to_string(),
    }
}

async fn peek_stream(stream: &TcpStream) -> Result<String, Error> {
    let mut peek_buffer = vec![0; 4096];
    let bytes_peeked = stream.peek(&mut peek_buffer).await?;
    Ok(String::from_utf8_lossy(&peek_buffer[..bytes_peeked]).to_string())
}

fn get_port() -> u16 {
    let args: Vec<String> = env::args().collect();
    args.windows(2)
        .find(|w| w[0] == "--port")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(80)
}

fn get_status() -> String {
    let args: Vec<String> = env::args().collect();
    args.windows(2)
        .find(|w| w[0] == "--status")
        .map(|w| w[1].clone())
        .unwrap_or_else(|| "@RustyManager".to_string())
}

fn log(level: &str, message: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("[{}] [{}] {}", timestamp, level, message);
}
