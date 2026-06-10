//! Lock-contention behavior: drops and collapses that hit a held lock are
//! deferred and retried when the lock is released, and per-node locks are
//! not reentrant.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use forke::Node;
use forke_test_utils::Tracked;

#[test]
fn leaf_removal_deferred_while_parent_write_locked() {
    let root = Node::root(Tracked::new());
    let (cd, co) = Tracked::pair();
    let child = root.fork(cd);

    let g = root.guard_mut();
    // The parent's write lock is held: the leaf removal must defer.
    drop(child);
    assert!(!co.is_dropped(), "removal must be deferred, not lost");

    // Releasing the guard runs the deferred removal.
    drop(g);
    assert!(co.is_dropped(), "deferred removal must run on lock release");
}

#[test]
fn drop_child_deferred_during_fork_many_iteration() {
    let root = Node::root(Tracked::new());
    let (cd, co) = Tracked::pair();
    let child = root.fork(cd);

    let mut it = root.fork_many((0..2).map(|_| Tracked::new()));
    let n1 = it.next().unwrap();

    // The iterator holds the parent's write lock: the removal defers.
    drop(child);
    assert!(!co.is_dropped());

    let n2 = it.next().unwrap();
    drop(it);
    assert!(
        co.is_dropped(),
        "deferred removal must run when the fork_many lock releases"
    );

    drop(n1);
    drop(n2);
}

#[test]
fn collapse_deferred_while_child_read_guard_held() {
    let (rd, ro) = Tracked::pair();
    let (md, mo) = Tracked::pair();
    let (ld, lo) = Tracked::pair();

    let root = Node::root(rd);
    let mid = root.fork(md);
    let leaf = mid.fork(ld);

    let g = leaf.guard();
    // mid is dead with a single child, but the collapse needs leaf's write
    // lock, which the read guard blocks.
    drop(mid);
    assert!(mo.merges().is_empty(), "collapse must be deferred");
    assert!(!lo.is_dropped());

    // During the deferral window the tree is still consistent: leaf's parent
    // is the dead-but-linked mid node.
    assert_eq!(g.parent().unwrap().data().id(), mo.id());

    drop(g);
    // Releasing the guard runs the collapse: mid's data absorbed leaf's.
    assert_eq!(mo.merges(), vec![lo.id()]);
    assert!(lo.is_dropped());
    assert!(!mo.is_dropped());
    assert_eq!(leaf.guard().parent().unwrap().data().id(), ro.id());

    drop(leaf);
    drop(root);
    assert!(ro.is_dropped());
    assert!(mo.is_dropped());
}

#[test]
fn collapse_deferred_while_child_write_guard_held() {
    let (rd, _ro) = Tracked::pair();
    let (md, mo) = Tracked::pair();
    let (ld, lo) = Tracked::pair();

    let root = Node::root(rd);
    let mid = root.fork(md);
    let leaf = mid.fork(ld);

    let og = leaf.owned_guard_mut();
    drop(mid);
    assert!(mo.merges().is_empty(), "collapse must be deferred");
    // The data is still accessible and owned by the original leaf slot.
    assert_eq!(og.data().id(), lo.id());

    drop(og);
    assert_eq!(mo.merges(), vec![lo.id()]);
    assert!(lo.is_dropped());
}

#[test]
fn owned_guard_defers_node_removal() {
    let root = Node::root(Tracked::new());
    let (cd, co) = Tracked::pair();
    let child = root.fork(cd);

    let og = child.owned_guard();
    drop(child);
    // The owned guard blocks the node's own write lock; removal defers and
    // the data stays readable.
    assert!(!co.is_dropped());
    assert_eq!(og.data().id(), co.id());

    drop(og);
    assert!(co.is_dropped());
}

/// Per-node locks are not reentrant: forking a node while the same thread
/// holds a guard on it deadlocks, like re-locking a `std::sync::Mutex`.
/// This test passes as long as that limitation exists; if it starts
/// failing, the behavior changed and the `Node` lock-reentrancy
/// documentation must be updated.
#[test]
fn fork_while_holding_guard_deadlocks_by_design() {
    let (tx, rx) = mpsc::channel();

    // Not a scoped thread: it stays blocked forever and is intentionally
    // leaked. It only touches its own tree.
    let _t = thread::spawn(move || {
        let root = Node::root(vec![1u32]);
        let _g = root.guard();
        tx.send(()).unwrap();
        let _child = root.fork(vec![2]); // blocks: write while read held
        tx.send(()).unwrap(); // unreachable while the limitation exists
    });

    rx.recv().unwrap();
    assert!(
        rx.recv_timeout(Duration::from_secs(2)).is_err(),
        "fork under a same-node guard unexpectedly completed; update the lock-reentrancy docs"
    );
}
