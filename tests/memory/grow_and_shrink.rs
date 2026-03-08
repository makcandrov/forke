use forke_test_utils::TrackingAllocator;

use forke::Node;

#[global_allocator]
static ALLOC: TrackingAllocator = TrackingAllocator::new();

#[test]
fn grow_and_shrink_repeatedly() {
    // Grow a tree, then tear it down, multiple times.
    // If there is a leak, memory will grow linearly across cycles.
    // Warmup cycle.
    {
        let root = Node::root(());
        let mut nodes = vec![];
        for _ in 0..200 {
            let parent = if nodes.is_empty() {
                &root
            } else {
                &nodes[nodes.len() / 2]
            };
            nodes.push(parent.fork(()));
        }
        drop(root);
        while let Some(n) = nodes.pop() {
            drop(n);
        }
    }

    let mem_before = ALLOC.allocated();

    for _ in 0..100 {
        let root = Node::root(());
        let mut nodes = vec![];
        for _ in 0..200 {
            let parent = if nodes.is_empty() {
                &root
            } else {
                &nodes[nodes.len() / 2]
            };
            nodes.push(parent.fork(()));
        }
        drop(root);
        while let Some(n) = nodes.pop() {
            drop(n);
        }
    }

    let mem_after = ALLOC.allocated();
    assert_eq!(
        mem_after, mem_before,
        "memory leak detected across 100 grow/shrink cycles"
    );
}
