use bytes::BufMut;
use bytes::{Bytes, BytesMut};
use chrono::Utc;
use crossbeam_queue::SegQueue;
use std::time::Instant;
use std::{sync::Arc, time::Duration};
use tokio::time;

use crate::utils::TRIGGER;
use crate::{client::ProcessorClient, repository::Repository};

#[derive(Clone)]
pub struct Service {
    client: ProcessorClient,
    repository: Repository,
    queue: Arc<SegQueue<Bytes>>,
}

impl Service {
    pub fn new(client: ProcessorClient, repository: Repository) -> Service {
        return Service {
            client: client,
            repository: repository,
            queue: Arc::new(SegQueue::new()),
        };
    }

    pub fn submit(&self, request: Bytes) {
        self.queue.push(request);
    }

    pub fn initialize_worker(&self) {
        let client = self.client.clone();
        let repository = self.repository.clone();
        let queue = self.queue.clone();

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
                        if duration <= TRIGGER {
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
                                if !success || duration > TRIGGER {
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
