use async_channel::{Receiver, Sender, unbounded};
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
use crate::{client::ProcessorClient, models::PaymentRequest, repository::Repository};

#[derive(Clone)]
pub struct Service {
    client: ProcessorClient,
    repository: Repository,
    default_health: Arc<AtomicBool>,
    #[allow(dead_code)]
    fallback_health: Arc<AtomicBool>,
    queue: Arc<SegQueue<PaymentRequest>>,
    sender: Sender<PaymentRequest>,
    receiver: Receiver<PaymentRequest>,
}

impl Service {
    pub fn new(client: ProcessorClient, repository: Repository) -> Service {
        let (sender, receiver) = unbounded::<PaymentRequest>();
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

    pub fn submit(&self, request: PaymentRequest) {
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
            loop {
                if let Some(request) = queue.pop() {
                    let instant = Instant::now();
                    let req = request.to_processor();
                    let success = client.capture_default(&req).await;
                    let duration = instant.elapsed().as_millis();
                    if success {
                        repository.insert_default(&req).await;
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
                while let Ok(request) = receiver.recv().await {
                    if health.load(Ordering::Relaxed) {
                        let instant = Instant::now();
                        let req = request.to_processor();
                        let success = client.capture_default(&req).await;
                        let duration = instant.elapsed().as_millis();
                        if success {
                            repository.insert_default(&req).await;
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
