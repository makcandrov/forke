//! Shared helpers for the `forke` integration tests: a payload type whose
//! drops and merges can be observed from outside the tree, and a global
//! allocator that tracks live allocations for leak detection.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering::Relaxed},
};

use forke::Merge;
use parking_lot::RwLock;

/// Global allocator wrapper that keeps a running total of allocated bytes.
///
/// Install it with `#[global_allocator]` and compare [`allocated`] snapshots
/// taken before and after the scenario under test: equal totals mean no leak.
///
/// [`allocated`]: Self::allocated
#[derive(Debug, Default)]
pub struct TrackingAllocator {
    inner: System,
    allocated: AtomicUsize,
}

impl TrackingAllocator {
    pub const fn new() -> Self {
        Self {
            inner: System,
            allocated: AtomicUsize::new(0),
        }
    }

    /// Currently allocated bytes.
    pub fn allocated(&self) -> usize {
        self.allocated.load(Relaxed)
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.inner.alloc(layout) };
        if !ptr.is_null() {
            self.allocated.fetch_add(layout.size(), Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.inner.dealloc(ptr, layout) };
        self.allocated.fetch_sub(layout.size(), Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { self.inner.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            if new_size > layout.size() {
                self.allocated.fetch_add(new_size - layout.size(), Relaxed);
            } else {
                self.allocated.fetch_sub(layout.size() - new_size, Relaxed);
            }
        }
        new_ptr
    }
}

/// Process-wide unique identifier of a [`Tracked`] payload.
pub type Id = u64;

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

fn next_id() -> Id {
    NEXT_ID.fetch_add(1, Relaxed)
}

/// Node payload whose drop and merge events are recorded externally.
///
/// Its [`Merge`] implementation appends the consumed child's [`Id`] to the
/// parent's merge log, so tests can reconstruct exactly which payloads were
/// folded where.
#[derive(Debug)]
pub struct Tracked {
    id: Id,
    dropped: Arc<AtomicBool>,
    merged_into_me: Arc<RwLock<Vec<Id>>>,
}

/// External observer of a [`Tracked`] payload — usable after the payload
/// itself is gone.
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

    /// Creates an observer of this payload.
    pub fn obs(&self) -> Obs {
        Obs {
            id: self.id,
            dropped: self.dropped.clone(),
            merged_into_me: self.merged_into_me.clone(),
        }
    }

    /// Creates a payload together with its observer.
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

    /// Whether the observed payload has been dropped.
    pub fn is_dropped(&self) -> bool {
        self.dropped.load(Relaxed)
    }

    /// Ids of the payloads that were merged into the observed payload, in
    /// merge order.
    pub fn merges(&self) -> Vec<Id> {
        self.merged_into_me.read().clone()
    }
}
