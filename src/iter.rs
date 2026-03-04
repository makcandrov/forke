use std::mem;

use lock_notify::MappedRwLockNotifyReadGuard;

use crate::{
    NodeData,
    inner::{Handle, NodeInner},
};

/// A guard holding a read lock on a node, yielded by [`AncestorIter`].
///
/// The lock is released when this guard is dropped.
pub struct AncestorGuard<T: NodeData> {
    // `guard` must be declared before `handle` so it is dropped first,
    // releasing the read lock before the Arc inside `handle` is decremented.
    //
    // SAFETY invariant: `guard` holds a read lock on the RwLock that lives
    // inside `handle`'s Arc. The `'static` lifetime is a lie: the true
    // lifetime is that of the Arc, which is kept alive by `handle` below.
    // Field drop order (guard first, handle second) upholds this invariant.
    guard: MappedRwLockNotifyReadGuard<'static, NodeInner<T>>,
    _handle: Handle<T>,
}

impl<T: NodeData> AncestorGuard<T> {
    fn new(handle: Handle<T>) -> Self {
        // SAFETY: The guard holds a read lock on the RwLock stored inside
        // `handle`'s Arc. That Arc — and thus the locked data — lives at
        // least as long as `handle`. We store `handle` in the same struct
        // and declare it after `guard`, so Rust drops `guard` (releasing
        // the lock) before dropping `handle` (decrementing the Arc).
        // The `'static` lifetime is therefore sound.
        let guard = unsafe {
            mem::transmute::<
                MappedRwLockNotifyReadGuard<'_, NodeInner<T>>,
                MappedRwLockNotifyReadGuard<'static, NodeInner<T>>,
            >(handle.read_node())
        };
        Self { guard, _handle: handle }
    }

    pub fn data(&self) -> &T {
        self.guard.data()
    }
}

impl<T: NodeData> std::ops::Deref for AncestorGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.guard.data()
    }
}

/// Iterator that walks from a node up to the root, locking each node on
/// demand when [`Iterator::next`] is called.
pub struct AncestorIter<T: NodeData> {
    current: Option<Handle<T>>,
}

impl<T: NodeData> AncestorIter<T> {
    pub(crate) fn new(start: &Handle<T>) -> Self {
        Self {
            current: Some(start.clone()),
        }
    }
}

impl<T: NodeData> Iterator for AncestorIter<T> {
    type Item = AncestorGuard<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let handle = self.current.take()?;
        let ancestor_guard = AncestorGuard::new(handle);
        // Advance to the parent *before* returning, so the iterator state is
        // consistent even if the caller holds the guard for a long time.
        self.current = ancestor_guard.guard.parent().cloned();
        Some(ancestor_guard)
    }
}
