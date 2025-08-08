use std::time::Instant;
use std::{sync::Arc, time::Duration};
use tokio::net::UnixStream;
use tokio::sync::Notify;

use bytes::BufMut;
use bytes::{Bytes, BytesMut};
use chrono::Utc;
use crossbeam_queue::SegQueue;
use tokio::time;

use crate::vars;
use crate::{client::ProcessorClient, repository::Repository};

#[derive(Clone)]
pub struct Service {
    client: ProcessorClient,
    repository: Repository,
    queue: Arc<SegQueue<Bytes>>,
    notify: Arc<Notify>,
}

use tokio::io::{AsyncReadExt, AsyncWriteExt};

impl Service {
    pub fn new(client: ProcessorClient, repository: Repository) -> Service {
        return Service {
            client: client,
            repository: repository,
            queue: Arc::new(SegQueue::new()),
            notify: Arc::new(Notify::new()),
        };
    }

    pub fn initialize_master_worker(&self) {
        let client = self.client.clone();
        let repository = self.repository.clone();
        let queue = self.queue.clone();
        let notify = self.notify.clone();
        let trigger = vars::trigger();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(1));
            let mut buffer = BytesMut::with_capacity(128);

            loop {
                if let Some(request) = queue.pop() {
                    let json = Service::enrich_json(&mut buffer, &request).await;
                    let instant = Instant::now();
                    let success = client.capture_default(json.clone()).await;
                    let duration = instant.elapsed().as_millis();
                    if success {
                        repository.insert_default(json.clone()).await;
                        if duration <= trigger {
                            notify.notify_waiters();
                            while let Some(request) = queue.pop() {
                                let json = Service::enrich_json(&mut buffer, &request).await;
                                let instant = Instant::now();
                                let success = client.capture_default(json.clone()).await;
                                let duration = instant.elapsed().as_millis();
                                if success {
                                    repository.insert_default(json.clone()).await;
                                } else {
                                    queue.push(request);
                                }
                                if !success || duration > trigger {
                                    break;
                                }
                            }
                        }
                    } else {
                        queue.push(request);
                    }
                }
                interval.tick().await;
            }
        });
    }

    pub fn initialize_slave_workers(&self) {
        let workers = vars::slaves();

        for _ in 0..workers {
            let client = self.client.clone();
            let repository = self.repository.clone();
            let queue = self.queue.clone();
            let notify = self.notify.clone();
            let trigger = vars::trigger();

            tokio::spawn(async move {
                let mut buffer = BytesMut::with_capacity(128);

                loop {
                    notify.notified().await;
                    while let Some(request) = queue.pop() {
                        let json = Service::enrich_json(&mut buffer, &request).await;
                        let instant = Instant::now();
                        let success = client.capture_default(json.clone()).await;
                        let duration = instant.elapsed().as_millis();
                        if success {
                            repository.insert_default(json.clone()).await;
                        } else {
                            queue.push(request);
                        }
                        if !success || duration > trigger {
                            break;
                        }
                    }
                }
            });
        }
    }

    pub fn initialize_data_analyst(&self) {
        let analyst = vars::analyst();
        if analyst {
            let queue = self.queue.clone();

            tokio::spawn(async move {
                let mut interval = time::interval(Duration::from_secs(2));
                loop {
                    let length = queue.len();
                    if length > 0 {
                        println!("QUEUE_LEN: {}", queue.len());
                    }
                    interval.tick().await;
                }
            });
        }
    }

    pub fn initialize_poller(&self) {
        let queue = self.queue.clone();
        tokio::spawn(async move {
            const QUERY_SOCK: &str = "/sockets/pull.sock";
            const OP_PULL: u8 = 0x02;
            const OP_RESP: u8 = 0x03;

            let mut interval = time::interval(Duration::from_secs(2));
            loop {
                interval.tick().await;

                let mut sock = match UnixStream::connect(QUERY_SOCK).await {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                if sock.write_all(&[OP_PULL]).await.is_err() {
                    continue;
                }

                let mut hdr = [0u8; 5];
                if sock.read_exact(&mut hdr).await.is_err() || hdr[0] != OP_RESP {
                    continue;
                }
                let count = u32::from_be_bytes([hdr[1], hdr[2], hdr[3], hdr[4]]) as usize;

                for _ in 0..count {
                    let mut lenbuf = [0u8; 4];
                    if sock.read_exact(&mut lenbuf).await.is_err() {
                        break;
                    }
                    let len = u32::from_be_bytes(lenbuf) as usize;

                    let mut buf = BytesMut::with_capacity(len);
                    buf.resize(len, 0);
                    if sock.read_exact(&mut buf).await.is_err() {
                        break;
                    }
                    queue.push(buf.freeze());
                }
            }
        });
    }

    async fn enrich_json(buffer: &mut BytesMut, request: &Bytes) -> Bytes {
        buffer.clear();
        let brace_pos = request.iter().rposition(|&b| b == b'}').unwrap();
        buffer.put(request.slice(..brace_pos));
        buffer.put_slice(b",\"requestedAt\":\"");
        let date = Utc::now().format("%Y-%m-%dT%H:%M:%S.%3fZ").to_string();
        buffer.put_slice(date.as_bytes());
        buffer.put_slice(b"\"}");
        return Bytes::copy_from_slice(&buffer);
    }
}
