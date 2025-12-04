#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![doc = include_str!("../README.md")]

mod inner;

mod guard;
mod iter;

mod merge;
pub use merge::{Merge, MergeInv};

mod node;
pub use node::Node;
