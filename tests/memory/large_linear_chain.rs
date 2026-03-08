use forke_test_utils::TrackingAllocator;

use forke::Node;

#[global_allocator]
static ALLOC: TrackingAllocator = TrackingAllocator::new();

#[test]
fn large_linear_chain_no_leak() {
    // Build a long chain, then drop from root down.
    // Warmup cycle.
    {
        let root = Node::root(());
        let mut nodes = vec![root];
        for _ in 1..1000 {
            let child = nodes.last().unwrap().fork(());
            nodes.push(child);
        }
        let leaf = nodes.pop().unwrap();
        for node in nodes {
            drop(node);
        }
        drop(leaf);
    }

    let mem_before = ALLOC.allocated();

    for _ in 0..10 {
        let root = Node::root(());
        let mut nodes = vec![root];
        for _ in 1..1000 {
            let child = nodes.last().unwrap().fork(());
            nodes.push(child);
        }
        let leaf = nodes.pop().unwrap();
        for node in nodes {
            drop(node);
        }
        drop(leaf);
    }

    let mem_after = ALLOC.allocated();
    assert_eq!(
        mem_after, mem_before,
        "memory leak detected in large linear chain"
    );
}
