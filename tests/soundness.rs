//! Aliasing-model coverage for the owned guards, which store a `'static`
//! guard next to the `Arc` handle it really borrows from.
//!
//! These pass under a plain `cargo test`; they exist to be run under Miri,
//! which checks the aliasing rules that lifetime laundering relies on:
//!
//! ```text
//! cargo +nightly miri test --test soundness
//! MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly miri test --test soundness
//! ```
//!
//! Every scenario below ends with the guard's own handle holding the last
//! reference to the node, so dropping the guard inside a call frees the
//! allocation the guard's stored references point into.

use forke::Node;

#[test]
fn owned_read_guard_is_last_reference() {
    let root = Node::root(vec![1u32]);
    let guard = root.owned_guard();

    // Deferred: the guard read-locks the node, so the removal is queued.
    drop(root);

    assert_eq!(guard.data(), &vec![1]);
    drop(guard);
}

#[test]
fn owned_write_guard_is_last_reference() {
    let root = Node::root(vec![1u32]);
    let mid = root.fork(vec![2]);
    let leaf = mid.fork(vec![3]);

    let guard = mid.owned_guard_mut();
    drop(mid);
    // Releasing the guard runs the deferred collapse and frees mid.
    drop(guard);

    assert_eq!(leaf.guard().data(), &vec![2, 3]);
    let _ = root;
}

#[test]
fn traverse_guard_is_last_reference() {
    let root = Node::root(vec![1u32]);
    let mut iter = root.traverse();
    drop(root);

    let guard = iter.next().unwrap();
    assert_eq!(guard.data(), &vec![1]);
    drop(iter);
    drop(guard);
}

#[test]
fn owned_guard_search_after_teardown() {
    let root = Node::root(vec![1u32]);
    let child = root.fork(vec![2]);

    let guard = child.owned_guard();
    // Both removals defer: the guard read-locks the only remaining node.
    drop(child);
    drop(root);

    assert_eq!(guard.search(|v| v.first().copied()), Some(2));
    drop(guard);
}
