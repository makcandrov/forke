use crate::{NodeData, OwnedNodeGuard, inner::StrongHandle};

/// Iterator that walks from a node up to the root, yielding
/// [`OwnedNodeGuard<T>`] values. Each guard holds a read lock on the visited
/// node; dropping the guard releases the lock.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct TraverseIter<T: NodeData> {
    current: Option<StrongHandle<T>>,
}

impl<T: NodeData> TraverseIter<T> {
    pub(crate) fn new(start: &StrongHandle<T>) -> Self {
        Self {
            current: Some(start.clone()),
        }
    }
}

impl<T: NodeData> Iterator for TraverseIter<T> {
    type Item = OwnedNodeGuard<T>;

    #[inline]
    fn next(&mut self) -> Option<OwnedNodeGuard<T>> {
        let handle = self.current.take()?;
        let guard = handle.owned_node_guard();
        self.current = guard.parent_handle().cloned();
        Some(guard)
    }
}

/// Stable storage for the read-lock guards accumulated during traversal.
/// Create one with [`TraverseGuards::new`] and pass a mutable reference to
/// [`crate::Node::traverse_ref`]. The `&T` references yielded by the iterator
/// are valid for the lifetime of this borrow.
pub struct TraverseGuards<T: NodeData> {
    guards: Vec<OwnedNodeGuard<T>>,
}

impl<T: NodeData> TraverseGuards<T> {
    /// Creates an empty guard storage.
    #[inline]
    pub fn new() -> Self {
        Self { guards: Vec::new() }
    }
}

impl<T: NodeData> Default for TraverseGuards<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator that walks from a node up to the root, yielding `&'a T`
/// references valid for the lifetime `'a` of the [`TraverseGuards`] borrow.
/// Guards accumulate in the external storage so read locks on every visited
/// node are held for `'a`, preventing concurrent merges from invalidating the
/// returned references.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct TraverseRefIter<'a, T: NodeData> {
    current: Option<StrongHandle<T>>,
    storage: &'a mut TraverseGuards<T>,
}

impl<'a, T: NodeData> TraverseRefIter<'a, T> {
    pub(crate) fn new(start: &StrongHandle<T>, storage: &'a mut TraverseGuards<T>) -> Self {
        Self {
            current: Some(start.clone()),
            storage,
        }
    }
}

impl<'a, T: NodeData> Iterator for TraverseRefIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        let handle = self.current.take()?;
        let static_guard = handle.owned_node_guard();

        self.current = static_guard.parent_handle().cloned();
        self.storage.guards.push(static_guard);

        // SAFETY: `data` points into the Arc heap allocation, which is at a
        // stable address independent of Vec layout. The guard we just pushed
        // into `self.storage` keeps the Arc alive, which is mutably borrowed
        // for 'a. The borrow checker prevents any code from removing guards
        // from `storage` while any &'a T derived from this call is alive,
        // so the data remains valid for 'a.
        let data: *const T = self.storage.guards.last().unwrap().data();
        Some(unsafe { &*data })
    }
}
