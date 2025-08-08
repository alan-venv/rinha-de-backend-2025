use crossbeam::queue::SegQueue;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;

fn main() {
    let listen_path = "/tmp/in.sock";
    let forward_path = "/tmp/out.sock";

    let _ = std::fs::remove_file(listen_path);
    let listener = UnixListener::bind(listen_path).unwrap();
    let mut forward = UnixStream::connect(forward_path).unwrap();
    let queue = Arc::new(SegQueue::new());
    let mut buf = [0u8; 1024];

    for stream in listener.incoming() {
        if let Ok(mut stream) = stream {
            if let Ok(n) = stream.read(&mut buf) {
                // push na fila
                queue.push((n, buf));

                // repasse para consumidor
                let _ = forward.write_all(&buf[..n]);

                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK");
            }
        }
    }
}
