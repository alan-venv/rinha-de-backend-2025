mod robin;

use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::SocketAddr;

const SERVER: Token = Token(0);
const BUFFER_SIZE: usize = 8192;

fn parse_body(input: &[u8]) -> Option<&[u8]> {
    let sep = b"\r\n\r\n";
    input
        .windows(sep.len())
        .position(|w| w == sep)
        .map(|i| &input[i + sep.len()..])
}

fn main() -> std::io::Result<()> {
    let addr: SocketAddr = "0.0.0.0:9999".parse().unwrap();
    let mut listener = TcpListener::bind(addr)?;
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(1024);
    let mut connections: HashMap<Token, TcpStream> = HashMap::new();
    let mut buffers: HashMap<Token, Vec<u8>> = HashMap::new();
    let mut unique_token = Token(SERVER.0 + 1);

    poll.registry()
        .register(&mut listener, SERVER, Interest::READABLE)?;

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                SERVER => loop {
                    match listener.accept() {
                        Ok((mut stream, _addr)) => {
                            let token = unique_token;
                            unique_token = Token(unique_token.0 + 1);
                            poll.registry()
                                .register(&mut stream, token, Interest::READABLE)?;
                            connections.insert(token, stream);
                            buffers.insert(token, Vec::with_capacity(BUFFER_SIZE));
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                        Err(e) => return Err(e),
                    }
                },
                token => {
                    let mut remove = false;

                    if let Some(stream) = connections.get_mut(&token) {
                        let buf = buffers.get_mut(&token).unwrap();
                        let mut tmp = [0u8; BUFFER_SIZE];
                        match stream.read(&mut tmp) {
                            Ok(0) => remove = true,
                            Ok(n) => {
                                buf.extend_from_slice(&tmp[..n]);
                                if let Some(_body) = parse_body(buf) {
                                    println!("{:?}", _body);
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
                                        "/payments-summary" => { if let Some(data) = query {
                                            println!("{:?}", data);
                                        }
                                        b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"default\":{\"totalRequests\":0,\"totalAmount\":-0.0},\"fallback\":{\"totalRequests\":0,\"totalAmount\":-0.0}}" }
                                        "/payments" => { b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nOK" }
                                        "/purge-payments" => { b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nqueued" }
                                        _ => { b"HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\n\r\nNot Found" }
                                    };
                                    let _ = stream.write_all(response);
                                    remove = true;
                                }
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                            Err(_) => remove = true,
                        }
                    }

                    if remove {
                        if let Some(mut stream) = connections.remove(&token) {
                            let _ = poll.registry().deregister(&mut stream);
                        }
                        buffers.remove(&token);
                    }
                }
            }
        }
    }
}
