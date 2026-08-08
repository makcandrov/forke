use std::{
    collections::{BinaryHeap, LinkedList},
    ffi::OsString,
    marker::PhantomData,
    mem::replace,
};

/// Folds a child's data into its parent when the child is removed.
///
/// # Panic safety
///
/// `merge` should not panic. A panic leaves the child's old value lost and
/// can surface on whichever thread happens to be running the cascade
/// callback. Use `Result` inside the data type instead.
///
/// # Re-entrancy
///
/// `merge` is called while internal node locks are held, on whichever
/// thread triggers the cascade. It must not touch the tree it belongs to
/// (forking, dropping nodes, taking guards, traversing): doing so can
/// deadlock.
pub trait Merge {
    /// Merges `child` into `parent`.
    fn merge(parent: &mut Self, child: Self);
}

/// Inverse merge — folds a parent's data into its child. Auto-derived for
/// every [`Merge`] type and used when a single-child parent is collapsed
/// downward.
///
/// # Example
/// ```
/// # use forke::MergeInv;
/// let mut child = vec![3, 4];
/// let parent = vec![1, 2];
/// <Vec<_> as MergeInv>::merge_inv(&mut child, parent);
/// assert_eq!(child, vec![1, 2, 3, 4]);
/// ```
pub trait MergeInv: Merge {
    /// Merges `parent` into `child`.
    fn merge_inv(child: &mut Self, parent: Self);
}

impl<T: Merge> MergeInv for T {
    #[inline]
    fn merge_inv(child: &mut Self, mut parent: Self) {
        parent = replace(child, parent);
        Merge::merge(child, parent);
    }
}

impl Merge for () {
    fn merge(_parent: &mut Self, _child: Self) {}
}

impl<T> Merge for PhantomData<T> {
    fn merge(_parent: &mut Self, _child: Self) {}
}

impl<T: Merge> Merge for Box<T> {
    #[inline]
    fn merge(parent: &mut Self, child: Self) {
        Merge::merge(parent.as_mut(), *child);
    }
}

impl<T: Merge> Merge for Option<T> {
    fn merge(parent: &mut Self, child: Self) {
        match (parent, child) {
            (_, None) => {}
            (parent @ None, child) => *parent = child,
            (Some(parent), Some(child)) => Merge::merge(parent, child),
        }
    }
}

impl<T: Merge, const N: usize> Merge for [T; N] {
    fn merge(parent: &mut Self, child: Self) {
        for (p, c) in parent.iter_mut().zip(child) {
            Merge::merge(p, c);
        }
    }
}

macro_rules! impl_merge_tuple_indexed {
    ($(($T:ident, $idx:tt)),+) => {
        impl<$($T: Merge),+> Merge for ($($T,)+) {
            fn merge(parent: &mut Self, child: Self) {
                $(Merge::merge(&mut parent.$idx, child.$idx);)+
            }
        }
    };
}

impl_merge_tuple_indexed!((A, 0));
impl_merge_tuple_indexed!((A, 0), (B, 1));
impl_merge_tuple_indexed!((A, 0), (B, 1), (C, 2));
impl_merge_tuple_indexed!((A, 0), (B, 1), (C, 2), (D, 3));
impl_merge_tuple_indexed!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4));
impl_merge_tuple_indexed!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5));
impl_merge_tuple_indexed!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6));
impl_merge_tuple_indexed!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7)
);
impl_merge_tuple_indexed!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8)
);
impl_merge_tuple_indexed!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9)
);
impl_merge_tuple_indexed!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10)
);
impl_merge_tuple_indexed!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10),
    (L, 11)
);
impl_merge_tuple_indexed!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10),
    (L, 11),
    (M, 12)
);
impl_merge_tuple_indexed!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10),
    (L, 11),
    (M, 12),
    (N, 13)
);
impl_merge_tuple_indexed!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10),
    (L, 11),
    (M, 12),
    (N, 13),
    (O, 14)
);
impl_merge_tuple_indexed!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10),
    (L, 11),
    (M, 12),
    (N, 13),
    (O, 14),
    (P, 15)
);

