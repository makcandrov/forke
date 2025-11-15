use std::{
    ptr,
    sync::{Arc, Weak},
};

use hashbrown::HashMap;
use parking_lot::{RwLock, RwLockWriteGuard};

use crate::{Merge, utils::Multiplicity};

use super::NodeInner;

#[derive(Debug)]
pub(super) struct ChildHandle<T: Merge> {
    pub(super) inner: Weak<RwLock<Option<NodeInner<T>>>>,
}

/// Handle of a child to its parent.
#[derive(Debug)]
pub(super) struct ParentHandle<T: Merge> {
    pub(super) inner: Arc<RwLock<Option<NodeInner<T>>>>,
    pub(super) child_index: usize,
    pub(super) skip_drop: bool,
}

#[derive(Debug)]
pub(crate) struct SelfHandle<T: Merge> {
    pub(super) inner: Arc<RwLock<Option<NodeInner<T>>>>,
}

impl<T: Merge> SelfHandle<T> {
    fn new(node: NodeInner<T>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Some(node))),
        }
    }

    pub fn root(data: T) -> Self {
        let node = NodeInner::root(data);
        Self::new(node)
    }

    pub fn create_child(&self, data: T) -> Self {
        self.write_node(|node| add_child(&self.inner, node, data))
    }

    #[inline]
    pub fn create_children(&self, data: impl IntoIterator<Item = T>) -> impl Iterator<Item = Self> {
        let mut node_guard =
            RwLockWriteGuard::map(self.inner.write(), |node| node.as_mut().unwrap());
        data.into_iter()
            .map(move |data| add_child(&self.inner, &mut *node_guard, data))
    }

    #[inline]
    fn write_node<U>(&self, f: impl FnOnce(&mut NodeInner<T>) -> U) -> U {
        f(self.inner.write().as_mut().unwrap())
    }
}

fn add_child<T: Merge>(
    self_inner: &Arc<RwLock<Option<NodeInner<T>>>>,
    this: &mut NodeInner<T>,
    data: T,
) -> SelfHandle<T> {
    this.add_child_with(|child_index| {
        let parent_handle = ParentHandle {
            inner: self_inner.clone(),
            child_index,
            skip_drop: false,
        };
        let child = NodeInner {
            parent: Some(parent_handle),
            children: HashMap::new(),
            alive: true,
            next_index: 0,
            data,
        };
        let child_arc = Arc::new(RwLock::new(Some(child)));
        let child_handle = ChildHandle {
            inner: Arc::downgrade(&child_arc),
        };
        let self_handle = SelfHandle { inner: child_arc };
        (child_handle, self_handle)
    })
}

impl<T: Merge> ParentHandle<T> {
    /// Drops the handle without running the custom drop, while still running drop of the other
    /// fields.
    #[inline]
    fn drop_forget(mut self) {
        self.skip_drop = true;
    }
}

impl<T: Merge> Drop for SelfHandle<T> {
    fn drop(&mut self) {
        let mut child_arc_opt: Option<Arc<RwLock<Option<NodeInner<T>>>>> = None;

        loop {
            let mut child_guard_opt = if let Some(child_arc) = &child_arc_opt {
                let Ok(child_guard) = RwLockWriteGuard::try_map(child_arc.write(), Option::as_mut)
                else {
                    // The child may have been dropped in the meantime. We need to start everything
                    // over.
                    child_arc_opt = None;
                    continue;
                };
                Some(child_guard)
            } else {
                None
            };

            let mut node_opt_guard = self.inner.write();
            let node = &mut node_opt_guard
                .as_mut()
                .expect("node must not be dropped as a user handle exist");

            match Multiplicity::from_iter(&node.children) {
                Multiplicity::None => {
                    // No children, the node is a leaf that can be removed from the tree.
                    node.alive = false;

                    let Some(parent_handle) = &node.parent else {
                        // If this is the root, there is nothing to do.
                        break;
                    };

                    let parent_arc = &parent_handle.inner;
                    let mut parent_opt_guard = parent_arc.write();
                    let parent = parent_opt_guard
                        .as_mut()
                        .expect("parent not removed from the tree");

                    let child_handle = parent
                        .children
                        .remove(&parent_handle.child_index)
                        .expect("node is child of parent");
                    drop(child_handle);

                    break;
                }
                Multiplicity::Multiple => {
                    // Two children or more, the node must not be dropped.
                    node.alive = false;
                    break;
                }
                Multiplicity::Single((child_index, child_handle)) => {
                    let child_index = *child_index;

                    let Some(ref mut child_guard) = child_guard_opt else {
                        drop(child_guard_opt);
                        let child_arc = Weak::upgrade(&child_handle.inner)
                            .expect("child should not be dropped while the weak exists");
                        child_arc_opt.replace(child_arc);
                        continue;
                    };

                    let child_arc = child_arc_opt.as_ref().expect("checked above");

                    if !ptr::eq(Arc::as_ptr(child_arc), Weak::as_ptr(&child_handle.inner)) {
                        // Child has changed in the meantime.
                        let child_arc = Weak::upgrade(&child_handle.inner)
                            .expect("child should not be dropped while the weake exists");
                        drop(child_guard_opt);
                        drop(node_opt_guard);
                        child_arc_opt.replace(child_arc);
                        continue;
                    }

                    if let Some(parent_handle) = &node.parent {
                        let parent_arc = parent_handle.inner.clone();

                        let mut parent_guard =
                            RwLockWriteGuard::map(parent_arc.write(), |parent_opt| {
                                parent_opt
                                    .as_mut()
                                    .expect("parent must not be dropped as a child exists")
                            });

                        // We can take ownership of node.

                        let node_owned = node_opt_guard.take().unwrap();
                        let mut parent_handle = node_owned.parent.expect("node has a prent");

                        let (child_index_confirm, child_handle) =
                            Multiplicity::from_iter(node_owned.children)
                                .into_single()
                                .expect("node has a single child");
                        assert_eq!(child_index, child_index_confirm);

                        // Link parent to child

                        let node_child_handle = parent_guard
                            .children
                            .remove(&parent_handle.child_index)
                            .expect("node is child of parent");
                        drop(node_child_handle);

                        let new_child_index = parent_guard.add_child(child_handle);
                        parent_handle.child_index = new_child_index;

                        // Link child to parent

                        let node_parent_handle = child_guard
                            .parent
                            .replace(parent_handle)
                            .expect("child has a parent");

                        node_parent_handle.drop_forget();

                        // Merge data
                        Merge::merge(node_owned.data, &mut child_guard.data);

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

                        node_parent_handle.drop_forget();

                        // Merge data
                        Merge::merge(node_owned.data, &mut child_guard.data);
                        break;
                    }
                }
            }
        }
    }
}

