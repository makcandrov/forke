#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![doc = include_str!("../README.md")]

mod inner;

mod guard;
pub use guard::{NodeGuard, NodeWriteGuard, OwnedNodeGuard, OwnedNodeWriteGuard};

mod iter;
pub use iter::{TraverseGuards, TraverseIter, TraverseRefIter};

mod merge;
pub use merge::{Merge, MergeInv};

mod node;
pub use node::{Node, NodeData};
