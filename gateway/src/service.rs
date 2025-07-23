use crossbeam_channel::{Receiver, Sender, unbounded};
use crossbeam_queue::SegQueue;
use reqwest::blocking::Client;
use std::cmp::min;
use std::thread;
use std::time::Instant;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use crate::utils::{FLAG_TIMEOUT_TRIGGER_IN_MILLISECONDS, WORKER_COUNT, WORKER_JOBS_COUNT};
use crate::{client::ProcessorClient, models::PaymentRequest, repository::Repository};

#[derive(Clone)]
pub struct Service {
    default_health: Arc<AtomicBool>,
    #[allow(dead_code)]
    fallback_health: Arc<AtomicBool>,
    queue: Arc<SegQueue<PaymentRequest>>,
    sender: Sender<PaymentRequest>,
    receiver: Receiver<PaymentRequest>,
}

impl Service {
    pub fn new() -> Service {
        let (sender, receiver) = unbounded::<PaymentRequest>();
        return Service {
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
        let queue = self.queue.clone();
        let sender = self.sender.clone();
        let health = self.default_health.clone();

        thread::spawn(move || {
            let sync = Client::new();
            let client = ProcessorClient::new(sync.clone());
            let repository = Repository::new(sync.clone());
            loop {
                if let Some(request) = queue.pop() {
                    let instant = Instant::now();
                    let req = request.to_processor();
                    let success = client.capture_default_sync(&req);
                    let duration = instant.elapsed().as_millis();
                    if success {
                        repository.insert_default_sync(&req);
                        if duration <= FLAG_TIMEOUT_TRIGGER_IN_MILLISECONDS {
                            health.store(true, Ordering::Relaxed);
                            if queue.len() > 0 {
                                let count = min(queue.len(), WORKER_JOBS_COUNT);
                                for _ in 0..count {
                                    if let Some(item) = queue.pop() {
                                        if sender.send(item).is_err() {
                                            println!("Morri");
                                            return;
                                        }
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                    } else {
                        queue.push(request);
                    }
                }
                thread::sleep(Duration::from_secs(1));
            }
        });
    }

    pub fn initialize_workers(&self) {
        for _ in 0..WORKER_COUNT {
            let queue = self.queue.clone();
            let receiver = self.receiver.clone();
            let health = self.default_health.clone();

            thread::spawn(move || {
                let sync = Client::new();
                let client = ProcessorClient::new(sync.clone());
                let repository = Repository::new(sync.clone());
                for request in receiver {
                    if health.load(Ordering::Relaxed) {
                        let instant = Instant::now();
                        let req = request.to_processor();
                        let success = client.capture_default_sync(&req);
                        let duration = instant.elapsed().as_millis();
                        if success {
                            repository.insert_default_sync(&req);
                        } else {
                            queue.push(request);
                        }
                        if !success || duration > FLAG_TIMEOUT_TRIGGER_IN_MILLISECONDS {
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
