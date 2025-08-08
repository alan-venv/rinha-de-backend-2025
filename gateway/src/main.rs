use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};

use bytes::{Bytes, BytesMut};
use memchr::memmem;
use umbral_socket::stream::UmbralSyncClient;

const R200: &[u8] = b"HTTP/1.1 200 OK\r\n\r\n";
const R204: &[u8] = b"HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
const R404: &[u8] = b"HTTP/1.1 404 Not Found\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
const R400: &[u8] = b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";

const BUF_CAP: usize = 2 * 1024; // 64
const HEADER_LIMIT: usize = 1 * 1024; // 16

#[inline]
fn headers_end(buf: &[u8]) -> Option<usize> {
    memmem::find(buf, b"\r\n\r\n").map(|i| i + 4)
}
#[inline]
fn parse_path_and_query(buf: &[u8]) -> Result<(&[u8], Option<&[u8]>), ()> {
    let line_end = memmem::find(buf, b"\r\n").ok_or(())?;
    let line = &buf[..line_end];
    let sp1 = memmem::find(line, b" ").ok_or(())?;
    let rest = &line[sp1 + 1..];
    let sp2_rel = memmem::find(rest, b" ").ok_or(())?;
    let sp2 = sp1 + 1 + sp2_rel;
    let full_path = &line[sp1 + 1..sp2];

    if let Some(q_idx) = memmem::find(full_path, b"?") {
        Ok((&full_path[..q_idx], Some(&full_path[q_idx + 1..])))
    } else {
        Ok((full_path, None))
    }
}

const INGEST_SOCK: &str = "/sockets/push.sock";

#[inline]
fn push_best_effort(sock: &mut UnixStream, body: &[u8]) {
    let mut hdr = [0u8; 5];
    hdr[0] = 0x01; // OP_PUSH
    hdr[1..5].copy_from_slice(&(body.len() as u32).to_be_bytes());

    // write header
    match sock.write(&hdr) {
        Ok(5) => {}
        Ok(_) => return,
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return,
        Err(_) => return,
    }

    // write body (short-write aware, nonblocking)
    let mut off = 0usize;
    while off < body.len() {
        match sock.write(&body[off..]) {
            Ok(0) => return,
            Ok(n) => off += n,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return,
            Err(_) => return,
        }
    }
}

fn main() {
    let path = &std::env::var("SOCKET").expect("socket path not set");
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path).unwrap();
    let permissions = std::fs::Permissions::from_mode(0o777);
    std::fs::set_permissions(path, permissions).unwrap();

    let mut ingest = UnixStream::connect(INGEST_SOCK).expect("connect ingest.sock");
    let _ = ingest.set_nonblocking(true);

    let mut buf = [0u8; BUF_CAP];

    // UMBRAL CLIENT FOR SUMMARY AND PURGE //
    let mut umbral = UmbralSyncClient::new("/sockets/worker.sock");

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };

        let mut n = 0usize;
        let hend = loop {
            if n >= HEADER_LIMIT {
                break None;
            }
            match stream.read(&mut buf[n..]) {
                Ok(0) => break None,
                Ok(m) => {
                    n += m;
                    if let Some(h) = headers_end(&buf[..n]) {
                        break Some(h);
                    }
                    if n == BUF_CAP {
                        break None;
                    }
                }
                Err(_) => break None,
            }
        };

        let Some(hend) = hend else {
            let _ = stream.write_all(R400);
            continue;
        };

        let req = &buf[..n];
        let (path, query) = match parse_path_and_query(req) {
            Ok(v) => v,
            Err(_) => {
                let _ = stream.write_all(R400);
                continue;
            }
        };

        let body = &req[hend..];

        if path.len() == 9 || path == b"/payments" {
            push_best_effort(&mut ingest, body);
            let _ = stream.write_all(R204);
        } else if path.len() == 17 || path == b"/payments-summary" {
            let payload = match query {
                Some(q) => Bytes::copy_from_slice(q),
                None => Bytes::new(),
            };
            println!("{:?}", query.unwrap());
            let response = umbral.send("SUMMARY", &payload).unwrap();
            let r2 = Bytes::copy_from_slice(R200);
            let mut combined = BytesMut::with_capacity(r2.len() + response.len());
            combined.extend_from_slice(&r2);
            combined.extend_from_slice(&response);
            let _ = stream.write_all(&combined.freeze());
        } else if path.len() == 15 || path == b"/purge-payments" {
            let _ = umbral.send("PURGE", &Bytes::new()).unwrap();
            let _ = stream.write_all(R200);
        } else {
            let _ = stream.write_all(R404);
        }
    }
}
