use crate::{
    Merge, NodeGuard, OwnedNodeGuard, TraverseGuards, TraverseIter, TraverseRefIter,
    inner::StrongHandle,
};

/// Convenience bound alias: types stored in a [`Node`] must implement
/// [`Merge`], [`Send`], [`Sync`], and be `'static`.
pub trait NodeData: Merge + Send + Sync + 'static {}
impl<T> NodeData for T where T: Merge + Send + Sync + 'static {}

/// A handle to a node in the fork tree.
///
/// Dropping a `Node` marks it as dead. If the node has zero or one children
/// it is removed from the tree and its data is merged into its neighbours
/// via [`Merge`].
#[derive(Debug)]
pub struct Node<T: NodeData> {
    handle: StrongHandle<T>,
}

impl<T: NodeData> Node<T> {
    /// Creates a new root node with the given data.
    #[inline]
    #[must_use]
    pub fn root(data: T) -> Self {
        Self {
            handle: StrongHandle::root(data),
        }
    }

    /// Forks this node, creating a child with the given data.
    #[inline]
    #[must_use]
    pub fn fork(&self, data: T) -> Self {
        Self {
            handle: self.handle.create_child(data),
        }
    }

    /// Forks this node multiple times, returning handles to all children.
    #[inline]
    #[must_use]
    pub fn fork_many(&self, data: impl IntoIterator<Item = T>) -> Vec<Self> {
        self.handle
            .create_children(data)
            .map(|handle| Self { handle })
            .collect()
    }

    /// Acquires a read lock on this node, borrowing `self`.
    #[inline]
    pub fn guard(&self) -> NodeGuard<'_, T> {
        self.handle.node_guard()
    }

    /// Acquires a read lock on this node with `'static` lifetime.
    /// The returned guard keeps the underlying data alive independently of
    /// the `Node` handle.
    #[inline]
    pub fn owned_guard(&self) -> OwnedNodeGuard<T> {
        self.handle.clone().static_node_guard()
    }

    /// Returns an iterator that walks from this node up to the root,
    /// yielding an [`OwnedNodeGuard`] for each visited node. Each guard is
    /// independent — dropping it releases the read lock on that node.
    #[inline]
    pub fn traverse(&self) -> TraverseIter<T> {
        TraverseIter::new(&self.handle)
    }

    /// Returns an iterator that walks from this node up to the root,
    /// yielding `&T` references. Guards are accumulated in `guards` so all
    /// read locks are held for the lifetime of the borrow.
    #[inline]
    pub fn traverse_ref<'a>(&self, guards: &'a mut TraverseGuards<T>) -> TraverseRefIter<'a, T> {
        TraverseRefIter::new(&self.handle, guards)
    }

    /// Walks from this node up to the root, returning the first non-`None`
    /// value produced by `f`.
    #[inline]
    #[must_use]
    pub fn search<U, F>(&self, f: F) -> Option<U>
    where
        F: Fn(&T) -> Option<U>,
    {
        self.traverse().find_map(|g| f(g.data()))
    }
}

impl<T: NodeData> Drop for Node<T> {
    fn drop(&mut self) {
        self.handle.try_drop(true);
    }
}
