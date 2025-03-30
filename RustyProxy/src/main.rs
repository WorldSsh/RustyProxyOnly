use std::env;
use std::io::Error;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

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
    let is_tunnel = method == "CONNECT";

    if is_tunnel {
        if let Some(addr) = first_line.split_whitespace().nth(1) {
            return handle_tunnel(client_stream, addr).await;
        } else {
            return Ok(());
        }
    }

    let response = match method {
        "GET" => "HTTP/1.1 200 OK\r\n\r\nGET recebido",
        "POST" => "HTTP/1.1 200 OK\r\n\r\nPOST recebido",
        "PUT" => "HTTP/1.1 200 OK\r\n\r\nPUT recebido",
        "PATCH" => "HTTP/1.1 200 OK\r\n\r\nPATCH recebido",
        _ => {
            let addr_proxy = "0.0.0.0:1194";
            return proxy_traffic(client_stream, addr_proxy).await;
        }
    };

    client_stream.write_all(response.as_bytes()).await?;
    Ok(())
}

async fn handle_tunnel(mut client_stream: TcpStream, addr: &str) -> Result<(), Error> {
    println!("Estabelecendo túnel para {}", addr);

    let mut server_stream = match TcpStream::connect(addr).await {
        Ok(stream) => stream,
        Err(_) => {
            eprintln!("Falha ao conectar-se ao destino: {}", addr);
            return Ok(());
        }
    };

    client_stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;

    bidirectional_transfer(client_stream, server_stream).await;

    Ok(())
}

async fn proxy_traffic(client_stream: TcpStream, addr_proxy: &str) -> Result<(), Error> {
    let server_stream = match TcpStream::connect(addr_proxy).await {
        Ok(stream) => stream,
        Err(_) => {
            eprintln!("Erro ao conectar-se ao servidor proxy em {}", addr_proxy);
            return Ok(());
        }
    };

    bidirectional_transfer(client_stream, server_stream).await;

    Ok(())
}

async fn bidirectional_transfer(client_stream: TcpStream, server_stream: TcpStream) {
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
