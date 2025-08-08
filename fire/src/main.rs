use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;

const R200: &[u8] = b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 2\r\n\r\nOK";
const R404: &[u8] = b"HTTP/1.1 404 Not Found\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
const R400: &[u8] = b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";

#[inline]
fn first_line_end(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\r\n")
}
#[inline]
fn headers_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
}
#[inline]
fn parse_path_bytes(buf: &[u8]) -> Option<&[u8]> {
    let end = first_line_end(buf)?;
    let line = &buf[..end];
    let mut it = line.split(|&b| b == b' ');
    it.next()?;
    it.next()
}

fn main() {
    let path = &std::env::var("SOCKET").expect("socket path not set");
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path).unwrap();
    let permissions = std::fs::Permissions::from_mode(0o766);
    std::fs::set_permissions(path, permissions).unwrap();

    let mut buf = [0u8; 64 * 1024];

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };

        let n = match stream.read(&mut buf) {
            Ok(n) if n > 0 => n,
            _ => {
                let _ = stream.write_all(R400);
                continue;
            }
        };
        let req = &buf[..n];

        let Some(path) = parse_path_bytes(req) else {
            let _ = stream.write_all(R400);
            continue;
        };
        let Some(hend) = headers_end(req) else {
            let _ = stream.write_all(R400);
            continue;
        };
        let _body = &req[hend..];

        // Fast-path de roteamento por comparação direta de bytes
        if path == b"/payments" {
            let _ = stream.write_all(R200);
        } else if path == b"/health" {
            let _ = stream.write_all(R200);
        } else {
            let _ = stream.write_all(R404);
        }
    }
}
