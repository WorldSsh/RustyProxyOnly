use clap::Parser;
use log::{error, info};
use serde::Deserialize;
use std::io;
use tokio::io::{copy_bidirectional, AsyncReadExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long, default_value_t = 80)]
    port: u16,
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

#[derive(Deserialize)]
struct Config {
    port: u16,
    status: String,
    backends: Vec<Backend>,
}

#[derive(Deserialize)]
struct Backend {
    contains: String,
    target: String,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("🚀 Iniciando Rusty Proxy v2.0");

    let args = Args::parse();

    let config: Config = match std::fs::read_to_string(&args.config) {
        Ok(content) => toml::from_str(&content).unwrap_or_else(|e| {
            error!("Erro ao ler config.toml: {}. Usando padrão.", e);
            Config {
                port: args.port,
                status: "@RUSTY PROXY".to_string(),
                backends: vec![Backend { contains: "".to_string(), target: "0.0.0.0:1194".to_string() }],
            }
        }),
        Err(_) => {
            info!("config.toml não encontrado. Criando padrão...");
            let default = Config {
                port: args.port,
                status: "@RUSTY PROXY".to_string(),
                backends: vec![
                    Backend { contains: "SSH".to_string(), target: "0.0.0.0:22".to_string() },
                    Backend { contains: "".to_string(), target: "0.0.0.0:1194".to_string() },
                ],
            };
            let _ = std::fs::write(&args.config, toml::to_string_pretty(&default).unwrap());
            default
        }
    };

    let listener = TcpListener::bind(format!("[::]:{}", config.port))
        .await
        .expect("Falha ao bind na porta");

    info!("✅ Servidor rodando na porta {} | Status: {}", config.port, config.status);
    info!("📌 Backends configurados: {}", config.backends.len());

    start_proxy(listener, config).await;
}

async fn start_proxy(listener: TcpListener, config: Config) {
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("Nova conexão de {}", addr);
                let config_clone = config.backends.clone();
                let status = config.status.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, status, config_clone).await {
                        error!("Erro com {}: {}", addr, e);
                    }
                });
            }
            Err(e) => error!("Erro ao aceitar conexão: {}", e),
        }
    }
}

async fn handle_client(mut client: TcpStream, status: String, backends: Vec<Backend>) -> io::Result<()> {
    client.write_all(format!("HTTP/1.1 101 {}\r\n\r\n", status).as_bytes()).await?;
    let _ = client.read(&mut [0; 1024]).await;
    client.write_all(format!("HTTP/1.1 101 {}\r\n\r\n", status).as_bytes()).await?;
    client.write_all(format!("HTTP/1.1 200 {}\r\n\r\n", status).as_bytes()).await?;

    let peeked = timeout(Duration::from_secs(5), peek_data(&mut client))
        .await
        .unwrap_or(Ok(String::new()))?;

    let target = backends
        .iter()
        .find(|b| peeked.contains(&b.contains))
        .map(|b| b.target.as_str())
        .unwrap_or("0.0.0.0:1194");

    info!("🔀 Encaminhando para {} (detectado: {})", target, if peeked.is_empty() { "vazio" } else { &peeked[0..peeked.len().min(30)] });

    let mut server = match TcpStream::connect(target).await {
        Ok(s) => s,
        Err(e) => {
            error!("Falha ao conectar no backend {}: {}", target, e);
            return Ok(());
        }
    };

    let _ = copy_bidirectional(&mut client, &mut server).await;
    info!("Conexão encerrada");
    Ok(())
}

async fn peek_data(stream: &mut TcpStream) -> io::Result<String> {
    let mut buf = vec![0u8; 8192];
    let n = stream.peek(&mut buf).await?;
    Ok(String::from_utf8_lossy(&buf[..n]).to_string())
}
