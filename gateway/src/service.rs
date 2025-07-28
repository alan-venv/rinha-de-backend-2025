use async_channel::{Receiver, Sender, unbounded};
use bytes::BufMut;
use bytes::{Bytes, BytesMut};
use chrono::Utc;
use crossbeam_queue::SegQueue;
use std::time::Instant;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::time;

use crate::utils::{TRIGGER, WORKERS};
use crate::{client::ProcessorClient, repository::Repository};

#[derive(Clone)]
pub struct Service {
    client: ProcessorClient,
    repository: Repository,
    default_health: Arc<AtomicBool>,
    #[allow(dead_code)]
    fallback_health: Arc<AtomicBool>,
    queue: Arc<SegQueue<Bytes>>,
    sender: Sender<Bytes>,
    receiver: Receiver<Bytes>,
}

impl Service {
    pub fn new(client: ProcessorClient, repository: Repository) -> Service {
        let (sender, receiver) = unbounded::<Bytes>();

        return Service {
            client: client,
            repository: repository,
            default_health: Arc::new(AtomicBool::new(false)),
            fallback_health: Arc::new(AtomicBool::new(false)),
            queue: Arc::new(SegQueue::new()),
            sender: sender,
            receiver: receiver,
        };
    }

    pub fn submit(&self, request: Bytes) {
        self.queue.push(request);
    }

    pub fn initialize_dispatcher(&self) {
        let client = self.client.clone();
        let repository = self.repository.clone();
        let queue = self.queue.clone();
        let sender = self.sender.clone();
        let health = self.default_health.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(1));
            let mut buffer = BytesMut::with_capacity(128);

            loop {
                if let Some(request) = queue.pop() {
                    buffer.clear();
                    let brace_pos = request.iter().rposition(|&b| b == b'}').unwrap();
                    buffer.put(request.slice(..brace_pos));
                    buffer.put_slice(b",\"requestedAt\":\"");
                    buffer.put_slice(
                        Utc::now()
                            .format("%Y-%m-%dT%H:%M:%S.%3fZ")
                            .to_string()
                            .as_bytes(),
                    );
                    buffer.put_slice(b"\"}");
                    let json = Bytes::copy_from_slice(&buffer);
                    let instant = Instant::now();
                    let success = client.capture_default(json.clone()).await;
                    let duration = instant.elapsed().as_millis();
                    if success {
                        repository.insert_default(json.clone()).await;
                        if duration <= TRIGGER {
                            health.store(true, Ordering::Relaxed);
                            loop {
                                if let Some(item) = queue.pop() {
                                    if sender.send_blocking(item).is_err() {
                                        break;
                                    }
                                } else {
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

    pub fn initialize_workers(&self) {
        for _ in 0..WORKERS {
            let client = self.client.clone();
            let repository = self.repository.clone();
            let queue = self.queue.clone();
            let receiver = self.receiver.clone();
            let health = self.default_health.clone();

            tokio::spawn(async move {
                let mut buffer = BytesMut::with_capacity(128);
                while let Ok(request) = receiver.recv().await {
                    if health.load(Ordering::Relaxed) {
                        buffer.clear();
                        let brace_pos = request.iter().rposition(|&b| b == b'}').unwrap();
                        buffer.put(request.slice(..brace_pos));
                        buffer.put_slice(b",\"requestedAt\":\"");
                        buffer.put_slice(Utc::now().to_rfc3339().as_bytes());
                        buffer.put_slice(b"\"}");
                        let json = Bytes::copy_from_slice(&buffer);
                        let instant = Instant::now();
                        let success = client.capture_default(json.clone()).await;
                        let duration = instant.elapsed().as_millis();
                        if success {
                            repository.insert_default(json.clone()).await;
                        } else {
                            queue.push(request);
                        }
                        if !success || duration > TRIGGER {
                            health.store(false, Ordering::Relaxed);
                        }
                    } else {
                        queue.push(request);
                    }
                }
            });
        }
    }
}
