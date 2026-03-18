# forke

A thread-safe tree with automatic node merging.

When a node is dropped and has at most one child, it is removed from the tree
and its data is folded into the remaining child (or simply discarded if it is a
leaf). Data folding is defined by the [`Merge`] trait. The process cascades
upward: if removing a node leaves its parent with a single child, the parent is
merged too.

## Quick start

```rust
use forke::{Merge, Node};

/// Per-node data: an append-only log of events.
#[derive(Debug, Default)]
struct Events(Vec<String>);

impl Merge for Events {
    fn merge(parent: &mut Self, child: Self) {
        parent.0.extend(child.0);
    }
}

// Build a tree
let root = Node::root(Events::default());
let child = root.fork(Events(vec!["created".into()]));
let grandchild = child.fork(Events(vec!["init".into()]));

// Read data through guards
assert_eq!(grandchild.guard().data().0, vec!["init"]);

// Search walks the path to the root
let has_created = grandchild.search(|e| {
    e.0.contains(&"created".to_string()).then_some(true)
});
assert_eq!(has_created, Some(true));

// Dropping `child` merges its events into `grandchild`
drop(child);
```

## How merging works

Consider the following tree where red nodes have been dropped and green nodes
are still alive:

<div align="center">
  <img src="https://raw.githubusercontent.com/makcandrov/forke/main/assets/graph0.png" width="400" />
</div>

When the user drops node **D**, a cascade begins:

**1.** Node D is a leaf -- it is discarded.

<div align="center">
  <img src="https://raw.githubusercontent.com/makcandrov/forke/main/assets/graph1.png" width="400" />
</div>

**2.** Node B is now dead with a single child -- it is merged into E.

<div align="center">
  <img src="https://raw.githubusercontent.com/makcandrov/forke/main/assets/graph2.png" width="400" />
</div>

Node A still has two children (C and B+E), so the cascade stops.

## Thread safety

All operations on `Node` are thread-safe. Nodes can be forked, dropped, read,
and traversed concurrently from any thread. The drop/merge logic uses
fine-grained locking with automatic retry: if a lock is contended during a merge
cascade, the operation is deferred and retried later rather than blocking the
caller.

## License

MIT OR Apache-2.0
