# forke

`forke` is a thread-safe tree implementation in Rust, where each node can store arbitrary, generic data.

The core idea behind `forke` is that when a node is no longer needed, it is merged with its descendant or discarded if there are none. A node is considered useless if it has been dropped by the user and has no more than one child. When a merge happens, the associated data is combined using the `Merge` trait.

## Example

Consider the following tree where red nodes are dropped by the user, and green nodes are still in use:

```mermaid
graph TD
    classDef red fill:#ff0000,stroke:#ff0000,color:#ffffff;
    classDef green fill:#00ff00,stroke:#00ff00,color:#000000;

    A((A))-->B((B));
    A-->C((C));
    B-->D((D));
    B-->E((E));
    E-->F((F));
    E-->G((G));
    E-->H((H));

    class B,E red;
    class A,C,D,F,G,H green;
```

Let’s assume the user drops node `D`. The following changes will occur in the tree:

1. **Node `D` is discarded:** The node D and its associated data are removed from the tree.

```mermaid
graph TD
    classDef red fill:#ff0000,stroke:#ff0000,color:#ffffff;
    classDef green fill:#00ff00,stroke:#00ff00,color:#000000;

    A((A))-->B((B));
    A-->C((C));
    B-->E((E));
    E-->F((F));
    E-->G((G));
    E-->H((H));

    class B,E red;
    class A,C,D,F,G,H green;
```

2. **Node `B` is considered useless:** Since node `B` now only has one child (node `E`), it is considered useless and is removed. The data associated with node `B` is merged with node `E`'s data.

```mermaid
graph TD
    classDef red fill:#ff0000,stroke:#ff0000,color:#ffffff;
    classDef green fill:#00ff00,stroke:#00ff00,color:#000000;

    A((A))-->E((B+E));
    A-->C((C));
    E-->F((F));
    E-->G((G));
    E-->H((H));

    class E red;
    class A,C,D,F,G,H green;
```
