use std::{
    ptr,
    sync::{Arc, Weak},
};

use lockbell::{
    MappedRwLockBellReadGuard, MappedRwLockBellWriteGuard, RwLockBell, RwLockBellReadGuard,
    RwLockBellWriteGuard,
};

use crate::{
    MergeInv, NodeData, NodeGuard,
    guard::{NodeWriteGuard, OwnedNodeGuard, OwnedNodeWriteGuard},
};

use super::{Multiplicity, NodeInner};

#[derive(Debug)]
pub(crate) struct StrongHandle<T: NodeData> {
    inner: Arc<RwLockBell<Option<NodeInner<T>>>>,
    index: u64,
}

#[derive(Debug)]
pub(crate) struct WeakHandle<T: NodeData> {
    inner: Weak<RwLockBell<Option<NodeInner<T>>>>,
    index: u64,
}

impl<T: NodeData> Clone for StrongHandle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            index: self.index,
        }
    }
}

impl<T: NodeData> Clone for WeakHandle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Weak::clone(&self.inner),
            index: self.index,
        }
    }
}

impl<T: NodeData> WeakHandle<T> {
    pub fn upgrade(&self) -> Option<StrongHandle<T>> {
        Some(StrongHandle {
            inner: Weak::upgrade(&self.inner)?,
            index: self.index,
        })
    }
}

impl<T: NodeData> StrongHandle<T> {
    fn new(node: NodeInner<T>) -> Self {
        Self {
            index: node.index,
            inner: Arc::new(RwLockBell::new(Some(node))),
        }
    }

    pub fn root(data: T) -> Self {
        let node = NodeInner::root(data);
        Self::new(node)
    }

    pub fn downgrade(&self) -> WeakHandle<T> {
        WeakHandle {
            inner: Arc::downgrade(&self.inner),
            index: self.index,
        }
    }

    fn ptr_eq(&self, weak: &WeakHandle<T>) -> bool {
        ptr::eq(Arc::as_ptr(&self.inner), Weak::as_ptr(&weak.inner))
    }

