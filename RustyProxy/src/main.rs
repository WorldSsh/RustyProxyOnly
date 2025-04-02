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
    
    let mut buffer = vec![0; 4096];
    client_stream.read(&mut buffer).await?;
    client_stream
        .write_all(format!("HTTP/1.1 200 Conexão Estabelecida {}\r\n\r\n", status).as_bytes())
        .await?;
    
    let result = match timeout(Duration::from_secs(8), peek_stream(&mut client_stream)).await {
        Ok(Ok(data)) => data,
        Ok(Err(e)) => {
            error!("Erro ao analisar fluxo de entrada: {}", e);
            return Err(e);
        }
        Err(_) => {
            error!("Tempo limite excedido ao analisar fluxo de entrada.");
            return Err(Error::new(std::io::ErrorKind::TimedOut, "Tempo limite excedido"));
        }
    };

    let addr_proxy = if !result.is_empty() && result.contains("SSH") {
        "0.0.0.0:22"
    } else {
        "0.0.0.0:1194"
    };

    let mut server_stream = TcpStream::connect(addr_proxy).await?;
    
    let (mut client_read, mut client_write) = client_stream.into_split();
    let (mut server_read, mut server_write) = server_stream.into_split();
    
    let client_to_server = tokio::spawn(async move {
        transfer_data(&mut client_read, &mut server_write).await
    });
    let server_to_client = tokio::spawn(async move {
        transfer_data(&mut server_read, &mut client_write).await
    });

    tokio::try_join!(client_to_server, server_to_client)?;
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
