use std::env;
use std::io::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};
use tracing::{info, error, debug};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    let port = get_port();
    let listener = TcpListener::bind(format!("[::]:{}", port)).await?;
    info!("Iniciando serviço na porta: {}", port);
    start_http(listener).await;
    Ok(())
}

async fn start_http(listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((client_stream, addr)) => {
                tokio::spawn(async move {
                    if let Err(e) = handle_client(client_stream).await {
                        error!("Erro ao processar cliente {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                error!("Erro ao aceitar conexão: {}", e);
            }
        }
    }
}

async fn handle_client(mut client_stream: TcpStream) -> Result<(), Error> {
    let status = get_status();
    client_stream
        .write_all(format!("HTTP/1.1 101 {}\r\n\r\n", status).as_bytes())
        .await?;
    
    let result = peek_stream(&client_stream).await?;

    if result.starts_with("SSH-") {
        info!("Conexão SSH detectada");
        let addr_proxy = "0.0.0.0:22"; // Proxy para servidor SSH
        proxy_connection(client_stream, addr_proxy).await?;
    } else if result.contains("GET ") || result.contains("Upgrade: websocket") {
        info!("Conexão WebSocket/HTTP detectada");
        client_stream.write_all(b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n").await?;
    } else {
        info!("Conexão OpenVPN ou desconhecida, encaminhando para OpenVPN");
        let addr_proxy = "0.0.0.0:1194"; // Proxy para OpenVPN
        proxy_connection(client_stream, addr_proxy).await?;
    }
    
    Ok(())
}

async fn transfer_data(
    read_stream: &mut tokio::net::tcp::OwnedReadHalf,
    write_stream: &mut tokio::net::tcp::OwnedWriteHalf,
) -> Result<(), Error> {
    let mut buffer = vec![0; 1024];
    let max_buffer_size = 64 * 1024;

    loop {
        let bytes_read = read_stream.read(&mut buffer).await?;
        
        if bytes_read == 0 {
            break;
        }

        debug!("Recebido: {:?}", String::from_utf8_lossy(&buffer[..bytes_read]));
        
        write_stream.write_all(&buffer[..bytes_read]).await?;
    }
    Ok(())
}

async fn peek_stream(stream: &TcpStream) -> Result<String, Error> {
    let mut peek_buffer = vec![0; 4096];
    let bytes_peeked = stream.peek(&mut peek_buffer).await?;
    let data = &peek_buffer[..bytes_peeked];
    Ok(String::from_utf8_lossy(data).to_string())
}

fn get_port() -> u16 {
    let args: Vec<String> = env::args().collect();
    if let Some(index) = args.iter().position(|arg| arg == "--port") {
        if let Some(port_str) = args.get(index + 1) {
            if let Ok(port) = port_str.parse::<u16>() {
                return port;
            } else {
                error!("Porta inválida fornecida: {}. Usando porta padrão 80.", port_str);
            }
        }
    }
    80
}

fn get_status() -> String {
    let args: Vec<String> = env::args().collect();
    if let Some(index) = args.iter().position(|arg| arg == "--status") {
        if let Some(status) = args.get(index + 1) {
            return status.clone();
        }
    }
    "@RustyManager".to_string()
}