    pub fn node_guard<'a>(&'a self) -> NodeGuard<'a, T> {
        NodeGuard::new(self)
    }

    pub fn owned_node_guard(self) -> OwnedNodeGuard<T> {
        OwnedNodeGuard::new(self)
    }

    pub fn node_write_guard<'a>(&'a self) -> NodeWriteGuard<'a, T> {
        NodeWriteGuard::new(self)
    }

    pub fn owned_node_write_guard(self) -> OwnedNodeWriteGuard<T> {
        OwnedNodeWriteGuard::new(self)
    }

    /// Read-locks the node, returning `None` if `inner` has been taken.
    ///
    /// `inner` is only taken by `try_drop`'s `Multiplicity::Single` branch,
    /// which requires `alive = false` — i.e. the user-facing `Node<T>` has
    /// been dropped. Callers anchored by a live `Node<T>` or a descendant's
    /// read lock may safely `.unwrap()` the result.
    pub fn try_read_node<'a>(&'a self) -> Option<MappedRwLockBellReadGuard<'a, NodeInner<T>>> {
        RwLockBellReadGuard::try_map(self.inner.read(), Option::as_ref).ok()
    }

    pub fn write_data<'a>(&'a self) -> MappedRwLockBellWriteGuard<'a, T> {
        // Unwrap soundness: see `try_read_node`.
        RwLockBellWriteGuard::map(self.inner.write(), |inner| {
            &mut inner.as_mut().unwrap().data
        })
    }

    #[inline]
    fn write_node<U>(&self, f: impl FnOnce(&mut NodeInner<T>) -> U) -> U {
        let mut node_guard = self.inner.write();
        // Unwrap soundness: see `try_read_node`.
        f(node_guard.as_mut().unwrap())
    }

    #[inline]
    pub fn create_child(&self, data: T) -> Self {
        self.write_node(|node| self.add_child(node, data))
    }

    #[inline]
    pub fn create_children(&self, data: impl IntoIterator<Item = T>) -> impl Iterator<Item = Self> {
        let mut node_guard = self.inner.write();
        data.into_iter()
            .map(move |data| self.add_child(node_guard.as_mut().unwrap(), data))
    }

    #[inline]
    pub fn create_children_array<const N: usize>(&self, data: [T; N]) -> [Self; N] {
        let mut node_guard = self.inner.write();
        let node = node_guard.as_mut().unwrap();
        let mut data_iter = data.into_iter();
        std::array::from_fn(|_| self.add_child(node, data_iter.next().unwrap()))
    }

    fn add_child(&self, node: &mut NodeInner<T>, data: T) -> StrongHandle<T> {
        let parent_handle = self.clone();
        let child_node = NodeInner::child(parent_handle, node.common.clone(), data);
        self.add_child_inner(node, child_node)
    }

    fn add_child_inner(
        &self,
        node: &mut NodeInner<T>,
        child_node: NodeInner<T>,
    ) -> StrongHandle<T> {
        let child_index = child_node.index;
        let child_handle = StrongHandle::new(child_node);
        node.insert_child(child_index, child_handle.downgrade());
        child_handle
    }

    pub fn try_drop(&mut self, self_drop: bool) {
        let retry_drop = || {
            let mut handle = self.clone();
            move || {
                handle.try_drop(self_drop);
            }
        };

        let mut child_handle_opt: Option<StrongHandle<T>> = None;

        loop {
            // Lock the child, if any.
            let child_guard_opt = if let Some(child_strong_handle) = &child_handle_opt {
                let Some(child_opt_guard) = child_strong_handle.inner.try_write_or_else(retry_drop)
                else {
                    // Contended; defer.
                    return;
                };

                match child_opt_guard.try_map(Option::as_mut) {
                    Ok(child_guard) => Some(child_guard),
                    guard @ Err(_) => {
                        // Child was merged away; restart.
                        drop(guard);
                        child_handle_opt = None;
                        continue;
                    }
                }
            } else {
                None
            };

            // Lock the node.
            let Some(mut node_opt_guard) = self.inner.try_write_or_else(retry_drop) else {
                return;
            };

            let Some(node) = node_opt_guard.as_mut() else {
                // Already merged away by a concurrent retry.
                return;
            };

            let node_index = node.index;

            // Mark dead under the write lock so a cascade from another
            // direction can pass through us even if we bail below.
            if self_drop {
                node.alive = false;
            }

            match Multiplicity::from_iter(&node.children) {
                Multiplicity::None => {
                    // Leaf: remove from tree.
                    if !self_drop && node.alive {
                        break;
                    }

                    let Some(mut parent_handle) = node.parent.take() else {
                        // Root leaf: nothing to unlink.
                        break;
                    };

                    let Some(mut parent_guard) = parent_handle.inner.try_write_or_else(retry_drop)
                    else {
                        node.parent = Some(parent_handle);
                        return;
                    };

                    let parent_node = parent_guard
                        .as_mut()
                        .expect("parent must not be dropped as it has a child");

                    let child_handle = parent_node
                        .children
                        .remove(&node_index)
                        .expect("node is child of parent");

                    drop(child_handle);
                    drop(parent_guard);

                    // Recurse: parent may now be collapsible.
                    parent_handle.try_drop(false);

                    break;
                }
                Multiplicity::Multiple => {
                    // Multiple children: keep the node in place.
                    break;
                }
                Multiplicity::Single((child_index, child_weak_handle)) => {
                    let child_index = *child_index;

                    if child_guard_opt.is_none() {
                        // Child not yet locked; restart with it locked.
                        drop(child_guard_opt);
                        child_handle_opt = Some(child_weak_handle.upgrade().unwrap());
                        continue;
                    };

                    let mut child_guard = child_guard_opt.unwrap();
                    let child_handle = child_handle_opt.as_ref().unwrap();

                    if !child_handle.ptr_eq(child_weak_handle) {
                        // Child changed; re-lock the new one.
                        drop(child_guard);
                        child_handle_opt = Some(child_weak_handle.upgrade().unwrap());
                        continue;
                    }

                    if !self_drop && node.alive {
                        break;
                    }

                    if let Some(parent_handle) = node.parent.clone() {
                        let Some(parent_guard) = parent_handle.inner.try_write_or_else(retry_drop)
                        else {
                            return;
                        };

                        let mut parent_guard =
                            RwLockBellWriteGuard::map(parent_guard, |parent_opt| {
                                parent_opt
                                    .as_mut()
                                    .expect("parent must not be dropped as a child exists")
                            });

                        let node_owned = node_opt_guard.take().unwrap();
                        let parent_handle = node_owned.parent.expect("node has a parent");

                        let child_handle = {
                            let (child_index_confirm, child_handle) =
                                Multiplicity::from_iter(node_owned.children)
                                    .into_single()
                                    .expect("node has a single child");

                            assert_eq!(child_index, child_index_confirm);
                            child_handle
                        };

                        // Relink parent ↔ child, skipping this node.
                        let node_child_handle = parent_guard
                            .children
                            .remove(&node_index)
                            .expect("node is child of parent");
                        drop(node_child_handle);
                        parent_guard.insert_child(child_index, child_handle);

                        let node_parent_handle = child_guard
                            .parent
                            .replace(parent_handle)
                            .expect("child has a parent");
                        // Stale handle: just decrement, no cascade.
                        drop(node_parent_handle);

                        MergeInv::merge_inv(&mut child_guard.data, node_owned.data);

                        break;
                    } else {
                        // Root: child becomes the new root.
                        let node_owned = node_opt_guard.take().unwrap();

                        let (child_index_confirm, child_handle) =
                            Multiplicity::from_iter(node_owned.children)
                                .into_single()
                                .expect("node has a single child");
                        assert_eq!(child_index, child_index_confirm);
                        drop(child_handle);

                        let node_parent_handle =
                            child_guard.parent.take().expect("child has a parent");
                        // Stale handle: just decrement, no cascade.
                        drop(node_parent_handle);

                        MergeInv::merge_inv(&mut child_guard.data, node_owned.data);
                        break;
                    }
                }
            }
        }
    }
}
