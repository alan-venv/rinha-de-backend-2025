use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;

use memchr::memmem;

const R204: &[u8] = b"HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
const R404: &[u8] = b"HTTP/1.1 404 Not Found\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
const R400: &[u8] = b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";

const BUF_CAP: usize = 2 * 1024; // 64
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
fn parse_path_bytes(buf: &[u8], line_end: usize) -> Option<&[u8]> {
    let line = &buf[..line_end];
    let sp1 = memmem::find(line, b" ")?;
    let sp2 = memmem::find(&line[sp1 + 1..], b" ")? + sp1 + 1;
    Some(&line[sp1 + 1..sp2])
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

        // read loop: headers only
        let mut n = 0usize;
        let (line_end, headers_end) = loop {
            if n >= HEADER_LIMIT {
                break (None, None);
            }
            match stream.read(&mut buf[n..]) {
                Ok(0) => break (None, None),
                Ok(m) => {
                    n += m;
                    let le = first_line_end(&buf[..n]);
                    let he = headers_end(&buf[..n]);
                    if let (Some(le), Some(he)) = (le, he) {
                        break (Some(le), Some(he));
                    }
                    if n == BUF_CAP {
                        break (None, None);
                    }
                }
                Err(_) => break (None, None),
            }
        };

        let (Some(le), Some(hend)) = (line_end, headers_end) else {
            let _ = stream.write_all(R400);
            continue;
        };

        let req = &buf[..n];
        let Some(path_bytes) = parse_path_bytes(req, le) else {
            let _ = stream.write_all(R400);
            continue;
        };
        let _body = &req[hend..]; // body presente até BUF_CAP; se precisar inteiro, ler mais depois

        if (path_bytes.len() == 9 && path_bytes == b"/payments")
            || (path_bytes.len() == 7 && path_bytes == b"/health")
        {
            let _ = stream.write_all(R204);
        } else {
            let _ = stream.write_all(R404);
        }
    }
}
