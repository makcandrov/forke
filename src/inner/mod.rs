use hashbrown::HashMap;

use crate::NodeData;

mod common;
pub(crate) use common::Common;

mod handle;
pub(crate) use handle::{StrongHandle, WeakHandle};

mod multiplicity;
use multiplicity::Multiplicity;

#[derive(Debug)]
pub(crate) struct NodeInner<T: NodeData> {
    parent: Option<StrongHandle<T>>,
    children: HashMap<u64, WeakHandle<T>>,
    alive: bool,
    index: u64,
    common: Common,
    data: T,
}

impl<T: NodeData> NodeInner<T> {
    pub fn new(parent: Option<StrongHandle<T>>, common: Common, data: T) -> Self {
        Self {
            parent,
            children: HashMap::new(),
            alive: true,
            index: common.next_node_index(),
            common,
            data,
        }
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn parent(&self) -> Option<&StrongHandle<T>> {
        self.parent.as_ref()
    }

    #[inline]
    pub fn root(data: T) -> Self {
        Self::new(None, Common::new(), data)
    }

    #[inline]
    pub fn child(parent: StrongHandle<T>, common: Common, data: T) -> Self {
        Self::new(Some(parent), common, data)
    }

    #[inline]
    pub fn insert_child(&mut self, index: u64, handle: WeakHandle<T>) {
        let old = self.children.insert(index, handle);
        debug_assert!(old.is_none(), "index duplicate");
    }
}
