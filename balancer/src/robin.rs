use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[allow(dead_code)]
pub struct RoundRobin<T> {
    items: Vec<T>,
    counter: AtomicUsize,
}

#[allow(dead_code)]
impl<T> RoundRobin<T> {
    pub fn new(items: Vec<T>) -> Arc<Self> {
        Arc::new(Self {
            items,
            counter: AtomicUsize::new(0),
        })
    }

    pub fn next(&self) -> &T {
        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.items.len();
        &self.items[idx]
    }
}
