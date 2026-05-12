use crate::{NodeData, OwnedNodeGuard, inner::StrongHandle};

/// Walks from a node up to the root, yielding [`OwnedNodeGuard<T>`] values.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct TraverseIter<T: NodeData> {
    /// Guard for the next node to yield. `new` locks the starting node;
    /// each `next` swaps this for the parent guard, acquired while the
    /// current is still held to anchor it against a concurrent cascade.
    current: Option<OwnedNodeGuard<T>>,
}

impl<T: NodeData> TraverseIter<T> {
    pub(crate) fn new(start: &StrongHandle<T>) -> Self {
        Self {
            current: Some(OwnedNodeGuard::new(start.clone())),
        }
    }
}

impl<T: NodeData> Iterator for TraverseIter<T> {
    type Item = OwnedNodeGuard<T>;

    #[inline]
    fn next(&mut self) -> Option<OwnedNodeGuard<T>> {
        let current = self.current.take()?;
        if let Some(parent_handle) = current.parent_handle().cloned() {
            self.current = Some(OwnedNodeGuard::new(parent_handle));
        }
        Some(current)
    }
}

/// Storage for the guards accumulated by [`crate::Node::traverse_ref`].
/// Yielded `&T` references stay valid for the lifetime of the borrow.
pub struct TraverseGuards<T: NodeData> {
    guards: Vec<OwnedNodeGuard<T>>,
}

impl<T: NodeData> TraverseGuards<T> {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self { guards: Vec::new() }
    }
}

impl<T: NodeData> Default for TraverseGuards<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Walks from a node up to the root, yielding `&'a T` references valid for
/// the lifetime of the [`TraverseGuards`] borrow. Read locks accumulate in
/// `storage` so concurrent merges can't invalidate the references.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct TraverseRefIter<'a, T: NodeData> {
    current: Option<OwnedNodeGuard<T>>,
    storage: &'a mut TraverseGuards<T>,
}

impl<'a, T: NodeData> TraverseRefIter<'a, T> {
    pub(crate) fn new(start: &StrongHandle<T>, storage: &'a mut TraverseGuards<T>) -> Self {
        Self {
            current: Some(OwnedNodeGuard::new(start.clone())),
            storage,
        }
    }
}

impl<'a, T: NodeData> Iterator for TraverseRefIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        let current = self.current.take()?;

        // Lock the parent while `current` is still held, then push `current`
        // into storage.
        if let Some(parent_handle) = current.parent_handle().cloned() {
            self.current = Some(OwnedNodeGuard::new(parent_handle));
        }
        self.storage.guards.push(current);

        // SAFETY: data lives in the Arc heap (stable address). The guard we
        // just pushed keeps the Arc alive, and the `&'a mut storage` borrow
        // blocks any caller from removing it while &'a T is live.
        let data: *const T = self.storage.guards.last().unwrap().data();
        Some(unsafe { &*data })
    }
}
