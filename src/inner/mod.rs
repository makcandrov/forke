use hashbrown::HashMap;

use crate::NodeData;

mod common;
pub use common::Common;

mod guard;
pub use guard::NodeGuard;

mod handle;
pub use handle::Handle;

mod multiplicity;
use multiplicity::Multiplicity;

pub type NodeIndex = u64;

#[derive(Debug)]
struct NodeInner<T: NodeData> {
    parent: Option<Handle<T>>,
    children: HashMap<u64, Handle<T>>,
    alive: bool,
    index: u64,
    common: Common,
    data: T,
}

impl<T: NodeData> NodeInner<T> {
    pub fn new(parent: Option<Handle<T>>, common: Common, data: T) -> Self {
        Self {
            parent,
            children: HashMap::new(),
            alive: true,
            index: common.next_node_index(),
            common,
            data,
        }
    }

    #[inline]
    pub fn root(data: T) -> Self {
        Self::new(None, Common::new(), data)
    }

    #[inline]
    pub fn child(parent: Handle<T>, common: Common, data: T) -> Self {
        Self::new(Some(parent), common, data)
    }

    #[inline]
    pub fn insert_child(&mut self, index: NodeIndex, handle: Handle<T>) {
        let old = self.children.insert(index, handle);
        debug_assert!(old.is_none(), "index duplicate");
    }
}
