use std::{
    ptr,
    sync::{Arc, Weak},
};

use lock_notify::{
    MappedRwLockNotifyReadGuard, RwLockNotify, RwLockNotifyReadGuard, RwLockNotifyWriteGuard,
};

use crate::{Merge, MergeInv, NodeData, inner::common::Common};

use super::{Multiplicity, NodeInner};

#[derive(Debug)]
pub struct Handle<T: NodeData> {
    inner: Arc<RwLockNotify<Option<NodeInner<T>>>>,
    index: u64,
}

impl<T: NodeData> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            index: self.index,
        }
    }
}

impl<T: NodeData> Handle<T> {
    fn new(node: NodeInner<T>) -> Self {
        Self {
            index: node.index,
            inner: Arc::new(RwLockNotify::new(Some(node))),
        }
    }

    pub fn root(data: T) -> Self {
        let node = NodeInner::root(data);
        Self::new(node)
    }

    pub fn read_node<'a>(&'a self) -> MappedRwLockNotifyReadGuard<'a, NodeInner<T>> {
        RwLockNotifyReadGuard::map(self.inner.read(), |inner| inner.as_ref().unwrap())
    }

    #[inline]
    fn write_node<U>(&self, f: impl FnOnce(&mut NodeInner<T>) -> U) -> U {
        let mut node_guard = self.inner.write();
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

    fn add_child(&self, node: &mut NodeInner<T>, data: T) -> Handle<T> {
        let parent_handle = self.clone();
        let child_node = NodeInner::child(parent_handle, node.common.clone(), data);
        self.add_child_inner(node, child_node)
    }

    fn add_child_inner(&self, node: &mut NodeInner<T>, child_node: NodeInner<T>) -> Handle<T> {
        let child_index = child_node.index;
        let child_handle = Handle::new(child_node);
        node.insert_child(child_index, child_handle.clone());
        child_handle
    }

    pub fn try_drop(&mut self, self_drop: bool) {
        let retry_drop = || {
            let mut handle = self.clone();
            move || {
                handle.try_drop(self_drop);
            }
        };

        let mut child_handle_opt: Option<Handle<T>> = None;

        loop {
            // Lock the child if there is some.
            let child_guard_opt = if let Some(child_strong_handle) = &child_handle_opt {
                let Some(child_opt_guard) =
                    child_strong_handle.inner.try_write_or_else(&retry_drop)
                else {
                    // Another thread reads the child - delay the drop.
                    return;
                };

                match child_opt_guard.try_map(Option::as_mut) {
                    Ok(child_guard) => Some(child_guard),
                    guard @ Err(_) => {
                        drop(guard);
                        // The child may have been dropped in the meantime. We need to start everything over.
                        child_handle_opt = None;
                        continue;
                    }
                }
            } else {
                None
            };

            // Lock the node.
            let Some(mut node_opt_guard) = self.inner.try_write_or_else(&retry_drop) else {
                // Another thread reads the node - delay the drop.
                return;
            };

            let Some(node) = node_opt_guard.as_ref() else {
                // The user dropped the node in the meantime.
                assert!(!self_drop);
                return;
            };

            let node_index = node.index;

            match Multiplicity::from_iter(&node.children) {
                Multiplicity::None => {
                    // No children, the node is a leaf that can be removed from the tree.

                    let node = node_opt_guard.as_mut().unwrap();

                    let Some(mut parent_handle) = node.parent.take() else {
                        // If this is the root, there is nothing to do.
                        break;
                    };

                    let Some(mut parent_guard) = parent_handle.inner.try_write_or_else(&retry_drop)
                    else {
                        // Another thread reads the parent - delay the drop.
                        node.parent = Some(parent_handle);
                        return;
                    };

                    node.alive = false;

                    let parent_node = parent_guard
                        .as_mut()
                        .expect("parent must not be dropped as it has a child");

                    let child_handle = parent_node
                        .children
                        .remove(&node_index)
                        .expect("node is child of parent");

                    drop(child_handle);
                    drop(parent_guard);

                    // The node is removed from the tree.
                    // We need to recursively drop its parent if necessary.
                    parent_handle.try_drop(false);

                    break;
                }
                Multiplicity::Multiple => {
                    // Two children or more, the node must not be dropped.

                    let node = node_opt_guard.as_mut().unwrap();
                    if self_drop {
                        node.alive = false;
                    }
                    break;
                }
                Multiplicity::Single((child_index, child_handle_confirm)) => {
                    let child_index = *child_index;

                    if child_guard_opt.is_none() {
                        // The node has a children that has not yet been locked.
                        // We need to start everything over to lock the child first.

                        drop(child_guard_opt);
                        child_handle_opt = Some(child_handle_confirm.clone());
                        continue;
                    };

                    let mut child_guard = child_guard_opt.unwrap(); // verified above
                    let child_handle = child_handle_opt.as_ref().unwrap(); // verified above

                    if !ptr::eq(
                        Arc::as_ptr(&child_handle.inner),
                        Arc::as_ptr(&child_handle_confirm.inner),
                    ) {
                        // Child has changed in the meantime.
                        drop(child_guard);
                        child_handle_opt = Some(child_handle_confirm.clone());
                        continue;
                    }

                    if let Some(parent_handle) = node.parent.clone() {
                        // The node isn't the root.

                        let Some(parent_guard) = parent_handle.inner.try_write_or_else(retry_drop)
                        else {
                            // Another thread reads the parent - delay the drop.
                            return;
                        };

                        let mut parent_guard =
                            RwLockNotifyWriteGuard::map(parent_guard, |parent_opt| {
                                parent_opt
                                    .as_mut()
                                    .expect("parent must not be dropped as a child exists")
                            });

                        // We can take ownership of node.

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

                        // Link parent to child

                        let node_child_handle = parent_guard
                            .children
                            .remove(&node_index)
                            .expect("node is child of parent");
                        drop(node_child_handle);

                        parent_guard.insert_child(child_index, child_handle);

                        // Link child to parent

                        let node_parent_handle = child_guard
                            .parent
                            .replace(parent_handle)
                            .expect("child has a parent");

                        // This handle is stale, do not run custom drop logic.
                        drop(node_parent_handle);

                        // Merge data
                        MergeInv::merge_inv(&mut child_guard.data, node_owned.data);

                        break;
                    } else {
                        // The node is the root.
                        // We can take ownership of node.

                        let node_owned = node_opt_guard.take().unwrap();

                        let (child_index_confirm, child_handle) =
                            Multiplicity::from_iter(node_owned.children)
                                .into_single()
                                .expect("node has a single child");
                        assert_eq!(child_index, child_index_confirm);
                        drop(child_handle);

                        // Remove parent from the child.
                        let node_parent_handle =
                            child_guard.parent.take().expect("child has a parent");

                        // This handle is stale, do not run custom drop logic.
                        drop(node_parent_handle);

                        // Merge data
                        MergeInv::merge_inv(&mut child_guard.data, node_owned.data);
                        break;
                    }
                }
            }
        }
    }
}
