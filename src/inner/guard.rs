use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};

use crate::Merge;

use super::{NodeInner, ParentHandle, SelfHandle};

#[derive(Debug)]
pub struct NodeGuard<'a, T: Merge> {
    node_guard: MappedRwLockReadGuard<'a, NodeInner<T>>,
}

impl<'a, T: Merge> NodeGuard<'a, T> {
    pub fn data(&self) -> &T {
        &self.node_guard.data
    }

    pub fn parent(&'a self) -> Option<Self> {
        self.node_guard.parent.as_ref().map(|parent_handle| Self {
            node_guard: parent_handle.inner.read_guard(),
        })
    }

    pub(super) fn new(node_lock: &'a RwLock<Option<NodeInner<T>>>) -> Self {
        let node_guard = RwLockReadGuard::map(node_lock.read(), |node_opt| {
            node_opt
                .as_ref()
                .expect("node isn't dropped as a user handle exists")
        });

        Self { node_guard }
    }

    pub fn search<U, F>(&self, f: F) -> Option<U>
    where
        F: Fn(&T) -> Option<U>,
    {
        if let Some(res) = f(self.data()) {
            Some(res)
        } else if let Some(parent) = self.parent() {
            parent.search(f)
        } else {
            None
        }
    }
}