impl Merge for String {
    #[inline]
    fn merge(parent: &mut Self, child: Self) {
        parent.push_str(&child);
    }
}

impl Merge for OsString {
    #[inline]
    fn merge(parent: &mut Self, child: Self) {
        parent.push(child);
    }
}

impl<T> Merge for Vec<T> {
    #[inline]
    fn merge(parent: &mut Self, child: Self) {
        parent.extend(child);
    }
}

impl<T> Merge for LinkedList<T> {
    #[inline]
    fn merge(parent: &mut Self, mut child: Self) {
        parent.append(&mut child);
    }
}

impl<T: Ord> Merge for BinaryHeap<T> {
    fn merge(parent: &mut Self, child: Self) {
        if parent.len() < child.len() {
            let parent_items = replace(parent, child);
            parent.extend(parent_items);
        } else {
            parent.extend(child);
        }
    }
}

const _: () = {
    use std::collections::{BTreeMap, BTreeSet, btree_map};

    impl<K: Ord, V: Merge> Merge for BTreeMap<K, V> {
        fn merge(parent: &mut Self, mut child: Self) {
            if parent.len() < child.len() {
                // Folding the smaller side into the larger one is cheaper,
                // but it puts the parent's entries on the inserting side.
                // `insert` keeps the *existing* key, so a colliding entry
                // has to be removed first for the parent's key to win.
                child = replace(parent, child);
                for (key, mut value) in child {
                    if let Some(existing) = parent.remove(&key) {
                        Merge::merge(&mut value, existing);
                    }
                    parent.insert(key, value);
                }
            } else {
                for (key, value) in child {
                    match parent.entry(key) {
                        btree_map::Entry::Occupied(e) => Merge::merge(e.into_mut(), value),
                        btree_map::Entry::Vacant(e) => drop(e.insert(value)),
                    }
                }
            }
        }
    }

    impl<T: Ord> Merge for BTreeSet<T> {
        fn merge(parent: &mut Self, mut child: Self) {
            if parent.len() < child.len() {
                // See `BTreeMap`: `extend` keeps the existing element, so
                // the parent's side needs `replace` to win collisions.
                child = replace(parent, child);
                for value in child {
                    parent.replace(value);
                }
            } else {
                parent.extend(child);
            }
        }
    }
};

const _: () = {
    use std::collections::{HashMap, HashSet, hash_map};
    use std::hash::{BuildHasher, Hash};

    impl<K, V, S> Merge for HashMap<K, V, S>
    where
        K: Eq + Hash,
        V: Merge,
        S: BuildHasher,
    {
        fn merge(parent: &mut Self, mut child: Self) {
            if parent.len() < child.len() {
                // See `BTreeMap`: `insert` keeps the existing key, so a
                // colliding entry is removed first for the parent's to win.
                child = replace(parent, child);
                for (key, mut value) in child {
                    if let Some(existing) = parent.remove(&key) {
                        Merge::merge(&mut value, existing);
                    }
                    parent.insert(key, value);
                }
            } else {
                for (key, value) in child {
                    match parent.entry(key) {
                        hash_map::Entry::Occupied(e) => Merge::merge(e.into_mut(), value),
                        hash_map::Entry::Vacant(e) => drop(e.insert(value)),
                    }
                }
            }
        }
    }

    impl<T, S> Merge for HashSet<T, S>
    where
        T: Eq + Hash,
        S: BuildHasher,
    {
        fn merge(parent: &mut Self, mut child: Self) {
            if parent.len() < child.len() {
                // See `BTreeSet`.
                child = replace(parent, child);
                for value in child {
                    parent.replace(value);
                }
            } else {
                parent.extend(child);
            }
        }
    }
};
