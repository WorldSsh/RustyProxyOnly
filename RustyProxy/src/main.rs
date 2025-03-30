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

    match TcpStream::connect(addr_proxy).await {
        Ok(server_stream) => relay_data(client_stream, server_stream).await?,
        Err(_) => eprintln!("Erro ao conectar ao proxy: {}", addr_proxy),
    }

    Ok(())
}

async fn send_response(client_stream: &mut TcpStream, status: String) -> Result<(), Error> {
    client_stream
        .write_all(format!("HTTP/1.1 200 {}\r\n\r\n", status).as_bytes())
        .await
}

async fn determine_proxy_address(client_stream: &mut TcpStream) -> &'static str {
    let result = timeout(Duration::from_secs(1), peek_stream(client_stream)).await;

    match result {
        Ok(Ok(data)) if data.contains("SSH") || data.is_empty() => "0.0.0.0:22",
        Ok(_) => "0.0.0.0:1194",
        _ => "0.0.0.0:22",
    }
}

async fn relay_data(client_stream: TcpStream, server_stream: TcpStream) -> Result<(), Error> {
    let (client_read, client_write) = client_stream.into_split();
    let (server_read, server_write) = server_stream.into_split();

    let client_read = Arc::new(Mutex::new(client_read));
    let client_write = Arc::new(Mutex::new(client_write));
    let server_read = Arc::new(Mutex::new(server_read));
    let server_write = Arc::new(Mutex::new(server_write));

    let client_to_server = transfer_data(client_read, server_write);
    let server_to_client = transfer_data(server_read, client_write);

    tokio::try_join!(client_to_server, server_to_client)?;

    Ok(())
}

async fn transfer_data(
    read_stream: Arc<Mutex<tokio::net::tcp::OwnedReadHalf>>,
    write_stream: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
) -> Result<(), Error> {
    let mut buffer = [0; 8192];
    loop {
        let bytes_read = {
            let mut read_guard = read_stream.lock().await;
            read_guard.read(&mut buffer).await?
        };

        if bytes_read == 0 {
            break;
        }

        let mut write_guard = write_stream.lock().await;
        write_guard.write_all(&buffer[..bytes_read]).await?;
    }

    Ok(())
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
