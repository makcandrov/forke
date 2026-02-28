use std::{
    ptr,
    sync::{Arc, Weak},
};

use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use safe_lock::parking_lot::SafeRwLock;

use crate::{
    Merge, MergeInv,
    inner::{NodeIndex, common::Common},
};

use super::{Multiplicity, NodeInner};

#[derive(Debug)]
pub struct StrongHandle<T: Merge> {
    inner: Arc<RwLock<Option<NodeInner<T>>>>,
    common: Common<T>,
    index: u64,
}

#[derive(Debug)]
pub struct WeakHandle<T: Merge> {
    inner: Weak<RwLock<Option<NodeInner<T>>>>,
    common: Common<T>,
    index: u64,
}

/// Handle of a parent to a child.
#[derive(Debug, Clone)]
pub struct ChildHandle<T: Merge> {
    inner: WeakHandle<T>,
}

/// Handle of a child to its parent.
#[derive(Debug, Clone)]
pub struct ParentHandle<T: Merge> {
    pub inner: StrongHandle<T>,
}

/// User handle to a node.
#[derive(Debug)]
pub struct SelfHandle<T: Merge> {
    pub inner: StrongHandle<T>,
}

impl<T: Merge> Clone for StrongHandle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            common: self.common.clone(),
            index: self.index,
        }
    }
}

impl<T: Merge> Clone for WeakHandle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Weak::clone(&self.inner),
            common: self.common.clone(),
            index: self.index,
        }
    }
}

impl<T: Merge> SelfHandle<T> {
    #[inline]
    pub fn root(data: T) -> Self {
        Self {
            inner: StrongHandle::root(data),
        }
    }
}

impl<T: Merge> WeakHandle<T> {
    #[inline]
    fn upgrade(&self) -> Option<StrongHandle<T>> {
        self.inner.upgrade().map(|inner| StrongHandle {
            inner,
            index: self.index,
            common: self.common.clone(),
        })
    }
}

impl<T: Merge> StrongHandle<T> {
    #[inline]
    fn downgrade(&self) -> WeakHandle<T> {
        WeakHandle {
            inner: Arc::downgrade(&self.inner),
            common: self.common.clone(),

            index: self.index,
        }
    }

