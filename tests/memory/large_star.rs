use forke_test_utils::TrackingAllocator;

use forke::Node;

#[global_allocator]
static ALLOC: TrackingAllocator = TrackingAllocator::new();

#[test]
fn large_star_no_leak() {
    // Warmup cycle.
    {
        let root = Node::root(());
        let children: Vec<_> = (0..1000).map(|_| root.fork(())).collect();
        drop(root);
        for child in children {
            drop(child);
        }
    }

    let mem_before = ALLOC.allocated();

    for _ in 0..10 {
        let root = Node::root(());
        let children: Vec<_> = (0..1000).map(|_| root.fork(())).collect();
        drop(root);
        for child in children {
            drop(child);
        }
    }

    let mem_after = ALLOC.allocated();
    assert_eq!(
        mem_after, mem_before,
        "memory leak detected in large star topology"
    );
}
