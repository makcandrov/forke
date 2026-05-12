use crate::NodeData;

mod common;
pub(crate) use common::Common;

mod handle;
pub(crate) use handle::{StrongHandle, WeakHandle};

mod map;
use map::ChildrenMap;

#[derive(Debug)]
pub(crate) struct NodeInner<T: NodeData> {
    parent: Option<StrongHandle<T>>,
    children: ChildrenMap<T>,
    alive: bool,
    index: u64,
    common: Common,
    data: T,
}

impl<T: NodeData> NodeInner<T> {
    pub fn new(parent: Option<StrongHandle<T>>, common: Common, data: T) -> Self {
        Self {
            parent,
            children: ChildrenMap::new(),
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
        self.children.insert(index, handle);
    }
}
