use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, atomic::AtomicU64};

use crate::{Merge, inner::queue::DropQueue};

#[derive(Debug)]
pub struct Common<T: Merge> {
    inner: Arc<CommonInner<T>>,
}

#[derive(Debug)]
struct CommonInner<T: Merge> {
    next_node_index: AtomicU64,
    next_process_index: AtomicU64,
    drop_queue: DropQueue<T>,
}

impl<T: Merge> Clone for Common<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Merge> Common<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(CommonInner {
                next_node_index: AtomicU64::new(0),
                next_process_index: AtomicU64::new(0),
                drop_queue: DropQueue::new(),
            }),
        }
    }

    pub fn next_node_index(&self) -> u64 {
        self.inner.next_node_index.fetch_add(1, Relaxed)
    }

    pub fn next_process_index(&self) -> u64 {
        self.inner.next_process_index.fetch_add(1, Relaxed)
    }

    pub fn drop_queue(&self) -> &DropQueue<T> {
        &self.inner.drop_queue
    }
}
