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
    let status = get_status();
    client_stream
        .write_all(format!("HTTP/1.1 101 {}\r\n\r\n", status).as_bytes())
        .await?;

    let mut buffer = [0; 4096];
    client_stream.read(&mut buffer).await?;
    client_stream
        .write_all(format!("HTTP/1.1 200 {}\r\n\r\n", status).as_bytes())
        .await?;       

    let addr_proxy = match timeout(Duration::from_secs(10), peek_stream(&mut client_stream)).await {
    Ok(Ok(data)) if data.contains("SSH") || data.is_empty() => {
        println!("Conectando a servidor SSH");
        "0.0.0.0:22"
    },
    Ok(_) => {
        println!("Conectando a servidor OpenVPN");
        "0.0.0.0:1194"
    },
    Err(_) => {
        println!("Timeout na detecção do protocolo, assumindo SSH");
        "0.0.0.0:22"
    },
};

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
        transfer_data(client_read.clone(), server_write.clone()),
        transfer_data(server_read.clone(), client_write.clone())
    )?;

    Ok(())
}

async fn transfer_data(
    read_stream: Arc<Mutex<tokio::net::tcp::OwnedReadHalf>>,
    write_stream: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
) -> Result<(), Error> {
    let mut buffer = [0; 4096];
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
        match writer.write_all(&buffer[..bytes_read]).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Erro ao escrever no socket: {}", e);
                break;
            }
        }
    }
    Ok(())
}

async fn peek_stream(stream: &TcpStream) -> Result<String, Error> {
    let mut buffer = vec![0; 4096];
    let bytes_peeked = match stream.peek(&mut buffer).await {
        Ok(n) => n,
        Err(_) => return Ok("".to_string()), // Retorna string vazia ao invés de falhar
    };
    Ok(String::from_utf8_lossy(&buffer[..bytes_peeked]).to_string())
}

fn get_status() -> String {
    env::args().nth(4).unwrap_or_else(|| "@RustyManager".to_string())
}
    
