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

async fn handle_client(mut client_stream: TcpStream) -> Result<(), Error> {
    let mut addr_proxy = "0.0.0.0:22";

    let result = timeout(Duration::from_secs(15), peek_stream(&client_stream)).await;
    let data = match result {
        Ok(Ok(data)) => data,
        Ok(Err(e)) => {
            eprintln!("Erro ao espiar stream: {}", e);
            return Err(e);
        }
        Err(_) => {
            eprintln!("Timeout ao espiar stream.");
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
    let mut buffer = vec![0; 8192]; // Começa com 8KB em vez de 1KB
let max_buffer_size = 128 * 1024; // Pode aumentar até 128KB

    loop {
        let bytes_read = {
            let mut read_guard = read_stream.lock().await;
            read_guard.read(&mut buffer).await?
        };

        if bytes_read == 0 {
            break;
        }

        if bytes_read == buffer.len() && buffer.len() < max_buffer_size {
            buffer.resize((buffer.len() * 2).min(max_buffer_size), 0);
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
        .skip(1)
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or(80)
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
            Err(e) => eprintln!("Erro ao aceitar conexão: {}", e),
        }
    }
}
