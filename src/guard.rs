use std::mem::transmute;

use lockbell::{MappedRwLockBellReadGuard, MappedRwLockBellWriteGuard};

use crate::{
    NodeData,
    inner::{NodeInner, StrongHandle},
};

/// Borrowed read-lock guard on a node.
#[derive(Debug)]
#[must_use = "if unused the lock is immediately released"]
pub struct NodeGuard<'a, T: NodeData> {
    guard: MappedRwLockBellReadGuard<'a, NodeInner<T>>,
}

/// Owned read-lock guard on a node. Unlike [`NodeGuard`], keeps the node
/// allocation alive on its own.
#[derive(Debug)]
#[must_use = "if unused the lock is immediately released"]
pub struct OwnedNodeGuard<T: NodeData> {
    // SAFETY: `guard`'s `'static` lifetime is a lie — it really borrows from
    // `_handle`'s Arc. Field declaration order makes `guard` drop first,
    // keeping the Arc alive for `guard`'s entire lifetime.
    guard: NodeGuard<'static, T>,
    _handle: StrongHandle<T>,
}

/// Borrowed write-lock guard on a node.
#[derive(Debug)]
#[must_use = "if unused the lock is immediately released"]
pub struct NodeWriteGuard<'a, T: NodeData> {
    guard: MappedRwLockBellWriteGuard<'a, T>,
}

/// Owned write-lock guard on a node. Unlike [`NodeWriteGuard`], keeps the
/// node allocation alive on its own.
#[derive(Debug)]
#[must_use = "if unused the lock is immediately released"]
pub struct OwnedNodeWriteGuard<T: NodeData> {
    // SAFETY: see `OwnedNodeGuard`.
    guard: NodeWriteGuard<'static, T>,
    _handle: StrongHandle<T>,
}

impl<'a, T: NodeData> NodeGuard<'a, T> {
    /// Returns a reference to the node's data.
    #[inline]
    pub fn data(&self) -> &T {
        self.guard.data()
    }

    /// Read-locks the parent node, if any.
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
            guard: handle.try_read_node().unwrap(),
        }
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

        let mut current = OwnedNodeGuard::new(self.parent_handle()?.clone());
        loop {
            if let Some(res) = f(current.data()) {
                return Some(res);
            }
            current = OwnedNodeGuard::new(current.parent_handle().cloned()?);
        }
    }
}

impl<T: NodeData> OwnedNodeGuard<T> {
    pub(crate) fn new(handle: StrongHandle<T>) -> Self {
        // SAFETY: see `OwnedNodeGuard`'s field-order invariant.
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

    /// Read-locks the parent node, if any.
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

impl<'a, T: NodeData> NodeWriteGuard<'a, T> {
    /// Returns a reference to the node's data.
    #[inline]
    pub fn data(&self) -> &T {
        &self.guard
    }

    /// Returns a mutable reference to the node's data.
    #[inline]
    pub fn data_mut(&mut self) -> &mut T {
        &mut self.guard
    }

    pub(crate) fn new(handle: &'a StrongHandle<T>) -> Self {
        Self {
            guard: handle.write_data(),
        }
    }
}

impl<T: NodeData> OwnedNodeWriteGuard<T> {
    pub(crate) fn new(handle: StrongHandle<T>) -> Self {
        // SAFETY: see `OwnedNodeGuard::new`.
        Self {
            guard: unsafe {
                transmute::<NodeWriteGuard<'_, T>, NodeWriteGuard<'static, T>>(NodeWriteGuard::new(
                    &handle,
                ))
            },
            _handle: handle,
        }
    }

    /// Returns a reference to the node's data.
    #[inline]
    pub fn data(&self) -> &T {
        self.guard.data()
    }

    /// Returns a mutable reference to the node's data.
    #[inline]
    pub fn data_mut(&mut self) -> &mut T {
        self.guard.data_mut()
    }
}
