use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;

use memchr::memmem;

const R204: &[u8] = b"HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
const R404: &[u8] = b"HTTP/1.1 404 Not Found\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
const R400: &[u8] = b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";

const BUF_CAP: usize = 1 * 1024; // 64
const HEADER_LIMIT: usize = 1 * 1024; // 16

#[inline]
fn first_line_end(buf: &[u8]) -> Option<usize> {
    memmem::find(buf, b"\r\n")
}
#[inline]
fn headers_end(buf: &[u8]) -> Option<usize> {
    memmem::find(buf, b"\r\n\r\n").map(|i| i + 4)
}
#[inline]
fn parse_path_bytes(buf: &[u8]) -> Option<&[u8]> {
    let end = first_line_end(buf)?;
    let line = &buf[..end];
    let mut it = line.split(|&b| b == b' ');
    it.next()?; // method
    it.next() // path
}

fn main() {
    let path = &std::env::var("SOCKET").expect("socket path not set");
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path).unwrap();
    let permissions = std::fs::Permissions::from_mode(0o777);
    std::fs::set_permissions(path, permissions).unwrap();

    let mut buf = [0u8; BUF_CAP];

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Leitura em loop até encontrar fim dos headers ou atingir limite
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
        let Some(path_bytes) = parse_path_bytes(req) else {
            let _ = stream.write_all(R400);
            continue;
        };
        let _body = &req[hend..];

        // Fast-path: checa comprimento antes do memcmp
        if (path_bytes.len() == 9 && path_bytes == b"/payments")
            || (path_bytes.len() == 7 && path_bytes == b"/health")
        {
            let _ = stream.write_all(R204);
        } else {
            let _ = stream.write_all(R404);
        }
    }
}
