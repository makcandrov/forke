use crate::{
    Merge, NodeGuard, NodeWriteGuard, OwnedNodeGuard, OwnedNodeWriteGuard,
    inner::StrongHandle,
    iter::{TraverseGuards, TraverseIter, TraverseRefIter},
};

/// Bound alias for types storable in a [`Node`].
pub trait NodeData: Merge + Send + Sync + 'static {}
impl<T> NodeData for T where T: Merge + Send + Sync + 'static {}

/// Handle to a node in the fork tree.
///
/// Dropping a `Node` marks it dead. If it has zero or one children it is
/// removed from the tree and its data is folded via [`Merge`].
///
/// # Lock reentrancy
///
/// Each node is protected by a non-reentrant read-write lock. Calling a
/// write-acquiring method ([`fork`], [`fork_many`], [`fork_n`],
/// [`guard_mut`], [`owned_guard_mut`]) on a node while the same thread
/// already holds a guard on that node deadlocks, exactly like re-locking a
/// [`std::sync::Mutex`]. Re-acquiring a read guard on a node the thread
/// already holds one for ([`guard`], [`traverse`], [`search`]) can also
/// deadlock if another thread starts waiting for a write lock in between.
///
/// Dropping a `Node` never blocks: if a lock is contended, the removal is
/// deferred and retried once the lock is released.
///
/// [`fork`]: Self::fork
/// [`fork_many`]: Self::fork_many
/// [`fork_n`]: Self::fork_n
/// [`guard`]: Self::guard
/// [`guard_mut`]: Self::guard_mut
/// [`owned_guard_mut`]: Self::owned_guard_mut
/// [`traverse`]: Self::traverse
/// [`search`]: Self::search
#[derive(Debug)]
pub struct Node<T: NodeData> {
    handle: StrongHandle<T>,
}

impl<T: NodeData + Default> Default for Node<T> {
    fn default() -> Self {
        Self::root(T::default())
    }
}

impl<T: NodeData> Node<T> {
    /// Creates a new root node.
    #[inline]
    #[must_use]
    pub fn root(data: T) -> Self {
        Self {
            handle: StrongHandle::root(data),
        }
    }

    /// Forks this node, creating a child.
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

    /// Forks this node multiple times.
    ///
    /// The returned iterator holds this node's write lock until dropped.
    /// Consume or drop it promptly; holding it across unrelated work blocks
    /// every other reader and writer of this node.
    #[inline]
    pub fn fork_many(&self, data: impl IntoIterator<Item = T>) -> impl Iterator<Item = Self> {
        self.handle
            .create_children(data)
            .map(|handle| Self { handle })
    }

    /// Forks this node `N` times, returning an array of children.
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
    /// # Example
    /// ```
    /// # use forke::{Node, Merge};
    /// let root = Node::root(vec![1, 2, 3]);
    /// let guard = root.guard();
    /// assert_eq!(guard.data(), &vec![1, 2, 3]);
    /// ```
    #[inline]
    pub fn guard(&self) -> NodeGuard<'_, T> {
        self.handle.node_guard()
    }

    /// Acquires an owned read-lock guard. Keeps the node alive
    /// independently of this `Node` handle.
    #[inline]
    pub fn owned_guard(&self) -> OwnedNodeGuard<T> {
        self.handle.clone().owned_node_guard()
    }

    /// Acquires a write lock on this node, borrowing `self`.
    #[inline]
    pub fn guard_mut(&self) -> NodeWriteGuard<'_, T> {
        self.handle.node_write_guard()
    }

    /// Acquires an owned write-lock guard. Keeps the node alive
    /// independently of this `Node` handle.
    #[inline]
    pub fn owned_guard_mut(&self) -> OwnedNodeWriteGuard<T> {
        self.handle.clone().owned_node_write_guard()
    }

    /// Iterator from this node up to the root, yielding an
    /// [`OwnedNodeGuard`] for each visited node.
    #[inline]
    pub fn traverse(&self) -> TraverseIter<T> {
        TraverseIter::new(&self.handle)
    }

    /// Iterator from this node up to the root, yielding `&T` references.
    /// Read locks accumulate in `guards` for the lifetime of the borrow.
    #[inline]
    pub fn traverse_ref<'a>(&self, guards: &'a mut TraverseGuards<T>) -> TraverseRefIter<'a, T> {
        TraverseRefIter::new(&self.handle, guards)
    }

    /// Walks from this node up to the root, returning the first non-`None`
    /// value produced by `f`. Each ancestor is visited once.
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
