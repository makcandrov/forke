use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, atomic::AtomicU64};

#[derive(Debug, Clone)]
pub struct Common {
    inner: Arc<CommonInner>,
}

#[derive(Debug)]
struct CommonInner {
    next_node_index: AtomicU64,
}

impl Common {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(CommonInner {
                next_node_index: AtomicU64::new(0),
            }),
        }
    }

    pub fn next_node_index(&self) -> u64 {
        self.inner.next_node_index.fetch_add(1, Relaxed)
    }
}