    fn safe_lock<'a>(&'a self) -> SafeRwLock<'a, Option<NodeInner<T>>> {
        SafeRwLock::new(&self.inner)
    }

    #[inline]
    pub fn write_guard<'a>(&'a self) -> MappedRwLockWriteGuard<'a, NodeInner<T>> {
        self.try_write_guard().unwrap()
    }

    #[inline]
    pub fn try_write_guard<'a>(&'a self) -> Option<MappedRwLockWriteGuard<'a, NodeInner<T>>> {
        RwLockWriteGuard::try_map(self.inner.write(), |node| node.as_mut()).ok()
    }

    #[inline]
    pub fn read_guard<'a>(&'a self) -> MappedRwLockReadGuard<'a, NodeInner<T>> {
        RwLockReadGuard::map(self.inner.read(), |node| node.as_ref().unwrap())
    }

    #[inline]
    pub fn try_read_guard<'a>(&'a self) -> Option<MappedRwLockReadGuard<'a, NodeInner<T>>> {
        RwLockReadGuard::try_map(self.inner.read(), |node| node.as_ref()).ok()
    }

    fn new(node: NodeInner<T>, common: Common<T>) -> Self {
        Self {
            index: node.index,
            common,
            inner: Arc::new(RwLock::new(Some(node))),
        }
    }

    pub fn root(data: T) -> Self {
        let node = NodeInner::root(data);
        Self::new(node, Common::new())
    }

    pub fn create_child(&self, data: T) -> Self {
        self.write_node(|node| self.add_child(node, data))
    }

    #[inline]
    pub fn create_children(&self, data: impl IntoIterator<Item = T>) -> impl Iterator<Item = Self> {
        let mut node_guard = self.write_guard();
        data.into_iter()
            .map(move |data| self.add_child(&mut *node_guard, data))
    }

    #[inline]
    fn write_node<U>(&self, f: impl FnOnce(&mut NodeInner<T>) -> U) -> U {
        f(&mut *self.write_guard())
    }

    fn add_child(&self, node: &mut NodeInner<T>, data: T) -> StrongHandle<T> {
        let parent_handle = ParentHandle {
            inner: self.clone(),
        };
        let child_node = NodeInner::child(parent_handle, node.counter.clone(), data);
        self.add_child_inner(node, child_noden)
    }

    fn add_child_inner(
        &self,
        node: &mut NodeInner<T>,
        child_node: NodeInner<T>,
        common: Common<T>,
    ) -> StrongHandle<T> {
        let child_index = child_node.index;
        let child_strong_handle = StrongHandle::new(child_node, common);
        let child_handle = ChildHandle {
            inner: child_strong_handle.downgrade(),
        };
        node.insert_child(child_index, child_handle);
        child_strong_handle
    }

    fn try_drop(&mut self, args: TryDropArg) -> Result<(), TryDropError<T>> {
        let mut child_strong_handle_opt: Option<StrongHandle<T>> = None;

        let mut node_opt_lock = self.safe_lock();

        loop {
            // Lock the child if there is some.
            let child_guard_opt = if let Some(child_strong_handle) = &child_strong_handle_opt {
                let child_index = child_strong_handle.index;
                let Ok(child_opt_guard) = child_strong_handle.safe_lock().try_lock_immediate()
                else {
                    // Another thread reads the child.
                    return Err(TryDropError {
                        handle: self.clone(),
                        blocking_node: child_index,
                    });
                };
                match child_opt_guard.try_map(Option::as_mut) {
                    Ok(child_guard) => Some(child_guard),
                    guard @ Err(_) => {
                        drop(guard);
                        // The child may have been dropped in the meantime. We need to start everything over.
                        child_strong_handle_opt = None;
                        continue;
                    }
                }
            } else {
                None
            };

            // Lock the node.
            let Ok(node_opt_guard) = node_opt_lock.try_lock_immediate() else {
                // Another thread reads the node.
                return Err(TryDropError {
                    handle: self.clone(),
                    blocking_node: self.index,
                });
            };
            let Some(node) = node_opt_guard.as_ref() else {
                // The user dropped the node in the meantime.
                assert!(!args.self_drop);
                return Ok(());
            };
            let node_index = node.index;

            match Multiplicity::from_iter(&node.children) {
                Multiplicity::None => {
                    let mut node_opt_guard = node_opt_guard.upgrade();
                    let node = node_opt_guard.as_mut().unwrap();
                    // No children, the node is a leaf that can be removed from the tree.
                    node.alive = false;

                    let Some(mut parent_handle) = node.parent.take() else {
                        // If this is the root, there is nothing to do.
                        break;
                    };

                    let mut parent_guard = parent_handle
                        .inner
                        .try_write_guard()
                        .expect("parent must not be dropped as it has a child");

                    let child_handle = parent_guard
                        .children
                        .remove(&node_index)
                        .expect("node is child of parent");

                    drop(child_handle);
                    drop(parent_guard);

                    // The node is removed from the tree.
                    // We need to recursively drop its parent if necessary.
                    parent_handle.inner.try_drop(false);

                    break;
                }
                Multiplicity::Multiple => {
                    let mut node_opt_guard = node_opt_guard.upgrade();
                    let node = node_opt_guard.as_mut().unwrap();
                    // Two children or more, the node must not be dropped.
                    if self_drop {
                        node.alive = false;
                    }
                    break;
                }
                Multiplicity::Single((child_index, child_handle)) => {
                    let child_index = *child_index;

                    if child_guard_opt.is_none() {
                        drop(child_guard_opt);
                        let child_strong_handle = child_handle
                            .inner
                            .upgrade()
                            .expect("child should not be dropped while the weak exists");
                        node_opt_lock = node_opt_guard.unlock();
                        child_strong_handle_opt.replace(child_strong_handle);
                        continue;
                    };

                    let child_guard = child_guard_opt.unwrap();

                    // let Some(child_guard) = child_guard_opt else {
                    //     drop(child_guard_opt);
                    //     let child_strong_handle = child_handle
                    //         .inner
                    //         .upgrade()
                    //         .expect("child should not be dropped while the weak exists");
                    //     node_opt_lock = node_opt_guard.unlock();
                    //     child_strong_handle_opt.replace(child_strong_handle);
                    //     continue;
                    // };

                    let child_strong_handle =
                        child_strong_handle_opt.as_ref().expect("checked above");

                    if !ptr::eq(
                        Arc::as_ptr(&child_strong_handle.inner),
                        Weak::as_ptr(&child_handle.inner.inner),
                    ) {
                        // Child has changed in the meantime.
                        let child_strong_handle = child_handle
                            .inner
                            .upgrade()
                            .expect("child should not be dropped while the weak exists");
                        drop(child_guard);
                        node_opt_lock = node_opt_guard.unlock();
                        child_strong_handle_opt.replace(child_strong_handle);
                        continue;
                    }

                    if let Some(parent_handle) = &node.parent {
                        // The node is node the root.

                        let parent_arc = parent_handle.inner.inner.clone();

                        let mut parent_guard =
                            RwLockWriteGuard::map(parent_arc.write(), |parent_opt| {
                                parent_opt
                                    .as_mut()
                                    .expect("parent must not be dropped as a child exists")
                            });

                        // We can take ownership of node.

                        let mut node_opt_guard = node_opt_guard.upgrade();
                        let node_owned = node_opt_guard.take().unwrap();
                        let parent_handle = node_owned.parent.expect("node has a parent");

                        let (child_index_confirm, child_handle) =
                            Multiplicity::from_iter(node_owned.children)
                                .into_single()
                                .expect("node has a single child");
                        assert_eq!(child_index, child_index_confirm);

                        // Link parent to child

                        let node_child_handle = parent_guard
                            .children
                            .remove(&node_index)
                            .expect("node is child of parent");
                        drop(node_child_handle);

                        parent_guard.insert_child(child_index, child_handle);

                        // Link child to parent

                        let mut child_guard = child_guard.upgrade();

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
                        let mut node_opt_guard = node_opt_guard.upgrade();
                        let node_owned = node_opt_guard.take().unwrap();

                        let (child_index_confirm, child_handle) =
                            Multiplicity::from_iter(node_owned.children)
                                .into_single()
                                .expect("node has a single child");
                        assert_eq!(child_index, child_index_confirm);
                        drop(child_handle);

                        // Remove parent from the child.
                        let mut child_guard = child_guard.upgrade();
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

        Ok(())
    }
}

impl<T: Merge> Drop for StrongHandle<T> {
    fn drop(&mut self) {
        let process_index = self.common.next_process_index();

        let args = TryDropArg {
            self_drop: true,
            process_index,
        };

        match self.try_drop(args) {
            Ok(()) => {
                // Empty the drop queue
                self.common.drop_queue().execute_drops(process_index);
            }
            Err(err) => {
                self.common
                    .drop_queue()
                    .insert_to_queue(err.blocking_node, self.clone());
            }
        }
    }
}

#[derive(Debug)]
struct TryDropArg {
    self_drop: bool,
    process_index: u64,
}

#[derive(Debug, Clone)]
pub struct TryDropError<T: Merge> {
    handle: StrongHandle<T>,
    blocking_node: NodeIndex,
}
