use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};

use memchr::memmem;

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
fn parse_path(buf: &[u8]) -> Option<&[u8]> {
    let line_end = memmem::find(buf, b"\r\n")?;
    let line = &buf[..line_end];
    let sp1 = memmem::find(line, b" ")?;
    let rest = &line[sp1 + 1..];
    let sp2_rel = memmem::find(rest, b" ")?;
    let sp2 = sp1 + 1 + sp2_rel;
    Some(&line[sp1 + 1..sp2])
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
        let Some(path) = parse_path(req) else {
            let _ = stream.write_all(R400);
            continue;
        };
        let body = &req[hend..];

        if (path.len() == 9 && path == b"/payments") || (path.len() == 7 && path == b"/health") {
            push_best_effort(&mut ingest, body);
            let _ = stream.write_all(R204);
        } else {
            let _ = stream.write_all(R404);
        }
    }
}
