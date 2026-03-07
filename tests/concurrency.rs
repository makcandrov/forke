use forke_test_utils::Tracked;
use std::sync::{Arc, Barrier};
use std::thread;

use forke::Node;

#[test]
fn concurrent_drop_parent_and_child() {
    for _ in 0..200 {
        let (rd, ro) = Tracked::pair();
        let (cd, co) = Tracked::pair();

        let root = Node::root(rd);
        let child = root.fork(cd);

        let barrier = Arc::new(Barrier::new(2));
        let b1 = barrier.clone();

        thread::scope(|s| {
            s.spawn(move || {
                b1.wait();
                drop(root);
            });
            s.spawn(move || {
                barrier.wait();
                drop(child);
            });
        });

        assert!(ro.is_dropped());
        assert!(co.is_dropped());
    }
}

#[test]
fn concurrent_drop_siblings() {
    for _ in 0..200 {
        let (rd, ro) = Tracked::pair();
        let (c1d, c1o) = Tracked::pair();
        let (c2d, c2o) = Tracked::pair();

        let root = Node::root(rd);
        let c1 = root.fork(c1d);
        let c2 = root.fork(c2d);
        drop(root); // root is dead but has 2 children

        let barrier = Arc::new(Barrier::new(2));
        let b1 = barrier.clone();

        thread::scope(|s| {
            s.spawn(move || {
                b1.wait();
                drop(c1);
            });
            s.spawn(move || {
                barrier.wait();
                drop(c2);
            });
        });

        assert!(c1o.is_dropped());
        assert!(c2o.is_dropped());
        assert!(ro.is_dropped());
    }
}

#[test]
fn concurrent_drop_chain() {
    // root -> a -> b -> c -> d, all dropped concurrently
    for _ in 0..200 {
        let root = Node::root(Tracked::new());
        let a = root.fork(Tracked::new());
        let b = a.fork(Tracked::new());
        let c = b.fork(Tracked::new());
        let d = c.fork(Tracked::new());

        let barrier = Arc::new(Barrier::new(5));

        thread::scope(|s| {
            for node in [root, a, b, c, d] {
                let b = barrier.clone();
                s.spawn(move || {
                    b.wait();
                    drop(node);
                });
            }
        });
    }
}

#[test]
fn concurrent_fork_and_drop() {
    // One thread forks children, another drops siblings.
    for _ in 0..200 {
        let root = Node::root(Tracked::new());
        let root2 = root.fork(Tracked::new());

        let barrier = Arc::new(Barrier::new(2));
        let b1 = barrier.clone();

        thread::scope(|s| {
            let r = &root;
            s.spawn(move || {
                b1.wait();
                let _children: Vec<_> = (0..10).map(|_| r.fork(Tracked::new())).collect();
            });
            s.spawn(move || {
                barrier.wait();
                drop(root2);
            });
        });
    }
}

#[test]
fn concurrent_read_during_drop() {
    for _ in 0..200 {
        let root = Node::root(vec![1u32, 2, 3]);
        let child = root.fork(vec![4, 5, 6]);
        let leaf = child.fork(vec![7, 8, 9]);

        let barrier = Arc::new(Barrier::new(3));

        thread::scope(|s| {
            let b = barrier.clone();
            let r = &leaf;
            s.spawn(move || {
                b.wait();
                // Read while other threads are dropping ancestors.
                let _g = r.guard();
            });

            let b = barrier.clone();
            s.spawn(move || {
                b.wait();
                drop(root);
            });

            s.spawn(move || {
                barrier.wait();
                drop(child);
            });
        });
    }
}

#[test]
fn concurrent_traverse_during_drop() {
    for _ in 0..200 {
        let root = Node::root(vec![1u32]);
        let mid = root.fork(vec![2]);
        let leaf = mid.fork(vec![3]);

        let barrier = Arc::new(Barrier::new(3));

        thread::scope(|s| {
            let b = barrier.clone();
            let r = &leaf;
            s.spawn(move || {
                b.wait();
                let _: Vec<_> = r.traverse().collect();
            });

            let b = barrier.clone();
            s.spawn(move || {
                b.wait();
                drop(root);
            });

            s.spawn(move || {
                barrier.wait();
                drop(mid);
            });
        });
    }
}

#[test]
fn concurrent_write_during_drop() {
    for _ in 0..200 {
        let root = Node::root(vec![1u32]);
        let child = root.fork(vec![2]);

        let barrier = Arc::new(Barrier::new(2));
        let b1 = barrier.clone();

        thread::scope(|s| {
            let r = &child;
            s.spawn(move || {
                b1.wait();
                let mut g = r.guard_mut();
                g.data_mut().push(99);
            });
            s.spawn(move || {
                barrier.wait();
                drop(root);
            });
        });
    }
}

#[test]
fn concurrent_star_topology() {
    // Root with many children, all dropped concurrently.
    for _ in 0..100 {
        let root = Node::root(Tracked::new());
        let children: Vec<_> = (0..20).map(|_| root.fork(Tracked::new())).collect();
        drop(root);

        let barrier = Arc::new(Barrier::new(children.len()));

        thread::scope(|s| {
            for child in children {
                let b = barrier.clone();
                s.spawn(move || {
                    b.wait();
                    drop(child);
                });
            }
        });
    }
}

#[test]
fn concurrent_deep_tree_random_drops() {
    for _ in 0..100 {
        // Build a binary tree of depth 5
        let root = Node::root(Tracked::new());
        let mut leaves = vec![root.fork(Tracked::new()), root.fork(Tracked::new())];
        drop(root);

        for _ in 0..3 {
            let mut next = Vec::new();
            for node in &leaves {
                next.push(node.fork(Tracked::new()));
                next.push(node.fork(Tracked::new()));
            }
            // Drop all the parents — they're dead but have children.
            let old_leaves = std::mem::replace(&mut leaves, next);
            for node in old_leaves {
                drop(node);
            }
        }

        // Now drop all leaves concurrently.
        let barrier = Arc::new(Barrier::new(leaves.len()));
        thread::scope(|s| {
            for leaf in leaves {
                let b = barrier.clone();
                s.spawn(move || {
                    b.wait();
                    drop(leaf);
                });
            }
        });
    }
}
