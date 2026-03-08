use std::thread;

use forke_test_utils::TrackingAllocator;

use forke::Node;

#[global_allocator]
static ALLOC: TrackingAllocator = TrackingAllocator::new();

#[test]
fn grow_and_shrink_concurrent() {
    // Multiple threads repeatedly fork and drop nodes on the same tree.
    // Warmup cycle.
    {
        let root = Node::root(());
        thread::scope(|s| {
            for _ in 0..5 {
                let r = &root;
                s.spawn(move || {
                    for _ in 0..500 {
                        let child = r.fork(());
                        let grandchild = child.fork(());
                        drop(child);
                        drop(grandchild);
                    }
                });
            }
        });
        drop(root);
    }

    let mem_before = ALLOC.allocated();

    for _ in 0..10 {
        let root = Node::root(());
        thread::scope(|s| {
            for _ in 0..5 {
                let r = &root;
                s.spawn(move || {
                    for _ in 0..500 {
                        let child = r.fork(());
                        let grandchild = child.fork(());
                        drop(child);
                        drop(grandchild);
                    }
                });
            }
        });
        drop(root);
    }

    let mem_after = ALLOC.allocated();
    assert_eq!(
        mem_after, mem_before,
        "memory leak detected in concurrent grow/shrink"
    );
}
