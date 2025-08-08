use std::collections::HashMap;
use std::io::ErrorKind::WouldBlock;
use std::io::{Read, Result, Write};
use std::net::SocketAddr;

use bytes::{Bytes, BytesMut};
use crossbeam_channel::{Sender, unbounded};
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use umbral_socket::stream::UmbralSyncClient;

const SERVER: Token = Token(0);
const READABLE: Interest = Interest::READABLE;
const BUFFER_SIZE: usize = 1024;
const R200: &[u8] = b"HTTP/1.1 200 OK\r\n\r\n";
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

fn handle_client_event(
    stream: &mut TcpStream,
    buffer: &mut Vec<u8>,
    tx: &Sender<Vec<u8>>,
    client: &mut UmbralSyncClient,
) -> bool {
    let mut tmp = [0u8; BUFFER_SIZE];
    match stream.read(&mut tmp) {
        Ok(0) => return true,
        Ok(n) => {
            buffer.extend_from_slice(&tmp[..n]);
            if let Some(body) = parse_body(buffer) {
                let route = extract_route(buffer);
                let (path, query) = extract_path_and_query(route);

                let response: Bytes = match path {
                    "/payments-summary" => {
                        let response = if let Some(data) = query {
                            let content = Bytes::copy_from_slice(data.as_bytes());
                            client.send("SUMMARY", &content).unwrap()
                        } else {
                            client.send("SUMMARY", &Bytes::new()).unwrap()
                        };
                        let r2 = Bytes::copy_from_slice(R200);
                        let mut combined = BytesMut::with_capacity(r2.len() + response.len());
                        combined.extend_from_slice(&r2);
                        combined.extend_from_slice(&response);
                        combined.freeze()
                    }
                    "/payments" => {
                        tx.send(body.to_vec()).ok();
                        Bytes::from_static(R202)
                    }
                    "/purge-payments" => {
                        client.send("PURGE", &Bytes::new()).unwrap();
                        Bytes::from_static(R200)
                    }
                    _ => Bytes::from_static(R404),
                };
                let _ = stream.write_all(response.as_ref());
                return true;
            }
        }
        Err(ref e) if e.kind() == WouldBlock => {}
        Err(_) => return true,
    }
    false
}

fn main() -> Result<()> {
    println!("VERSION: 1.0 SKYLAKE");
    let addr: SocketAddr = "0.0.0.0:9999".parse().unwrap();
    let mut listener = TcpListener::bind(addr)?;
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(1024);
    let mut connections: HashMap<Token, TcpStream> = HashMap::new();
    let mut buffers: HashMap<Token, Vec<u8>> = HashMap::new();
    let mut unique_token = Token(SERVER.0 + 1);
    let (tx, rx) = unbounded::<Vec<u8>>();
    let mut database = UmbralSyncClient::new("/sockets/database.sock");

    std::thread::spawn(move || {
        let mut gateway1 = UmbralSyncClient::new("/sockets/actix.sock.1");
        let mut gateway2 = UmbralSyncClient::new("/sockets/actix.sock.2");

        let mut robin = true;
        for msg in rx {
            let response = if robin {
                gateway1.send("SAVE", &Bytes::from(msg))
            } else {
                gateway2.send("SAVE", &Bytes::from(msg))
            };

            if let Err(_) = response {
                println!("Deu ruim!");
            }
            robin = !robin;
        }
    });

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
                            remove = handle_client_event(stream, buffer, &tx, &mut database);
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
