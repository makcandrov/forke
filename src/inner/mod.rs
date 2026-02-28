use hashbrown::HashMap;

use crate::Merge;

mod common;
pub use common::Common;

mod guard;
pub use guard::NodeGuard;

mod handle;
pub(crate) use handle::SelfHandle;
use handle::{ChildHandle, ParentHandle};

mod lock;

mod multiplicity;
use multiplicity::Multiplicity;

mod queue;

pub type NodeIndex = u64;

#[derive(Debug)]
struct NodeInner<T: Merge> {
    parent: Option<ParentHandle<T>>,
    children: HashMap<u64, ChildHandle<T>>,
    alive: bool,
    index: u64,
    counter: Counter,
    data: T,
}

impl<T: Merge> NodeInner<T> {
    pub fn new(parent: Option<ParentHandle<T>>, counter: Counter, data: T) -> Self {
        Self {
            parent,
            children: HashMap::new(),
            alive: true,
            index: counter.next(),
            counter,
            data,
        }
    }

    #[inline]
    pub fn root(data: T) -> Self {
        Self::new(None, Counter::new(), data)
    }

    #[inline]
    pub fn child(parent: ParentHandle<T>, counter: Counter, data: T) -> Self {
        Self::new(Some(parent), counter, data)
    }

    #[inline]
    pub fn insert_child(&mut self, index: NodeIndex, handle: ChildHandle<T>) {
        let old = self.children.insert(index, handle);
        debug_assert!(old.is_none(), "index duplicate");
    }
}
