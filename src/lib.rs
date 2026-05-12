#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![doc = include_str!("../README.md")]

mod inner;

mod guard;
pub use guard::{NodeGuard, NodeWriteGuard, OwnedNodeGuard, OwnedNodeWriteGuard};

pub mod iter;

mod merge;
pub use merge::{Merge, MergeInv};

mod node;
pub use node::{Node, NodeData};

const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_sync<T: Sync>() {}

    assert_send_sync::<Node<()>>();
    assert_sync::<NodeGuard<()>>();
    assert_sync::<NodeWriteGuard<()>>();
    assert_sync::<OwnedNodeGuard<()>>();
    assert_sync::<OwnedNodeWriteGuard<()>>();
    assert_sync::<iter::TraverseIter<()>>();
    assert_sync::<iter::TraverseRefIter<()>>();
    assert_sync::<iter::TraverseGuards<()>>();
};
