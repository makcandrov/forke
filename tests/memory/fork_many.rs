use forke_test_utils::TrackingAllocator;

use forke::Node;

#[global_allocator]
static ALLOC: TrackingAllocator = TrackingAllocator::new();

#[test]
fn fork_many_then_drop_all() {
    // Warmup cycle.
    {
        let root = Node::root(());
        for _ in 0..20 {
            let children = root.fork_many((0..100).map(|_| ()));
            drop(children);
        }
        drop(root);
    }

    let mem_before = ALLOC.allocated();

    for _ in 0..10 {
        let root = Node::root(());
        for _ in 0..20 {
            let children = root.fork_many((0..100).map(|_| ()));
            drop(children);
        }
        drop(root);
    }

    let mem_after = ALLOC.allocated();
    assert_eq!(mem_after, mem_before, "memory leak detected in fork_many");
}
