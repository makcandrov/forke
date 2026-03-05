use crate::{NodeData, StaticNodeGuard, inner::StrongHandle};

/// Stable storage for the read-lock guards accumulated during ancestor
/// traversal. Create one with [`AncestorGuards::new`] and pass a mutable
/// reference to [`crate::Node::iter`]. The `&T` references yielded by the iterator
/// are valid for the lifetime of this borrow — the borrow checker prevents
/// any removal of guards while those references are alive.
pub struct AncestorGuards<T: NodeData> {
    guards: Vec<StaticNodeGuard<T>>,
}

impl<T: NodeData> AncestorGuards<T> {
    pub fn new() -> Self {
        Self { guards: Vec::new() }
    }
}

impl<T: NodeData> Default for AncestorGuards<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator that walks from a node up to the root, yielding `&'a T`
/// references valid for the lifetime `'a` of the [`AncestorGuards`] borrow.
/// Implements [`std::iter::Iterator`]; guards accumulate in the external
/// storage so read locks on every visited node are held for `'a`, preventing
/// concurrent merges from invalidating the returned references.
pub struct AncestorIter<'a, T: NodeData> {
    current: Option<StrongHandle<T>>,
    storage: &'a mut AncestorGuards<T>,
}

impl<'a, T: NodeData> AncestorIter<'a, T> {
    pub(crate) fn new(start: &StrongHandle<T>, storage: &'a mut AncestorGuards<T>) -> Self {
        Self {
            current: Some(start.clone()),
            storage,
        }
    }
}

impl<'a, T: NodeData> Iterator for AncestorIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        let handle = self.current.take()?;
        let static_guard = handle.static_node_guard();

        self.current = static_guard.parent_handle().cloned();
        self.storage.guards.push(static_guard);

        // SAFETY: `data` points into the Arc heap allocation, which is at a
        // stable address independent of Vec layout. The guard keeping the Arc
        // alive was just pushed into `self.storage`, which is mutably borrowed
        // for 'a. The borrow checker prevents any code from removing guards
        // from `storage` while any &'a T derived from this call is alive,
        // so the data remains valid for 'a.
        let data: *const T = self.storage.guards.last().unwrap().data();
        Some(unsafe { &*data })
    }
}
