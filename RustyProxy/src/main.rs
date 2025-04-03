use std::env;
use std::io::Error;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
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

    let mut addr_proxy = "0.0.0.0:22";
    let result = timeout(Duration::from_secs(8), peek_stream(&mut client_stream)).await
        .unwrap_or_else(|_| Ok(String::new()));

    let addr_proxy = match result {
    Ok(data) if data.contains("SSH") || data.is_empty() => "0.0.0.0:22",
    Ok(_) => "0.0.0.0:1194",
    Err(_) => {
        println!("Erro ao identificar o protocolo, redirecionando para SSH por padrão.");
        "0.0.0.0:22"
    }
};

    let server_stream = match TcpStream::connect(addr_proxy).await {
    Ok(stream) => stream,
    Err(e) => {
        println!("Erro ao conectar ao proxy {}: {}", addr_proxy, e);
        return Err(e);
    }
};

    // Transfere dados bidirecionalmente
    tokio::io::copy_bidirectional(&mut client_stream, &mut server_stream).await?;

    Ok(())
}

async fn peek_stream(stream: &TcpStream) -> Result<String, Error> {
    let mut peek_buffer = vec![0; 4096];
    let bytes_peeked = stream.peek(&mut peek_buffer).await?;
    let data = &peek_buffer[..bytes_peeked];
    let data_str = String::from_utf8_lossy(data);
    Ok(data_str.to_string())
}


fn get_port() -> u16 {
    let args: Vec<String> = env::args().collect();
    let mut port = 80;

    for i in 1..args.len() {
        if args[i] == "--port" {
            if i + 1 < args.len() {
                port = args[i + 1].parse().unwrap_or(80);
            }
        }
    }

    port
}

fn get_status() -> String {
    let args: Vec<String> = env::args().collect();
    let mut status = String::from("@RustyManager");

    for i in 1..args.len() {
        if args[i] == "--status" {
            if i + 1 < args.len() {
                status = args[i + 1].clone();
            }
        }
    }

    status
}
