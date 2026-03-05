use std::mem::transmute;

use lock_notify::MappedRwLockNotifyReadGuard;

use crate::{
    NodeData,
    inner::{NodeInner, StrongHandle},
};

#[derive(Debug)]
pub struct NodeGuard<'a, T: NodeData> {
    guard: MappedRwLockNotifyReadGuard<'a, NodeInner<T>>,
}

#[derive(Debug)]
pub struct StaticNodeGuard<T: NodeData> {
    // guard dropped before _handle (field declaration order)
    //
    // SAFETY invariant: `guard` holds a read lock on a RwLock that lives inside
    // `_handle`'s Arc. The `'static` lifetime on `guard` is a lie: the true
    // lifetime is that of the Arc, kept alive by `_handle`.
    // Field drop order (guard first, _handle second) upholds this invariant.
    guard: NodeGuard<'static, T>,
    _handle: StrongHandle<T>,
}

impl<'a, T: NodeData> NodeGuard<'a, T> {
    pub fn data(&self) -> &T {
        self.guard.data()
    }

    pub fn parent(&'a self) -> Option<Self> {
        self.parent_handle().map(|parent| Self::new(parent))
    }

    pub(crate) fn parent_handle(&self) -> Option<&StrongHandle<T>> {
        self.guard.parent()
    }

    pub(crate) fn new(handle: &'a StrongHandle<T>) -> Self {
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

impl<T: NodeData> StaticNodeGuard<T> {
    pub(crate) fn new(handle: StrongHandle<T>) -> Self {
        // SAFETY: `guard` borrows from the Arc inside `handle`. Both are stored
        // together in `StaticNodeGuard`, which drops `guard` before `_handle`,
        // keeping the Arc alive for the guard's entire lifetime. The `'static`
        // lie is sound: the Arc is kept alive by `_handle`.
        Self {
            guard: unsafe {
                transmute::<NodeGuard<'_, T>, NodeGuard<'static, T>>(NodeGuard::new(&handle))
            },
            _handle: handle,
        }
    }

    pub fn data(&self) -> &T {
        self.guard.data()
    }

    pub(crate) fn parent_handle(&self) -> Option<&StrongHandle<T>> {
        self.guard.guard.parent()
    }

    pub fn parent(&self) -> Option<Self> {
        self.parent_handle().map(|parent| Self::new(parent.clone()))
    }
}
