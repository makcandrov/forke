use std::sync::atomic::AtomicU64;

use hashbrown::{HashMap, hash_map::Entry};
use parking_lot::RwLock;

use crate::{Merge, inner::handle::StrongHandle};

pub type ProcessIndex = u64;

#[derive(Debug)]
pub struct DropQueue<T: Merge> {
    queues: RwLock<HashMap<ProcessIndex, Vec<StrongHandle<T>>>>,
    next_process_index: AtomicU64,
}

impl<T: Merge> DropQueue<T> {
    pub fn new() -> Self {
        Self {
            queues: Default::default(),
            next_process_index: AtomicU64::new(0),
        }
    }

    pub fn execute_drops(&self, process_id: u64) {
        let mut queues = self.queues.write();
        let queue = queues.remove(&process_id);
        for to_drop in queue {
            drop(to_drop);
        }
    }

    pub fn insert_to_queue(&self, process_id: u64, handle: StrongHandle<T>) {
        match self.queues.write().entry(process_id) {
            Entry::Occupied(o) => o.into_mut().push(handle),
            Entry::Vacant(v) => {
                v.insert(vec![handle]);
            }
        }
    }
}
