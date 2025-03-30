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
    let mut buffer = [0; 4096];
    let bytes_read = match client_stream.read(&mut buffer).await {
        Ok(0) => return Ok(()),
        Ok(n) => n,
        Err(_) => return Ok(()),
    };

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let mut lines = request.lines();
    let first_line = lines.next().unwrap_or("");

    let method = first_line.split_whitespace().next().unwrap_or("");

    let response = match method {
        "GET" => "HTTP/1.1 200 OK\r\n\r\nGET recebido",
        "POST" => "HTTP/1.1 200 OK\r\n\r\nPOST recebido",
        "PUT" => "HTTP/1.1 200 OK\r\n\r\nPUT recebido",
        "PATCH" => "HTTP/1.1 200 OK\r\n\r\nPATCH recebido",
        _ => {
            let addr_proxy = detect_protocol(&mut client_stream).await;
            return proxy_traffic(client_stream, addr_proxy).await;
        }
    };

    client_stream.write_all(response.as_bytes()).await?;
    Ok(())
}

async fn detect_protocol(stream: &mut TcpStream) -> &str {
    let mut buffer = [0; 8192];
    match timeout(Duration::from_secs(1), stream.peek(&mut buffer)).await {
        Ok(Ok(bytes_peeked)) if bytes_peeked > 0 => {
            let data = String::from_utf8_lossy(&buffer[..bytes_peeked]);
            if data.contains("SSH") { "0.0.0.0:22" } else { "0.0.0.0:1194" }
        }
        _ => "0.0.0.0:22",
    }
}

async fn proxy_traffic(mut client_stream: TcpStream, addr_proxy: &str) -> Result<(), Error> {
    let server_stream = match TcpStream::connect(addr_proxy).await {
        Ok(stream) => stream,
        Err(_) => {
            eprintln!("Erro ao conectar-se ao servidor proxy em {}", addr_proxy);
            return Ok(());
        }
    };

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
    let mut buffer = [0; 8192];
    loop {
        let bytes_read = {
            let mut reader = read_stream.lock().await;
            match reader.read(&mut buffer).await {
                Ok(0) => break, 
                Ok(n) => n,
                Err(_) => break,
            }
        };

        let mut writer = write_stream.lock().await;
        if writer.write_all(&buffer[..bytes_read]).await.is_err() {
            break;
        }
    }
    Ok(())
}

fn get_port() -> u16 {
    env::args().nth(2).unwrap_or_else(|| "80".to_string()).parse().unwrap_or(80)
}
