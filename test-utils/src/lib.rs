use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering::Relaxed},
};

use forke::Merge;
use parking_lot::RwLock;

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

pub type Id = u64;

pub fn next_id() -> Id {
    NEXT_ID.fetch_add(1, Relaxed)
}

/// Per-node payload that tracks drops and merges externally.
#[derive(Debug)]
pub struct Tracked {
    id: Id,
    dropped: Arc<AtomicBool>,
    merged_into_me: Arc<RwLock<Vec<Id>>>,
}

/// External observer — can check state after the node is gone.
#[derive(Debug, Clone)]
pub struct Obs {
    id: Id,
    dropped: Arc<AtomicBool>,
    merged_into_me: Arc<RwLock<Vec<Id>>>,
}

impl Tracked {
    pub fn new() -> Self {
        Self {
            id: next_id(),
            dropped: Default::default(),
            merged_into_me: Default::default(),
        }
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn obs(&self) -> Obs {
        Obs {
            id: self.id,
            dropped: self.dropped.clone(),
            merged_into_me: self.merged_into_me.clone(),
        }
    }

    pub fn pair() -> (Self, Obs) {
        let t = Self::new();
        let o = t.obs();
        (t, o)
    }
}

impl Default for Tracked {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.dropped.store(true, Relaxed);
    }
}

impl Merge for Tracked {
    fn merge(parent: &mut Self, child: Self) {
        parent.merged_into_me.write().push(child.id);
    }
}

impl Obs {
    pub fn id(&self) -> Id {
        self.id
    }

    pub fn is_dropped(&self) -> bool {
        self.dropped.load(Relaxed)
    }

    pub fn merges(&self) -> Vec<Id> {
        self.merged_into_me.read().clone()
    }
}
