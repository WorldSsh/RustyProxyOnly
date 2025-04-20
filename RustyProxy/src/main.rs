use std::env;
use std::io::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt, copy_bidirectional};
use tokio::net::{TcpListener, TcpStream};
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
    let status = get_status();

    // Primeiro handshake 101
    client_stream
        .write_all(format!("HTTP/1.1 101 {}\r\n\r\n", status).as_bytes())
        .await?;

    let mut buffer = [0; 1024];
    client_stream.read(&mut buffer).await?;

    // Segundo handshake 101
    client_stream
        .write_all(format!("HTTP/1.1 101 {}\r\n\r\n", status).as_bytes())
        .await?;

    // Resposta 200 opcional
    client_stream
        .write_all(format!("HTTP/1.1 200 {}\r\n\r\n", status).as_bytes())
        .await?;

    let addr_proxy = match timeout(Duration::from_secs(5), peek_stream(&mut client_stream)).await {
        Ok(Ok(data)) if data.contains("SSH") || data.is_empty() => "0.0.0.0:22",
        Ok(_) => "0.0.0.0:1194",
        Err(_) => "0.0.0.0:22",
    };

    let mut server_stream = match TcpStream::connect(addr_proxy).await {
        Ok(stream) => stream,
        Err(_) => {
            eprintln!("Erro ao conectar-se ao servidor proxy em {}", addr_proxy);
            return Ok(());
        }
    };

    // Transfere dados entre cliente e servidor com buffer dinâmico
    let _ = copy_bidirectional(&mut client_stream, &mut server_stream).await;

    Ok(())
}

async fn peek_stream(stream: &TcpStream) -> Result<String, Error> {
    let mut buffer = vec![0; 8192];
    let bytes_peeked = stream.peek(&mut buffer).await?;
    Ok(String::from_utf8_lossy(&buffer[..bytes_peeked]).to_string())
}

fn get_port() -> u16 {
    env::args().nth(2).unwrap_or_else(|| "80".to_string()).parse().unwrap_or(80)
}

fn get_status() -> String {
    env::args().nth(4).unwrap_or_else(|| "@RUSTY PROXY".to_string())
}
