use std::collections::{HashMap, VecDeque};
use std::io::{ErrorKind, Read, Write};
use std::os::unix::fs::PermissionsExt;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use mio::net::{UnixListener, UnixStream};
use mio::{Events, Interest, Poll, Token, Waker};

const PUSH_SOCK: &str = "/sockets/push.sock";
const PULL_SOCK: &str = "/sockets/pull.sock";

const TKN_WAKE: Token = Token(0);
const TKN_INGEST_LISTENER: Token = Token(1);
const TKN_QUERY_LISTENER: Token = Token(2);
const TKN_START: usize = 10;

// Protocolo:
// PUSH: [u8 op=0x01][u32 body_len][body]
// PULL: [u8 op=0x02]
// RESP: [u8 op=0x03][u32 count]{[u32 len][bytes]}*count
const OP_PUSH: u8 = 0x01;
const OP_PULL: u8 = 0x02;
const OP_RESP: u8 = 0x03;

const MAX_BODY: usize = 4096;
const QUEUE_CAP_ITEMS: usize = 100_000;
const QUEUE_CAP_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone)]
struct Item {
    data: Bytes,
    sz: usize, // len field (4) + data.len()
}

struct Conn {
    stream: UnixStream,
    rd: BytesMut,
    wr: BytesMut,
    kind: ConnKind,
}

enum ConnKind {
    Ingest,
    Query,
}

struct Queue {
    q: VecDeque<Item>,
    bytes: usize,
}
impl Queue {
    fn new() -> Self {
        Self {
            q: VecDeque::new(),
            bytes: 0,
        }
    }
    fn can_push(&self, sz: usize) -> bool {
        self.q.len() < QUEUE_CAP_ITEMS && self.bytes + sz <= QUEUE_CAP_BYTES
    }
    fn push(&mut self, it: Item) {
        self.bytes += it.sz;
        self.q.push_back(it);
    }
    fn drain_all(&mut self) -> Vec<Item> {
        let mut out = Vec::with_capacity(self.q.len());
        while let Some(it) = self.q.pop_front() {
            self.bytes -= it.sz;
            out.push(it);
        }
        out
    }
}

fn chmod_777(p: &str) {
    let _ = std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o777));
}
fn bind_listener(path: &str) -> UnixListener {
    let _ = std::fs::remove_file(path);
    let l = UnixListener::bind(path).expect("bind");
    chmod_777(path);
    l
}
fn register_stream(poll: &Poll, stream: &mut UnixStream, token: Token) {
    poll.registry()
        .register(stream, token, Interest::READABLE | Interest::WRITABLE)
        .expect("register");
}

fn main() {
    let mut ingest_listener = bind_listener(PUSH_SOCK);
    let mut query_listener = bind_listener(PULL_SOCK);

    let mut poll = Poll::new().unwrap();
    let waker = Waker::new(poll.registry(), TKN_WAKE).unwrap();
    poll.registry()
        .register(
            &mut ingest_listener,
            TKN_INGEST_LISTENER,
            Interest::READABLE,
        )
        .unwrap();
    poll.registry()
        .register(&mut query_listener, TKN_QUERY_LISTENER, Interest::READABLE)
        .unwrap();

    let mut events = Events::with_capacity(1024);
    let mut conns: HashMap<usize, Conn> = HashMap::new();
    let mut next_token = TKN_START;
    let mut queue = Queue::new();

    loop {
        poll.poll(&mut events, None).unwrap();
        for ev in events.iter() {
            match ev.token() {
                TKN_WAKE => {}
                TKN_INGEST_LISTENER => accept_loop(
                    &poll,
                    &mut ingest_listener,
                    ConnKind::Ingest,
                    &mut conns,
                    &mut next_token,
                ),
                TKN_QUERY_LISTENER => accept_loop(
                    &poll,
                    &mut query_listener,
                    ConnKind::Query,
                    &mut conns,
                    &mut next_token,
                ),
                Token(t) => {
                    if let Some(conn) = conns.get_mut(&t) {
                        if ev.is_readable() {
                            if !read_into(conn) {
                                drop_conn(&poll, &mut conns, t);
                                continue;
                            }
                            match conn.kind {
                                ConnKind::Ingest => handle_push(conn, &mut queue),
                                ConnKind::Query => handle_query(conn, &mut queue),
                            }
                        }
                        if ev.is_writable() && !conn.wr.is_empty() {
                            if !write_from(conn) {
                                drop_conn(&poll, &mut conns, t);
                            }
                        }
                    }
                }
            }
        }
        let _ = &waker;
    }
}

