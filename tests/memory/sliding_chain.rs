use forke_test_utils::TrackingAllocator;
use rand::seq::SliceRandom;

use forke::Node;

#[global_allocator]
static ALLOC: TrackingAllocator = TrackingAllocator::new();

#[test]
fn sliding_chain_no_leak() {
    // Start with a chain of 100 nodes, then repeatedly extend by 100 and
    // drop the oldest 100, sliding the window forward. After 100 cycles the
    // chain has been fully replaced many times over — memory must stay flat.
    let root = Node::root(());
    let mut nodes: Vec<Node<()>> = vec![];
    for _ in 0..200 {
        let parent = nodes.last().unwrap_or(&root);
        nodes.push(parent.fork(()));
    }
    drop(root);

    let mut rng = rand::rng();

    // Run one cycle so the Vec capacity stabilizes after split_off.
    {
        let rest = nodes.split_off(100);
        let mut to_drop = nodes;
        to_drop.shuffle(&mut rng);
        for node in to_drop {
            drop(node);
        }
        nodes = rest;
        for _ in 0..100 {
            let parent = nodes.last().unwrap();
            nodes.push(parent.fork(()));
        }
    }

    let mem_before = ALLOC.allocated();

    for _ in 0..100 {
        // Drop the oldest 100 in random order.
        let rest = nodes.split_off(100);
        let mut to_drop = nodes;
        to_drop.shuffle(&mut rng);
        for node in to_drop {
            drop(node);
        }
        nodes = rest;

        // Extend chain by 100 nodes.
        for _ in 0..100 {
            let parent = nodes.last().unwrap();
            nodes.push(parent.fork(()));
        }
    }

    let mem_after = ALLOC.allocated();
    assert_eq!(
        mem_after, mem_before,
        "memory leak detected across 100 sliding cycles"
    );

    for node in nodes {
        drop(node);
    }
}
