use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::thread;
use std::time::{Duration, Instant};

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 4096];

    // Aguarda até 60s para detectar o tipo de conexão
    stream.set_read_timeout(Some(Duration::from_secs(60))).unwrap();
    match stream.peek(&mut buffer) {
        Ok(n) if n > 0 => {
            let data = &buffer[..n];
            if is_http(data) {
                println!("[Proxy] HTTP Detectado");
                handle_http_proxy(stream);
            } else {
                println!("[Proxy] Provavelmente SOCKS ou SSH");
                handle_tcp_proxy(stream);
            }
        },
        _ => {
            println!("[Proxy] Nada recebido, mantendo conexão...");
        }
    }
}

fn is_http(data: &[u8]) -> bool {
    data.starts_with(b"GET") || data.starts_with(b"POST") || data.starts_with(b"CONNECT")
}

fn handle_http_proxy(mut client: TcpStream) {
    let mut buffer = vec![0u8; 8192];
    match client.read(&mut buffer) {
        Ok(n) if n > 0 => {
            let request = String::from_utf8_lossy(&buffer[..n]);
            if let Some(host_line) = request.lines().find(|line| line.to_lowercase().starts_with("host:")) {
                let host = host_line.split_whitespace().nth(1).unwrap_or("");
                let mut parts = host.split(':');
                let hostname = parts.next().unwrap_or("");
                let port = parts.next().unwrap_or("80");
                let address = format!("{}:{}", hostname, port);

                match TcpStream::connect(&address) {
                    Ok(mut remote) => {
                        client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").unwrap();

                        let mut c = client.try_clone().unwrap();
                        let mut r = remote.try_clone().unwrap();

                        thread::spawn(move || {
                            let _ = std::io::copy(&mut r, &mut c);
                        });

                        let _ = std::io::copy(&mut client, &mut remote);
                    }
                    Err(e) => println!("Erro conectando ao destino: {}", e),
                }
            }
        }
        _ => println!("[Proxy] Nenhum dado HTTP válido recebido"),
    }
}

fn handle_tcp_proxy(mut client: TcpStream) {
    match TcpStream::connect("127.0.0.1:22") {
        Ok(mut remote) => {
            let mut c = client.try_clone().unwrap();
            let mut r = remote.try_clone().unwrap();

            thread::spawn(move || {
                let _ = std::io::copy(&mut r, &mut c);
            });

            let _ = std::io::copy(&mut client, &mut remote);
        }
        Err(e) => println!("Erro conectando ao SSH local: {}", e),
    }
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:8080").unwrap();
    println!("[Proxy] Servidor escutando na porta 8080...");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                stream.set_nodelay(true).unwrap();
                thread::spawn(move || handle_client(stream));
            }
            Err(e) => println!("Erro na conexão: {}", e),
        }
    }
}
