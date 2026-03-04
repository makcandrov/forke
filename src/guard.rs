
use lock_notify::MappedRwLockNotifyReadGuard;

use crate::{NodeData, inner::{Handle, NodeInner}};

#[derive(Debug)]
pub struct NodeGuard<'a, T: NodeData> {
    guard: MappedRwLockNotifyReadGuard<'a, NodeInner<T>>,
}

impl<'a, T: NodeData> NodeGuard<'a, T> {
    pub fn data(&self) -> &T {
        self.guard.data()
    }

    pub fn parent(&'a self) -> Option<Self> {
        self.guard.parent().as_ref().map(|parent| Self::new(parent))
    }

    pub(crate) fn new(handle: &'a Handle<T>) -> Self {
        Self {
            guard: handle.read_node(),
        }
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
