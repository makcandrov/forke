use forke::{Node, TraverseGuards};
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
fn root_has_no_parent() {
    let root = Node::root(vec![0u8]);
    assert!(root.guard().parent().is_none());
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
    let children = root.fork_many(vec!["a".into(), "b".into(), "c".into()]);
    assert_eq!(children.len(), 3);
    assert_eq!(*children[0].guard().data(), "a");
    assert_eq!(*children[1].guard().data(), "b");
    assert_eq!(*children[2].guard().data(), "c");
}

#[test]
fn deep_chain() {
    // Each reassignment drops the old node (single child) => cascade merge.
    let root = Node::root(vec![0u32]);
    let mut current = root.fork(vec![1]);
    for i in 2..10 {
        current = current.fork(vec![i]);
    }
    // Each intermediate node was dead with 1 child => merged via MergeInv.
    // Root is still alive so its data doesn't merge.
    let data = current.guard().data().clone();
    assert_eq!(data, (1..10).collect::<Vec<u32>>());
}

#[test]
fn drop_leaf_child_no_merge() {
    let (rd, ro) = Tracked::pair();
    let (cd, co) = Tracked::pair();
    let root = Node::root(rd);
    let child = root.fork(cd);

    drop(child);
    assert!(co.is_dropped());
    assert!(!ro.is_dropped());
    assert!(co.merges().is_empty());
    assert!(ro.merges().is_empty());

    drop(root);
    assert!(ro.is_dropped());
}

#[test]
fn drop_parent_single_child_merges_into_child() {
    // root -> child.  Drop root => MergeInv(child_slot, root_data).
    // MergeInv swaps: root's Tracked goes into child slot, child's Tracked is consumed.
    let (rd, ro) = Tracked::pair();
    let (cd, co) = Tracked::pair();
    let root = Node::root(rd);
    let child = root.fork(cd);

    drop(root);
    // child's original Tracked was consumed by MergeInv => dropped.
    assert!(co.is_dropped());
    // root's Tracked survives in the child node slot.
    assert!(!ro.is_dropped());
    // Merge::merge was called on root's Tracked with child's Tracked => root absorbed child.
    assert_eq!(ro.merges(), vec![co.id()]);
    // child is now the new root.
    assert!(child.guard().parent().is_none());

    drop(child);
    assert!(ro.is_dropped());
}

#[test]
fn drop_middle_node_relinks_and_merges() {
    // root -> mid -> leaf.  Drop mid => MergeInv(leaf_slot, mid_data).
    // mid's Tracked goes into leaf slot, leaf's Tracked is consumed.
    let (rd, ro) = Tracked::pair();
    let (md, mo) = Tracked::pair();
    let (ld, lo) = Tracked::pair();

    let root = Node::root(rd);
    let mid = root.fork(md);
    let leaf = mid.fork(ld);

    drop(mid);
    // leaf's original Tracked consumed => dropped.
    assert!(lo.is_dropped());
    // mid's Tracked survives in leaf slot.
    assert!(!mo.is_dropped());
    assert_eq!(mo.merges(), vec![lo.id()]);
    // leaf is now root's direct child; its data is mid's Tracked.
    assert_eq!(leaf.guard().parent().unwrap().data().id(), ro.id());

    drop(leaf);
    assert!(mo.is_dropped());
    drop(root);
    assert!(ro.is_dropped());
}

// --- Drop / merge: multi-child prevents collapse ---

#[test]
fn drop_parent_with_multiple_children_no_merge() {
    let (rd, ro) = Tracked::pair();
    let (c1d, c1o) = Tracked::pair();
    let (c2d, c2o) = Tracked::pair();

    let root = Node::root(rd);
    let c1 = root.fork(c1d);
    let c2 = root.fork(c2d);

    drop(root);
    // root has 2 children — marked dead but not removed.
    assert!(!ro.is_dropped());
    assert!(c1o.merges().is_empty());
    assert!(c2o.merges().is_empty());

    // Now drop c1 => root has 1 child (c2) => MergeInv(c2_slot, root_data).
    // c2's Tracked consumed, root's Tracked survives in c2 slot.
    drop(c1);
    assert!(c1o.is_dropped());
    assert!(c2o.is_dropped());
    assert!(!ro.is_dropped());
    assert_eq!(ro.merges(), vec![c2o.id()]);

    drop(c2);
    assert!(ro.is_dropped());
}

