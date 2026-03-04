#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![doc = include_str!("../README.md")]

mod data;
pub use data::NodeData;

mod inner;

mod guard;
pub use guard::NodeGuard;

mod iter;
pub use iter::{AncestorGuard, AncestorIter};

mod merge;
pub use merge::{Merge, MergeInv};

mod node;
pub use node::Node;
