use std::{io::Result, net::SocketAddr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const BUFFER_SIZE: usize = 1024;
const R200: &[u8] = b"HTTP/1.1 200 OK\r\n\r\n";
const R202: &[u8] = b"HTTP/1.1 202 Accepted\r\n\r\n";
const R404: &[u8] = b"HTTP/1.1 404 Not Found\r\n\r\n";

fn parse_body(input: &[u8]) -> Option<&[u8]> {
    input
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| &input[i + 4..])
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr: SocketAddr = "0.0.0.0:9999".parse().unwrap();
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (mut socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            let mut buf = vec![0u8; BUFFER_SIZE];
            match socket.read(&mut buf).await {
                Ok(n) if n == 0 => return,
                Ok(n) => {
                    let buf = &buf[..n];
                    if let Some(body) = parse_body(buf) {
                        println!("{:?}", body);
                        let route = std::str::from_utf8(buf)
                            .ok()
                            .and_then(|s| s.lines().next())
                            .unwrap_or("");

                        let (path, query) = route
                            .split_whitespace()
                            .nth(1)
                            .map(|full| match full.find('?') {
                                Some(i) => (&full[..i], Some(&full[i + 1..])),
                                None => (full, None),
                            })
                            .unwrap_or(("/", None));

                        let response: &[u8] = match path {
                            "/payments-summary" => {
                                if let Some(q) = query {
                                    println!("{q}");
                                }
                                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"default\":{\"totalRequests\":0,\"totalAmount\":-0.0},\"fallback\":{\"totalRequests\":0,\"totalAmount\":-0.0}}"
                            }
                            "/payments" => R202,
                            "/purge-payments" => R200,
                            _ => R404,
                        };

                        let _ = socket.write_all(response).await;
                    }
                }
                Err(_) => {}
            }
        });
    }
}