fn accept_loop(
    poll: &Poll,
    listener: &mut UnixListener,
    kind: ConnKind,
    conns: &mut HashMap<usize, Conn>,
    next_token: &mut usize,
) {
    loop {
        match listener.accept() {
            Ok((mut s, _)) => {
                let tok = *next_token;
                *next_token += 1;
                register_stream(poll, &mut s, Token(tok));
                conns.insert(
                    tok,
                    Conn {
                        stream: s,
                        rd: BytesMut::with_capacity(8 * 1024),
                        wr: BytesMut::new(),
                        kind: match kind {
                            ConnKind::Ingest => ConnKind::Ingest,
                            ConnKind::Query => ConnKind::Query,
                        },
                    },
                );
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }
}

fn drop_conn(poll: &Poll, conns: &mut HashMap<usize, Conn>, t: usize) {
    if let Some(mut c) = conns.remove(&t) {
        let _ = poll.registry().deregister(&mut c.stream);
    }
}

fn read_into(c: &mut Conn) -> bool {
    let mut tmp = [0u8; 8192];
    loop {
        match c.stream.read(&mut tmp) {
            Ok(0) => return false,
            Ok(n) => c.rd.extend_from_slice(&tmp[..n]),
            Err(e) if e.kind() == ErrorKind::WouldBlock => return true,
            Err(_) => return false,
        }
    }
}

fn write_from(c: &mut Conn) -> bool {
    while !c.wr.is_empty() {
        match c.stream.write(&c.wr) {
            Ok(0) => return false,
            Ok(n) => {
                let _ = c.wr.split_to(n);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => return true,
            Err(_) => return false,
        }
    }
    true
}

// ---- Ingest: PUSH [0x01][u32 len][bytes]
fn handle_push(c: &mut Conn, queue: &mut Queue) {
    loop {
        if c.rd.len() < 1 + 4 {
            return;
        }
        if c.rd[0] != OP_PUSH {
            c.rd.clear();
            return;
        }
        let len = u32::from_be_bytes([c.rd[1], c.rd[2], c.rd[3], c.rd[4]]) as usize;
        if len > MAX_BODY {
            c.rd.clear();
            return;
        }
        let need = 1 + 4 + len;
        if c.rd.len() < need {
            return;
        }

        let mut take = c.rd.split_to(need);
        take.advance(1 + 4);
        let data = take.freeze();
        let sz = 4 + data.len();

        if queue.can_push(sz) {
            println!("{:?}", data);
            queue.push(Item { data, sz });
        } else {
            // drop sob pressão
        }
    }
}

// ---- Query: PULL [0x02] → RESP com TODOS os itens
fn handle_query(c: &mut Conn, queue: &mut Queue) {
    if c.rd.is_empty() {
        return;
    }
    if c.rd[0] != OP_PULL {
        c.rd.clear();
        return;
    }
    let _ = c.rd.split_to(1);

    let batch = queue.drain_all();
    let mut size = 1 + 4; // op + count
    for it in &batch {
        size += 4 + it.data.len();
    }

    c.wr.reserve(size);
    c.wr.put_u8(OP_RESP);
    c.wr.put_u32(batch.len() as u32);
    for it in batch {
        c.wr.put_u32(it.data.len() as u32);
        c.wr.extend_from_slice(&it.data);
    }
    let _ = write_from(c);
}
