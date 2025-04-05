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
            Err(e) => println!("Erro ao aceitar conexão: {}", e),
        }
    }
}

async fn handle_client(mut client_stream: TcpStream) -> Result<(), Error> {
    let status = get_status();
    client_stream
        .write_all(format!("HTTP/1.1 101 {}\r\n\r\n", status).as_bytes())
        .await?;

    client_stream
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\
                     Proxy-Agent: RustProxy\r\n\
                     Connection: keep-alive\r\n\
                     Keep-Alive: timeout=300, max=600\r\n\r\n")
        .await?;

    let _ = client_stream.read(&mut vec![0; 4096]).await?;
    let mut addr_proxy = "0.0.0.0:22";
    
    let result = timeout(Duration::from_secs(15), peek_stream(&client_stream)).await;
    let data = match result {
        Ok(Ok(data)) => data,
        Ok(Err(e)) => {
            println!("Erro ao espiar stream: {}", e);
            return Err(e);
        }
        Err(_) => {
            println!("Timeout ao espiar stream.");
            String::new()
        }
    };

    if !(data.contains("SSH") || data.is_empty()) {
        addr_proxy = "0.0.0.0:1194";
    }

    let server_stream = TcpStream::connect(addr_proxy).await?;
    let (client_read, client_write) = client_stream.into_split();
    let (server_read, server_write) = server_stream.into_split();

    let client_read = Arc::new(Mutex::new(client_read));
    let client_write = Arc::new(Mutex::new(client_write));
    let server_read = Arc::new(Mutex::new(server_read));
    let server_write = Arc::new(Mutex::new(server_write));

    tokio::try_join!(
        transfer_data(client_read, server_write),
        transfer_data(server_read, client_write)
    )?;

    Ok(())
}

async fn transfer_data(
    read_stream: Arc<Mutex<tokio::net::tcp::OwnedReadHalf>>,
    write_stream: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
) -> Result<(), Error> {
    let mut buffer = vec![0; 4096]; // 4KB inicial
    let max_buffer_size = 128 * 1024; // 128KB máximo

    loop {
        let bytes_read = {
            let mut read_guard = read_stream.lock().await;
            read_guard.read(&mut buffer).await?
        };

        if bytes_read == 0 {
            break;
        }

        if bytes_read == buffer.len() && buffer.len() < max_buffer_size {
            let new_size = (buffer.len() * 2).min(max_buffer_size);
            println!("Aumentando buffer de {} para {}", buffer.len(), new_size);
            buffer.resize(new_size, 0);
        }

        let mut write_guard = write_stream.lock().await;
        write_guard.write_all(&buffer[..bytes_read]).await?;
    }

    Ok(())
}

async fn peek_stream(stream: &TcpStream) -> Result<String, Error> {
    let mut peek_buffer = vec![0; 4096];
    let bytes_peeked = stream.peek(&mut peek_buffer).await?;
    let data = String::from_utf8_lossy(&peek_buffer[..bytes_peeked]).to_string();

    println!("Peeked Data: {}", data); // <-- Adicione este log para depuração
    Ok(data)
}

fn get_port() -> u16 {
    env::args()
        .skip_while(|arg| arg != "--port")
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(80)
}

fn get_status() -> String {
    env::args()
        .skip_while(|arg| arg != "--status")
        .nth(1)
        .unwrap_or_else(|| "@RustyManager".to_string())
}
