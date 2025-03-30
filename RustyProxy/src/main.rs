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
    
    while let Ok((client_stream, addr)) = listener.accept().await {
        tokio::spawn(async move {
            if let Err(e) = handle_client(client_stream).await {
                eprintln!("Erro ao processar cliente {}: {}", addr, e);
            }
        });
    }
    
    Ok(())
}

async fn handle_client(mut client_stream: TcpStream) -> Result<(), Error> {
    let status = get_status();
    client_stream.write_all(format!("HTTP/1.1 101 {}

", status).as_bytes()).await?;
    
    let mut buffer = vec![0; 1024];
    client_stream.read(&mut buffer).await?;
    client_stream.write_all(format!("HTTP/1.1 200 {}

", status).as_bytes()).await?;
    
    let addr_proxy = detect_protocol(&mut client_stream).await.unwrap_or("0.0.0.0:22");
    
    let server_stream = match TcpStream::connect(addr_proxy).await {
        Ok(stream) => stream,
        Err(_) => {
            eprintln!("Erro ao conectar ao proxy");
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
    let mut buffer = vec![0; 8192];
    
    loop {
        let bytes_read = {
            let mut read_guard = read_stream.lock().await;
            match read_guard.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            }
        };
        
        let mut write_guard = write_stream.lock().await;
        write_guard.write_all(&buffer[..bytes_read]).await?;
    }
    
    Ok(())
}

async fn detect_protocol(stream: &mut TcpStream) -> Result<&'static str, Error> {
    let mut peek_buffer = vec![0; 8192];
    match timeout(Duration::from_secs(1), stream.peek(&mut peek_buffer)).await {
        Ok(Ok(bytes_peeked)) => {
            let data = &peek_buffer[..bytes_peeked];
            let data_str = String::from_utf8_lossy(data);
            if data_str.contains("SSH") || data_str.is_empty() {
                Ok("0.0.0.0:22")
            } else {
                Ok("0.0.0.0:1194")
            }
        }
        _ => Ok("0.0.0.0:22"),
    }
}

fn get_port() -> u16 {
    env::args().skip(1).collect::<Vec<String>>().windows(2).find_map(|args| {
        if args[0] == "--port" { args[1].parse().ok() } else { None }
    }).unwrap_or(80)
}

fn get_status() -> String {
    env::args().skip(1).collect::<Vec<String>>().windows(2).find_map(|args| {
        if args[0] == "--status" { Some(args[1].clone()) } else { None }
    }).unwrap_or_else(|| "@RustyManager".to_string())
}
