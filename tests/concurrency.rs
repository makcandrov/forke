//! Concurrent forks, drops, reads, and traversals on a shared tree.

use std::sync::{Arc, Barrier};
use std::thread;

use forke::Node;
use forke_test_utils::{Obs, Tracked};
use parking_lot::Mutex;
use rand::RngExt;

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

#[test]
fn concurrent_fork_and_drop() {
    // One thread forks children, another drops a sibling.
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
fn concurrent_traverse_step_by_step_during_merge() {
    // Guards are yielded and dropped one at a time while an ancestor is
    // merged away concurrently. The read lock `TraverseIter` keeps on the
    // previously yielded node prevents its parent from being collapsed
    // between two `next` calls.
    for _ in 0..1000 {
        let root = Node::root(vec![1u32]);
        let mid = root.fork(vec![2]);
        let leaf = mid.fork(vec![3]);

        let barrier = Arc::new(Barrier::new(2));

        thread::scope(|s| {
            let b = barrier.clone();
            let r = &leaf;
            s.spawn(move || {
                b.wait();
                let it = r.traverse();
                for g in it {
                    let _ = g.data().clone();
                    drop(g);
                    std::thread::yield_now();
                }
            });

            s.spawn(move || {
                barrier.wait();
                drop(mid);
            });
        });

        drop(leaf);
        drop(root);
    }
}

#[test]
fn traverse_and_search_reach_live_root_under_concurrent_collapse() {
    // A traversal anchors each visited node with a read lock before
    // stepping to its parent, so a concurrent merge can never detach the
    // path: every traversal and search started from a live node ends at
    // the live root.
    for _ in 0..300 {
        let root = Node::root(vec![0u64]);

        let mut interior = Vec::new();
        let mut cur = root.fork(vec![1]);
        for i in 2..=6u64 {
            let next = cur.fork(vec![i]);
            interior.push(cur);
            cur = next;
        }
        let leaf = cur;

        let barrier = Arc::new(Barrier::new(interior.len() + 3));

        thread::scope(|s| {
            for node in interior {
                let b = barrier.clone();
                s.spawn(move || {
                    b.wait();
                    drop(node);
                });
            }

            let b = barrier.clone();
            let l = &leaf;
            s.spawn(move || {
                b.wait();
                let items: Vec<Vec<u64>> = l.traverse().map(|g| g.data().clone()).collect();
                assert_eq!(
                    items.last().unwrap(),
                    &vec![0],
                    "traverse must always end at the live root"
                );
            });

            let b = barrier.clone();
            let l = &leaf;
            s.spawn(move || {
                b.wait();
                let found = l.search(|v| v.contains(&0).then_some(()));
                assert!(found.is_some(), "Node::search must reach the live root");
            });

            let b = barrier.clone();
            let l = &leaf;
            s.spawn(move || {
                b.wait();
                let g = l.guard();
                let found = g.search(|v| v.contains(&0).then_some(()));
                assert!(
                    found.is_some(),
                    "NodeGuard::search must reach the live root"
                );
            });
        });
    }
}

#[test]
fn stress_random_concurrent_ops_with_accounting() {
    // Threads randomly fork, drop, read, traverse, and search nodes of
    // shared trees. Afterwards every tracked payload must have been
    // dropped and no payload may have been merged into two places.
    const THREADS: usize = 8;
    const OPS: usize = 400;

    let observers = Mutex::new(Vec::<Obs>::new());
    let pool = Mutex::new(Vec::<Node<Tracked>>::new());

    {
        let (d, o) = Tracked::pair();
        observers.lock().push(o);
        pool.lock().push(Node::root(d));
    }

    thread::scope(|s| {
        for _ in 0..THREADS {
            let pool = &pool;
            let observers = &observers;
            s.spawn(move || {
                let mut rng = rand::rng();
                for _ in 0..OPS {
                    let node = {
                        let mut p = pool.lock();
                        if p.is_empty() {
                            let (d, o) = Tracked::pair();
                            observers.lock().push(o);
                            p.push(Node::root(d));
                        }
                        let i = rng.random_range(0..p.len());
                        p.swap_remove(i)
                    };

                    match rng.random_range(0..100u32) {
                        // Fork 1..=3 children.
                        0..35 => {
                            let mut children = Vec::new();
                            for _ in 0..rng.random_range(1..=3) {
                                let (d, o) = Tracked::pair();
                                observers.lock().push(o);
                                children.push(node.fork(d));
                            }
                            let mut p = pool.lock();
                            p.push(node);
                            p.extend(children);
                        }
                        // Drop the node (its subtree may still be pooled).
                        35..60 => drop(node),
                        // Traverse to the root.
                        60..75 => {
                            assert!(node.traverse().count() >= 1);
                            pool.lock().push(node);
                        }
                        // Read guard.
                        75..90 => {
                            let _ = node.guard().data().id();
                            pool.lock().push(node);
                        }
                        // Search the whole path (never matches).
                        _ => {
                            let _ = node.search(|d| (d.id() == u64::MAX).then_some(()));
                            pool.lock().push(node);
                        }
                    }
                }
            });
        }
    });

    drop(pool.into_inner());

    let observers = observers.into_inner();
    for o in &observers {
        assert!(o.is_dropped(), "leaked node data (id {})", o.id());
    }

    // Data is folded into at most one destination, exactly once: no payload
    // may appear as a merge source twice.
    let mut merged: Vec<u64> = observers.iter().flat_map(|o| o.merges()).collect();
    let total = merged.len();
    merged.sort_unstable();
    merged.dedup();
    assert_eq!(
        total,
        merged.len(),
        "some payload was merged into two places"
    );
}