impl<T: Merge> Drop for ParentHandle<T> {
    fn drop(&mut self) {
        if self.skip_drop {
            return;
        }

        let mut child_arc_opt: Option<Arc<RwLock<Option<NodeInner<T>>>>> = None;

        loop {
            let mut child_guard_opt = if let Some(child_arc) = &child_arc_opt {
                let Ok(child_guard) = RwLockWriteGuard::try_map(child_arc.write(), Option::as_mut)
                else {
                    // The child may have been dropped in the meantime. We need to start everything
                    // over.
                    child_arc_opt = None;
                    continue;
                };
                Some(child_guard)
            } else {
                None
            };

            let mut node_opt_guard = self.inner.write();
            let node = &mut node_opt_guard
                .as_mut()
                .expect("node must not be dropped as a user handle exist");

            if node.alive {
                break;
            }

            match Multiplicity::from_iter(&node.children) {
                Multiplicity::None => {
                    // No children, the node is a leaf that can be removed from the tree.
                    node.alive = false;

                    let Some(parent_handle) = &node.parent else {
                        // If this is the root, there is nothing to do.
                        break;
                    };

                    let parent_arc = &parent_handle.inner;
                    let mut parent_opt_guard = parent_arc.write();
                    let parent = parent_opt_guard
                        .as_mut()
                        .expect("parent not removed from the tree");

                    let child_handle = parent
                        .children
                        .remove(&parent_handle.child_index)
                        .expect("node is child of parent");
                    drop(child_handle);

                    break;
                }
                Multiplicity::Multiple => {
                    // Two children or more, the node must not be dropped.
                    node.alive = false;
                    break;
                }
                Multiplicity::Single((child_index, child_handle)) => {
                    let child_index = *child_index;

                    let Some(ref mut child_guard) = child_guard_opt else {
                        drop(child_guard_opt);
                        let child_arc = Weak::upgrade(&child_handle.inner)
                            .expect("child should not be dropped while the weak exists");
                        child_arc_opt.replace(child_arc);
                        continue;
                    };

                    let child_arc = child_arc_opt.as_ref().expect("checked above");

                    if !ptr::eq(Arc::as_ptr(child_arc), Weak::as_ptr(&child_handle.inner)) {
                        // Child has changed in the meantime.
                        let child_arc = Weak::upgrade(&child_handle.inner)
                            .expect("child should not be dropped while the weake exists");
                        drop(child_guard_opt);
                        drop(node_opt_guard);
                        child_arc_opt.replace(child_arc);
                        continue;
                    }

                    if let Some(parent_handle) = &node.parent {
                        let parent_arc = parent_handle.inner.clone();

                        let mut parent_guard =
                            RwLockWriteGuard::map(parent_arc.write(), |parent_opt| {
                                parent_opt
                                    .as_mut()
                                    .expect("parent must not be dropped as a child exists")
                            });

                        // We can take ownership of node.

                        let node_owned = node_opt_guard.take().unwrap();
                        let mut parent_handle = node_owned.parent.expect("node has a prent");

                        let (child_index_confirm, child_handle) =
                            Multiplicity::from_iter(node_owned.children)
                                .into_single()
                                .expect("node has a single child");
                        assert_eq!(child_index, child_index_confirm);

                        // Link parent to child

                        let node_child_handle = parent_guard
                            .children
                            .remove(&parent_handle.child_index)
                            .expect("node is child of parent");
                        drop(node_child_handle);

                        let new_child_index = parent_guard.add_child(child_handle);
                        parent_handle.child_index = new_child_index;

                        // Link child to parent

                        let node_parent_handle = child_guard
                            .parent
                            .replace(parent_handle)
                            .expect("child has a parent");

                        node_parent_handle.drop_forget();

                        // Merge data
                        Merge::merge(node_owned.data, &mut child_guard.data);

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

                        node_parent_handle.drop_forget();

                        // Merge data
                        Merge::merge(node_owned.data, &mut child_guard.data);
                        break;
                    }
                }
            }
        }
    }
}
