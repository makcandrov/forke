//! Traversal and search from a node up to the root.

use forke::{Node, iter::TraverseGuards};

#[test]
fn traverse_iter_root_to_leaf() {
    let root = Node::root(vec![1u32]);
    let mid = root.fork(vec![2]);
    let leaf = mid.fork(vec![3]);

    let collected: Vec<Vec<u32>> = leaf.traverse().map(|g| g.data().clone()).collect();
    assert_eq!(collected, vec![vec![3], vec![2], vec![1]]);
}

#[test]
fn traverse_ref_iter() {
    let root = Node::root(vec![10u32]);
    let child = root.fork(vec![20]);
    let leaf = child.fork(vec![30]);

    let mut guards = TraverseGuards::new();
    let collected: Vec<&Vec<u32>> = leaf.traverse_ref(&mut guards).collect();
    assert_eq!(collected, vec![&vec![30], &vec![20], &vec![10]]);
}

#[test]
fn traverse_guards_default() {
    let _g: TraverseGuards<Vec<u8>> = TraverseGuards::default();
}

#[test]
fn traverse_survives_starting_node_merge() {
    // `TraverseIter::new` locks the starting node eagerly, so dropping the
    // starting node's handle before iterating defers its single-child
    // collapse instead of invalidating the iterator.
    let root = Node::root(vec![1u32]);
    let mid = root.fork(vec![2]);
    let leaf = mid.fork(vec![3]);

    let iter = mid.traverse();
    drop(mid);

    let collected: Vec<Vec<u32>> = iter.map(|g| g.data().clone()).collect();
    assert_eq!(collected[0], vec![2]);

    let _ = leaf;
    let _ = root;
}

#[test]
fn traverse_survives_drop_between_calls() {
    // Between two `next` calls, the next ancestor's handle is dropped. The
    // read lock acquired on that ancestor during the previous `next` defers
    // its single-child collapse, so iteration still yields it.
    let root = Node::root(vec![1u32]);
    let mid = root.fork(vec![2]);
    let leaf = mid.fork(vec![3]);

    let mut iter = leaf.traverse();

    let g1 = iter.next().unwrap();
    assert_eq!(g1.data(), &vec![3]);
    drop(g1);

    drop(mid);

    let g2 = iter.next().unwrap();
    assert_eq!(g2.data(), &vec![2]);
    drop(g2);

    let g3 = iter.next().unwrap();
    assert_eq!(g3.data(), &vec![1]);
    drop(g3);

    assert!(iter.next().is_none());

    let _ = leaf;
    let _ = root;
}

#[test]
fn traverse_ref_survives_drop_between_calls() {
    // Same scenario as `traverse_survives_drop_between_calls`, through the
    // reference-yielding iterator: the guards accumulated in
    // `TraverseGuards` keep the next ancestor pinned.
    let root = Node::root(vec![1u32]);
    let mid = root.fork(vec![2]);
    let leaf = mid.fork(vec![3]);

    let mut storage = TraverseGuards::new();
    let mut iter = leaf.traverse_ref(&mut storage);

    let d1 = iter.next().unwrap();
    assert_eq!(d1, &vec![3]);

    drop(mid);

    let d2 = iter.next().unwrap();
    assert_eq!(d2, &vec![2]);

    let d3 = iter.next().unwrap();
    assert_eq!(d3, &vec![1]);

    assert!(iter.next().is_none());

    let _ = leaf;
    let _ = root;
}

#[test]
fn search_finds_in_ancestor() {
    let root = Node::root(vec![100u32]);
    let child = root.fork(vec![]);
    let leaf = child.fork(vec![]);

    let found = leaf.search(|v| if v.contains(&100) { Some(true) } else { None });
    assert_eq!(found, Some(true));
}

#[test]
fn search_returns_none() {
    let root = Node::root(vec![1u32]);
    let found = root.search(|v| if v.contains(&999) { Some(true) } else { None });
    assert!(found.is_none());
}

#[test]
fn guard_recursive_search() {
    let root = Node::root(vec![42u32]);
    let child = root.fork(vec![]);
    let leaf = child.fork(vec![]);

    let g = leaf.guard();
    let found = g.search(|v| if v.contains(&42) { Some(42) } else { None });
    assert_eq!(found, Some(42));
}

#[test]
fn guard_recursive_search_none() {
    let root = Node::root(vec![1u32]);
    let g = root.guard();
    let found = g.search(|v| if v.contains(&999) { Some(true) } else { None });
    assert!(found.is_none());
}

#[test]
fn owned_guard_search_found_on_self() {
    let root = Node::root(vec![7u32]);
    let g = root.owned_guard();
    let found = g.search(|v| if v.contains(&7) { Some(7) } else { None });
    assert_eq!(found, Some(7));
}

#[test]
fn owned_guard_search_found_in_ancestor() {
    let root = Node::root(vec![7u32]);
    let child = root.fork(vec![]);
    let g = child.owned_guard();
    let found = g.search(|v| if v.contains(&7) { Some(7) } else { None });
    assert_eq!(found, Some(7));
}

#[test]
fn owned_guard_search_none() {
    let root = Node::root(vec![1u32]);
    let child = root.fork(vec![]);
    let g = child.owned_guard();
    let found = g.search(|v| if v.contains(&999) { Some(true) } else { None });
    assert!(found.is_none());
}
