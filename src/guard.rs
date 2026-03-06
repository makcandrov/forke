use std::mem::transmute;

use lock_notify::MappedRwLockNotifyReadGuard;

use crate::{
    NodeData,
    inner::{NodeInner, StrongHandle},
};

/// A read-lock guard on a node, borrowed from a [`crate::Node`] handle.
/// Provides access to the node's data and its parent.
#[derive(Debug)]
#[must_use = "if unused the lock is immediately released"]
pub struct NodeGuard<'a, T: NodeData> {
    guard: MappedRwLockNotifyReadGuard<'a, NodeInner<T>>,
}

/// An owned read-lock guard on a node. Unlike [`NodeGuard`], this is not
/// tied to the lifetime of a [`crate::Node`] handle — it keeps the
/// underlying allocation alive on its own.
#[derive(Debug)]
#[must_use = "if unused the lock is immediately released"]
pub struct OwnedNodeGuard<T: NodeData> {
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
    /// Returns a reference to the node's data.
    #[inline]
    pub fn data(&self) -> &T {
        self.guard.data()
    }

    /// Acquires a read lock on the parent node, if any.
    #[inline]
    #[must_use]
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

    /// Walks from this node up to the root, returning the first non-`None`
    /// value produced by `f`.
    #[must_use]
    pub fn recursive_search<U, F>(&self, f: F) -> Option<U>
    where
        F: Fn(&T) -> Option<U>,
    {
        if let Some(res) = f(self.data()) {
            Some(res)
        } else if let Some(parent) = self.parent() {
            parent.recursive_search(f)
        } else {
            None
        }
    }
}

impl<T: NodeData> OwnedNodeGuard<T> {
    pub(crate) fn new(handle: StrongHandle<T>) -> Self {
        // SAFETY: `guard` borrows from the Arc inside `handle`. Both are stored
        // together in `OwnedNodeGuard`, which drops `guard` before `_handle`,
        // keeping the Arc alive for the guard's entire lifetime. The `'static`
        // lie is sound: the Arc is kept alive by `_handle`.
        Self {
            guard: unsafe {
                transmute::<NodeGuard<'_, T>, NodeGuard<'static, T>>(NodeGuard::new(&handle))
            },
            _handle: handle,
        }
    }

    /// Returns a reference to the node's data.
    #[inline]
    pub fn data(&self) -> &T {
        self.guard.data()
    }

    pub(crate) fn parent_handle(&self) -> Option<&StrongHandle<T>> {
        self.guard.guard.parent()
    }

    /// Acquires a read lock on the parent node, if any.
    #[inline]
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        self.parent_handle().map(|parent| Self::new(parent.clone()))
    }

    /// Walks from this node up to the root, returning the first non-`None`
    /// value produced by `f`.
    #[must_use]
    pub fn search<U, F>(&self, f: F) -> Option<U>
    where
        F: Fn(&T) -> Option<U>,
    {
        if let Some(res) = f(self.data()) {
            return Some(res);
        }
        let mut current = self.parent();
        while let Some(guard) = current {
            if let Some(res) = f(guard.data()) {
                return Some(res);
            }
            current = guard.parent();
        }
        None
    }
}
