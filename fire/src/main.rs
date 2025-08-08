use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};

const R_OK: &[u8] = b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 2\r\n\r\nOK";
const R_404: &[u8] = b"HTTP/1.1 404 Not Found\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
const R_400: &[u8] = b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";

fn parse_path(buf: &[u8]) -> Option<&str> {
    let line_end = buf.windows(2).position(|w| w == b"\r\n")?;
    let line = &buf[..line_end];
    let mut it = line.split(|&b| b == b' ');
    it.next()?;
    let path = it.next()?;
    std::str::from_utf8(path).ok()
}

fn parse_body(buf: &[u8]) -> Option<&[u8]> {
    if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
        Some(&buf[i + 4..])
    } else {
        None
    }
}

fn main() {
    let listen_path = "/tmp/in.sock";
    let forward_path = "/tmp/out.sock";

    let _ = std::fs::remove_file(listen_path);
    let listener = UnixListener::bind(listen_path).unwrap();
    let mut forward = UnixStream::connect(forward_path).unwrap();

    let mut buf = [0u8; 64 * 1024];

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };

        let n = match stream.read(&mut buf) {
            Ok(n) if n > 0 => n,
            _ => {
                let _ = stream.write_all(R_400);
                continue;
            }
        };
        let req = &buf[..n];

        let Some(path) = parse_path(req) else {
            let _ = stream.write_all(R_400);
            continue;
        };
        let Some(body) = parse_body(req) else {
            let _ = stream.write_all(R_400);
            continue;
        };

        match path {
            "/payments" => {
                let _ = forward.write_all(&(body.len() as u32).to_be_bytes());
                let _ = forward.write_all(body);
                let _ = forward.write_all(b"\n");
                let _ = stream.write_all(R_OK);
            }
            "/health" => {
                let _ = stream.write_all(R_OK);
            }
            _ => {
                let _ = stream.write_all(R_404);
            }
        }
    }
}
