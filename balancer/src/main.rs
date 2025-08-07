mod robin;

use std::collections::HashMap;
use std::io::ErrorKind::WouldBlock;
use std::io::{Read, Result, Write};
use std::net::SocketAddr;

use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};

const SERVER: Token = Token(0);
const READABLE: Interest = Interest::READABLE;
const BUFFER_SIZE: usize = 1024;
const R200: &[u8] = b"HTTP/1.1 200 OK\r\n\r\n";
const R200C: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"default\":{\"totalRequests\":0,\"totalAmount\":-0.0},\"fallback\":{\"totalRequests\":0,\"totalAmount\":-0.0}}";
const R202: &[u8] = b"HTTP/1.1 202 Accepted\r\n\r\n";
const R404: &[u8] = b"HTTP/1.1 404 Not Found\r\n\r\n";

fn parse_body(input: &[u8]) -> Option<&[u8]> {
    let sep = b"\r\n\r\n";
    input
        .windows(sep.len())
        .position(|w| w == sep)
        .map(|i| &input[i + sep.len()..])
}

fn extract_route(buffer: &[u8]) -> &str {
    return std::str::from_utf8(buffer)
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("");
}

fn extract_path_and_query(route: &str) -> (&str, Option<&str>) {
    return route
        .split_whitespace()
        .nth(1)
        .map(|full| match full.find('?') {
            Some(i) => (&full[..i], Some(&full[i + 1..])),
            None => (full, None),
        })
        .unwrap_or(("/", None));
}

fn handle_client_event(stream: &mut TcpStream, buffer: &mut Vec<u8>) -> bool {
    let mut tmp = [0u8; BUFFER_SIZE];
    match stream.read(&mut tmp) {
        Ok(0) => return true,
        Ok(n) => {
            buffer.extend_from_slice(&tmp[..n]);
            if let Some(body) = parse_body(buffer) {
                println!("{:?}", body);
                let route = extract_route(buffer);
                let (path, query) = extract_path_and_query(route);

                let response: &[u8] = match path {
                    "/payments-summary" => {
                        if let Some(data) = query {
                            println!("{:?}", data);
                        }
                        R200C
                    }
                    "/payments" => R202,
                    "/purge-payments" => R200,
                    _ => R404,
                };
                let _ = stream.write_all(response);
                return true;
            }
        }
        Err(ref e) if e.kind() == WouldBlock => {}
        Err(_) => return true,
    }
    false
}

fn main() -> Result<()> {
    let addr: SocketAddr = "0.0.0.0:9999".parse().unwrap();
    let mut listener = TcpListener::bind(addr)?;
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(1024);
    let mut connections: HashMap<Token, TcpStream> = HashMap::new();
    let mut buffers: HashMap<Token, Vec<u8>> = HashMap::new();
    let mut unique_token = Token(SERVER.0 + 1);

    poll.registry().register(&mut listener, SERVER, READABLE)?;

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                SERVER => loop {
                    match listener.accept() {
                        Ok((mut stream, _addr)) => {
                            let token = unique_token;
                            unique_token = Token(unique_token.0 + 1);
                            poll.registry().register(&mut stream, token, READABLE)?;
                            connections.insert(token, stream);
                            buffers.insert(token, Vec::with_capacity(BUFFER_SIZE));
                        }
                        Err(ref e) if e.kind() == WouldBlock => break,
                        Err(e) => return Err(e),
                    }
                },
                token => {
                    let mut remove = false;
                    if let Some(stream) = connections.get_mut(&token) {
                        if let Some(buffer) = buffers.get_mut(&token) {
                            remove = handle_client_event(stream, buffer);
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