#[test]
fn cascade_drop_multiple_levels() {
    // root -> a -> b -> c.  Drop root, then a, then b.
    // Each MergeInv: parent Tracked survives in child slot, child Tracked consumed.
    let (rd, ro) = Tracked::pair();
    let (ad, ao) = Tracked::pair();
    let (bd, bo) = Tracked::pair();
    let (cd, co) = Tracked::pair();

    let root = Node::root(rd);
    let a = root.fork(ad);
    let b = a.fork(bd);
    let c = b.fork(cd);

    // Drop root: root has 1 child (a) => MergeInv(a_slot, root_data).
    // a's Tracked consumed, root's Tracked now in a's slot.
    drop(root);
    assert!(ao.is_dropped());
    assert!(!ro.is_dropped());
    assert_eq!(ro.merges(), vec![ao.id()]);

    // Drop a: a's slot has root's Tracked. a has 1 child (b) =>
    // MergeInv(b_slot, root_tracked). b's Tracked consumed, root's Tracked now in b's slot.
    drop(a);
    assert!(bo.is_dropped());
    assert!(!ro.is_dropped());
    assert_eq!(ro.merges(), vec![ao.id(), bo.id()]);

    // Drop b: b's slot has root's Tracked. b has 1 child (c) =>
    // MergeInv(c_slot, root_tracked). c's Tracked consumed, root's Tracked now in c's slot.
    drop(b);
    assert!(co.is_dropped());
    assert!(!ro.is_dropped());
    assert_eq!(ro.merges(), vec![ao.id(), bo.id(), co.id()]);

    // c is now root, holding root's original Tracked.
    assert!(c.guard().parent().is_none());
    drop(c);
    assert!(ro.is_dropped());
}

#[test]
fn readme_example_cascade() {
    // The README scenario: A->B, A->C, B->D, B->E, E->F, E->G, E->H
    // Drop B, E, then D.
    let (ad, _ao) = Tracked::pair();
    let (bd, bo) = Tracked::pair();
    let (cd, _co) = Tracked::pair();
    let (dd, do_) = Tracked::pair();
    let (ed, eo) = Tracked::pair();
    let (fd, _fo) = Tracked::pair();
    let (gd, _go) = Tracked::pair();
    let (hd, _ho) = Tracked::pair();

    let a = Node::root(ad);
    let b = a.fork(bd);
    let c = a.fork(cd);
    let d = b.fork(dd);
    let e = b.fork(ed);
    let f = e.fork(fd);
    let g = e.fork(gd);
    let h = e.fork(hd);

    // B and E are dead but have multiple children — no collapse yet.
    drop(b);
    drop(e);
    assert!(!bo.is_dropped());
    assert!(!eo.is_dropped());
    assert!(!do_.is_dropped());

    // Drop D: D is a dead leaf => removed.
    // B now has single child E => MergeInv(e_slot, b_data).
    // E's Tracked consumed, B's Tracked survives in E's slot.
    // A still has 2 children (C, E) => stops.
    drop(d);
    assert!(do_.is_dropped());
    assert!(eo.is_dropped());
    assert!(!bo.is_dropped());
    assert_eq!(bo.merges(), vec![eo.id()]);

    drop(f);
    drop(g);
    drop(h);
    drop(c);
    drop(a);
}

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
    let found = g.recursive_search(|v| if v.contains(&42) { Some(42) } else { None });
    assert_eq!(found, Some(42));
}

#[test]
fn guard_recursive_search_none() {
    let root = Node::root(vec![1u32]);
    let g = root.guard();
    let found = g.recursive_search(|v| if v.contains(&999) { Some(true) } else { None });
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
