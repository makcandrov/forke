use forke_test_utils::Tracked;
use std::thread;

use forke::Node;

#[test]
fn large_linear_chain_no_leak() {
    // Build a long chain, then drop from root down.
    // The cascade should merge everything cleanly.
    let root = Node::root(vec![0u32]);
    let mut nodes = vec![root];
    for i in 1..1000 {
        let child = nodes.last().unwrap().fork(vec![i]);
        nodes.push(child);
    }

    let leaf_ref = &nodes[999];
    assert_eq!(leaf_ref.guard().data(), &vec![999]);

    // Drop all but the leaf, oldest first.
    let leaf = nodes.pop().unwrap();
    for node in nodes {
        drop(node);
    }

    // Leaf should now be the root with all data merged into it.
    assert!(leaf.guard().parent().is_none());
    drop(leaf);
}

#[test]
fn large_star_no_leak() {
    let root = Node::root(Tracked::new());
    let children: Vec<_> = (0..1000).map(|_| root.fork(Tracked::new())).collect();
    drop(root);

    // Drop all children.
    for child in children {
        drop(child);
    }
}

#[test]
fn grow_and_shrink_repeatedly() {
    // Grow a tree, then tear it down, multiple times.
    // Verifies no memory accumulates across cycles.
    for _ in 0..50 {
        let root = Node::root(Tracked::new());
        let mut nodes = vec![];
        for _ in 0..200 {
            let parent = if nodes.is_empty() {
                &root
            } else {
                &nodes[nodes.len() / 2]
            };
            nodes.push(parent.fork(Tracked::new()));
        }
        drop(root);
        // Drop in reverse.
        while let Some(n) = nodes.pop() {
            drop(n);
        }
    }
}

#[test]
fn grow_and_shrink_concurrent() {
    // Multiple threads repeatedly fork and drop nodes on the same tree.
    let root = Node::root(Tracked::new());

    thread::scope(|s| {
        for _ in 0..5 {
            let r = &root;
            s.spawn(move || {
                for _ in 0..500 {
                    let child = r.fork(Tracked::new());
                    let grandchild = child.fork(Tracked::new());
                    drop(child);
                    drop(grandchild);
                }
            });
        }
    });

    drop(root);
}

#[test]
fn fork_many_then_drop_all() {
    let root = Node::root(Tracked::new());
    for _ in 0..20 {
        let children = root.fork_many((0..100).map(|_| Tracked::new()));
        // All children dropped immediately at end of iteration.
        drop(children);
    }
    drop(root);
}
