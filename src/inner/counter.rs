use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

#[derive(Debug, Clone, Default)]
pub struct Counter(Arc<AtomicU64>);

impl Counter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next(&self) -> u64 {
        let n = self.0.fetch_add(1, Ordering::Relaxed);
        assert_ne!(n, u64::MAX);
        n
    }
}
