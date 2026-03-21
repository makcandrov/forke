use crate::{
    Merge, NodeGuard, NodeWriteGuard, OwnedNodeGuard, OwnedNodeWriteGuard, TraverseGuards,
    TraverseIter, TraverseRefIter, inner::StrongHandle,
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
    ///
    /// # Example
    /// ```
    /// use forke::Node;
    ///
    /// let parent = Node::root(vec![1]);
    /// let child = parent.fork(vec![2]);
    /// assert_eq!(*child.guard().data(), vec![2]);
    /// ```
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

    /// Forks this node N times, creating an array of N child nodes.
    ///
    /// # Example
    /// ```
    /// use forke::Node;
    ///
    /// let parent = Node::root(vec![0]);
    /// let [child1, child2, child3] = parent.fork_n([vec![1], vec![2], vec![3]]);
    /// assert_eq!(*child1.guard().data(), vec![1]);
    /// assert_eq!(*child2.guard().data(), vec![2]);
    /// assert_eq!(*child3.guard().data(), vec![3]);
    /// ```
    #[inline]
    #[must_use]
    pub fn fork_n<const N: usize>(&self, data: [T; N]) -> [Self; N] {
        let handles = self.handle.create_children_array(data);
        handles.map(|handle| Self { handle })
    }

    /// Acquires a read lock on this node, borrowing `self`.
    ///
    /// Returns a guard that provides read access to the node's data via [`NodeGuard::data`].
    /// The lock is held as long as the guard exists.
    ///
    /// # Example
    /// ```
    /// # use forke::{Node, Merge};
    /// let root = Node::root(vec![1, 2, 3]);
    /// let guard = root.guard();
    /// assert_eq!(guard.data(), &vec![1, 2, 3]);
    /// // lock released when guard is dropped
    /// ```
    #[inline]
    pub fn guard(&self) -> NodeGuard<'_, T> {
        self.handle.node_guard()
    }

    /// Acquires a read lock on this node with `'static` lifetime.
    /// The returned guard keeps the underlying data alive independently of
    /// the `Node` handle.
    #[inline]
    pub fn owned_guard(&self) -> OwnedNodeGuard<T> {
        self.handle.clone().owned_node_guard()
    }

    /// Acquires a write lock on this node, borrowing `self`.
    /// Provides mutable access to the node's data.
    #[inline]
    pub fn guard_mut(&self) -> NodeWriteGuard<'_, T> {
        self.handle.node_write_guard()
    }

    /// Acquires a write lock on this node with `'static` lifetime.
    /// The returned guard keeps the underlying data alive independently of
    /// the `Node` handle.
    #[inline]
    pub fn owned_guard_mut(&self) -> OwnedNodeWriteGuard<T> {
        self.handle.clone().owned_node_write_guard()
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
    /// value produced by the closure `f`.
    ///
    /// This is useful for searching for a condition in the ancestor chain.
    /// Each node is visited once, starting from this node and continuing up to the root.
    ///
    /// # Example
    /// ```
    /// use forke::Node;
    ///
    /// let root = Node::root(vec![1, 2]);
    /// let child = root.fork(vec![3]);
    /// let found = child.search(|v| (v.len() == 2).then_some("parent length"));
    /// assert_eq!(found, Some("parent length"));
    /// ```
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
