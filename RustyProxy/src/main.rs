use std::env;
use std::io::Error;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::{time::{Duration}};
use tokio::time::timeout;
use tracing::{info, error};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO) // Define nível mínimo de logs (pode ser DEBUG, TRACE)
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

    let addr_proxy;
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

    if !result.is_empty() && result.contains("SSH") {
    addr_proxy = "0.0.0.0:22";
} else {
    addr_proxy = "0.0.0.0:1194";
}

    let server_stream = match TcpStream::connect(addr_proxy).await {
    Ok(stream) => stream,
    Err(e) => {
        error!("Erro ao conectar ao proxy {}: {}", addr_proxy, e);
        return Err(e);
    }
};

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
    let mut buffer = vec![0; 1024]; // Começa pequeno
    let max_buffer_size = 64 * 1024; // Define um tamanho máximo razoável (64KB)

    loop {
        llet bytes_read = {
    let mut read_guard = read_stream.lock().await;
    read_guard.read(&mut buffer).await?
};

debug!("Recebido: {:?}", String::from_utf8_lossy(&buffer[..bytes_read]));

        if bytes_read == 0 {
            break;
        }

        // buffer em 64kb mas não ultrapassa o limite máximo
        if bytes_read == buffer.len() && buffer.len() < max_buffer_size {
    buffer.resize((buffer.len() * 2).min(max_buffer_size), 0);
} else if bytes_read < buffer.len() / 2 && buffer.len() > 1024 {
    buffer.resize((buffer.len() / 2).max(1024), 0);
}

        let mut write_guard = write_stream.lock().await;
        write_guard.write_all(&buffer[..bytes_read]).await?;
    }

    Ok(())
}

async fn peek_stream(stream: &TcpStream) -> Result<String, Error> {
    let mut peek_buffer = vec![0; 4096];
    match stream.peek(&mut peek_buffer).await {
        Ok(bytes_peeked) => {
            let data = &peek_buffer[..bytes_peeked];
            Ok(String::from_utf8_lossy(data).to_string())
        }
        Err(e) => {
            error!("Falha ao analisar fluxo de entrada: {}", e);
            Ok(String::new()) // Retorna string vazia ao invés de erro crítico
        }
    }
}
    let mut peek_buffer = vec![0; 4096];
    match stream.peek(&mut peek_buffer).await {
        Ok(bytes_peeked) => {
            let data = &peek_buffer[..bytes_peeked];
            Ok(String::from_utf8_lossy(data).to_string())
        }
        Err(e) => {
            error!("Falha ao analisar fluxo de entrada: {}", e);
            Ok(String::new()) // Retorna string vazia ao invés de erro crítico
        }
    }
}
    let mut peek_buffer = vec![0; 4096];
    match stream.peek(&mut peek_buffer).await {
        Ok(bytes_peeked) => {
            let data = &peek_buffer[..bytes_peeked];
            Ok(String::from_utf8_lossy(data).to_string())
        }
        Err(e) => {
            error!("Falha ao analisar fluxo de entrada: {}", e);
            Ok(String::new()) // Retorna string vazia ao invés de erro crítico
        }
    }
}
    let mut peek_buffer = vec![0; 4096];
    let bytes_peeked = stream.peek(&mut peek_buffer).await?;
    let data = &peek_buffer[..bytes_peeked];
    let data_str = String::from_utf8_lossy(data);
    Ok(data_str.to_string())
}


fn get_port() -> u16 {
    let mut args = env::args();
    while let Some(arg) = args.next() {
        if arg == "--port" {
            if let Some(port_str) = args.next() {
                if let Ok(port) = port_str.parse::<u16>() {
    return port;
} else {
    error!("Porta inválida fornecida: {}. Usando porta padrão 80.", port_str);
    return 80;
}
            }
        }
    }
    80
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
