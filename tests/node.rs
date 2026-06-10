//! Node creation, guard access, and forking.

use forke::Node;
use forke_test_utils::Tracked;

#[test]
fn root_create_and_drop() {
    let (data, obs) = Tracked::pair();
    let root = Node::root(data);
    assert!(!obs.is_dropped());
    drop(root);
    assert!(obs.is_dropped());
}

#[test]
fn root_has_no_parent() {
    let root = Node::root(vec![0u8]);
    assert!(root.guard().parent().is_none());
}

#[test]
fn root_guard_read() {
    let (data, obs) = Tracked::pair();
    let root = Node::root(data);
    assert_eq!(root.guard().data().id(), obs.id());
}

#[test]
fn root_guard_write() {
    let root = Node::root(vec![1u32]);
    root.guard_mut().data_mut().push(2);
    assert_eq!(*root.guard().data(), vec![1, 2]);
}

#[test]
fn root_owned_guard() {
    let (data, obs) = Tracked::pair();
    let root = Node::root(data);
    let g = root.owned_guard();
    assert_eq!(g.data().id(), obs.id());
    assert!(g.parent().is_none());
}

#[test]
fn root_owned_write_guard() {
    let root = Node::root(vec![10u32]);
    {
        let mut g = root.owned_guard_mut();
        g.data_mut().push(20);
        assert_eq!(*g.data(), vec![10, 20]);
    }
    assert_eq!(*root.guard().data(), vec![10, 20]);
}

#[test]
fn fork_single_child() {
    let root = Node::root(vec![1u32]);
    let child = root.fork(vec![2]);
    assert_eq!(*child.guard().data(), vec![2]);
    assert_eq!(*child.guard().parent().unwrap().data(), vec![1]);
}

#[test]
fn fork_many() {
    let root = Node::root(String::new());
    let children: Vec<_> = root
        .fork_many(vec!["a".into(), "b".into(), "c".into()])
        .collect();
    assert_eq!(children.len(), 3);
    assert_eq!(*children[0].guard().data(), "a");
    assert_eq!(*children[1].guard().data(), "b");
    assert_eq!(*children[2].guard().data(), "c");

    assert!(children[0].guard().parent().is_some());
    assert!(children[1].guard().parent().is_some());
    assert!(children[2].guard().parent().is_some());
}

#[test]
fn fork_n() {
    let root = Node::root(String::new());
    let [child0, child1, child2] = root.fork_n(["a".into(), "b".into(), "c".into()]);
    assert_eq!(child0.guard().data(), "a");
    assert_eq!(child1.guard().data(), "b");
    assert_eq!(child2.guard().data(), "c");

    assert!(child0.guard().parent().is_some());
    assert!(child1.guard().parent().is_some());
    assert!(child2.guard().parent().is_some());
}

#[test]
fn fork_many_empty_and_unconsumed() {
    let root = Node::root(vec![0u32]);

    assert_eq!(root.fork_many(std::iter::empty::<Vec<u32>>()).count(), 0);

    // Dropping the iterator unconsumed creates nothing and releases the
    // parent's write lock.
    let it = root.fork_many(vec![vec![1]]);
    drop(it);

    let child = root.fork(vec![2]);
    assert_eq!(*child.guard().data(), vec![2]);
}

#[test]
fn many_children_grow_and_shrink_transitions() {
    // The children map switches representation as it grows and shrinks
    // (single entry, sorted vec, hash map). Crossing those boundaries in
    // both directions must not panic or lose children.
    let root = Node::root(vec![0u32]);

    let mut children: Vec<_> = (1..=40u32).map(|i| root.fork(vec![i])).collect();
    let survivor = children.pop().unwrap();

    for c in children {
        drop(c);
    }

    assert_eq!(*survivor.guard().data(), vec![40]);
    assert_eq!(*survivor.guard().parent().unwrap().data(), vec![0]);

    // Root collapse merges downward into the survivor.
    drop(root);
    assert_eq!(*survivor.guard().data(), vec![0, 40]);
    assert!(survivor.guard().parent().is_none());
}
