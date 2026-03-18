use std::{
    collections::{BinaryHeap, LinkedList},
    ffi::OsString,
    marker::PhantomData,
    mem::replace,
};

/// Defines how a child node's data is folded into its parent when the child
/// is removed from the tree.
pub trait Merge {
    /// Merges `child` into `parent`.
    fn merge(parent: &mut Self, child: Self);
}

/// Inverse merge direction — folds a parent's data into its child.
///
/// This trait is automatically implemented for all [`Merge`] types by swapping
/// the operands and calling [`Merge::merge`]. This is used internally when a
/// parent node with a single child is merged up the tree: the parent's data is
/// merged into the child (rather than the typical child-into-parent).
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
    #[inline(always)]
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
            let merge = if parent.len() < child.len() {
                child = replace(parent, child);
                <V as MergeInv>::merge_inv
            } else {
                <V as Merge>::merge
            };

            for (k, v) in child {
                match parent.entry(k) {
                    btree_map::Entry::Occupied(e) => merge(e.into_mut(), v),
                    btree_map::Entry::Vacant(e) => drop(e.insert(v)),
                }
            }
        }
    }

    impl<T: Ord> Merge for BTreeSet<T> {
        fn merge(parent: &mut Self, mut child: Self) {
            if parent.len() < child.len() {
                child = replace(parent, child);
            }
            parent.extend(child);
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
            let merge = if parent.len() < child.len() {
                child = replace(parent, child);
                <V as MergeInv>::merge_inv
            } else {
                <V as Merge>::merge
            };

            for (k, v) in child {
                match parent.entry(k) {
                    hash_map::Entry::Occupied(e) => merge(e.into_mut(), v),
                    hash_map::Entry::Vacant(e) => drop(e.insert(v)),
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
                child = replace(parent, child);
            }
            parent.extend(child);
        }
    }
};

const _: () = {
    use hashbrown::{HashMap, HashSet, hash_map};
    use std::hash::{BuildHasher, Hash};

    impl<T, S> Merge for HashSet<T, S>
    where
        T: Eq + Hash,
        S: BuildHasher,
    {
        fn merge(parent: &mut Self, mut child: Self) {
            if parent.len() < child.len() {
                child = replace(parent, child);
            }
            parent.extend(child);
        }
    }

    impl<K, V, S> Merge for HashMap<K, V, S>
    where
        K: Eq + Hash,
        V: Merge,
        S: BuildHasher,
    {
        fn merge(parent: &mut Self, mut child: Self) {
            let merge = if parent.len() < child.len() {
                child = replace(parent, child);
                <V as MergeInv>::merge_inv
            } else {
                <V as Merge>::merge
            };

            for (k, v) in child {
                match parent.entry(k) {
                    hash_map::Entry::Occupied(e) => merge(e.into_mut(), v),
                    hash_map::Entry::Vacant(e) => drop(e.insert(v)),
                }
            }
        }
    }
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

    #[test]
    fn merge_vec() {
        let mut parent = vec![1, 2];
        Merge::merge(&mut parent, vec![3, 4]);
        assert_eq!(parent, vec![1, 2, 3, 4]);
    }

    #[test]
    fn merge_string() {
        let mut parent = String::from("hello");
        Merge::merge(&mut parent, " world".into());
        assert_eq!(parent, "hello world");
    }

    #[test]
    fn merge_option_both_some() {
        let mut parent = Some(vec![1]);
        Merge::merge(&mut parent, Some(vec![2]));
        assert_eq!(parent, Some(vec![1, 2]));
    }

    #[test]
    fn merge_option_parent_none() {
        let mut parent: Option<Vec<i32>> = None;
        Merge::merge(&mut parent, Some(vec![1]));
        assert_eq!(parent, Some(vec![1]));
    }

    #[test]
    fn merge_option_child_none() {
        let mut parent = Some(vec![1]);
        Merge::merge(&mut parent, None);
        assert_eq!(parent, Some(vec![1]));
    }

    #[test]
    fn merge_option_both_none() {
        let mut parent: Option<Vec<i32>> = None;
        Merge::merge(&mut parent, None);
        assert_eq!(parent, None);
    }

    #[test]
    fn merge_hashset() {
        let mut parent: HashSet<i32> = [1, 2].into();
        Merge::merge(&mut parent, [2, 3].into());
        assert_eq!(parent, [1, 2, 3].into());
    }

    #[test]
    fn merge_hashset_swap() {
        let mut parent: HashSet<i32> = [1].into();
        Merge::merge(&mut parent, [10, 20, 30].into());
        assert_eq!(parent, [1, 10, 20, 30].into());
    }

    #[test]
    fn merge_btreeset() {
        let mut parent: BTreeSet<i32> = [1, 2].into();
        Merge::merge(&mut parent, [2, 3].into());
        assert_eq!(parent, [1, 2, 3].into());
    }

    #[test]
    fn merge_btreeset_swap() {
        let mut parent: BTreeSet<i32> = [1].into();
        Merge::merge(&mut parent, [10, 20, 30].into());
        assert_eq!(parent, [1, 10, 20, 30].into());
    }

    #[test]
    fn merge_hashmap_normal() {
        let mut parent: HashMap<i32, String> = HashMap::from([(1, "a".into()), (2, "b".into())]);
        let child: HashMap<i32, String> = HashMap::from([(2, "c".into()), (3, "d".into())]);
        Merge::merge(&mut parent, child);
        assert_eq!(parent[&1], "a");
        assert_eq!(parent[&2], "bc");
        assert_eq!(parent[&3], "d");
    }

    #[test]
    fn merge_hashmap_swap() {
        let mut parent: HashMap<i32, String> = HashMap::from([(1, "a".into())]);
        let child: HashMap<i32, String> =
            HashMap::from([(1, "b".into()), (2, "c".into()), (3, "d".into())]);
        Merge::merge(&mut parent, child);
        assert_eq!(parent[&1], "ab");
        assert_eq!(parent[&2], "c");
        assert_eq!(parent[&3], "d");
    }

    #[test]
    fn merge_btreemap_normal() {
        let mut parent: BTreeMap<i32, String> = BTreeMap::from([(1, "a".into()), (2, "b".into())]);
        let child: BTreeMap<i32, String> = BTreeMap::from([(2, "c".into()), (3, "d".into())]);
        Merge::merge(&mut parent, child);
        assert_eq!(parent[&1], "a");
        assert_eq!(parent[&2], "bc");
        assert_eq!(parent[&3], "d");
    }

    #[test]
    fn merge_btreemap_swap() {
        let mut parent: BTreeMap<i32, String> = BTreeMap::from([(1, "a".into())]);
        let child: BTreeMap<i32, String> =
            BTreeMap::from([(1, "b".into()), (2, "c".into()), (3, "d".into())]);
        Merge::merge(&mut parent, child);
        assert_eq!(parent[&1], "ab");
        assert_eq!(parent[&2], "c");
        assert_eq!(parent[&3], "d");
    }

    #[test]
    fn test_hashmap_merge() {
        let mut parent = HashMap::from([
            (5, "he".to_string()),
            (100, "hello".to_string()),
            (101, "hello wo".to_string()),
        ]);

        let child = HashMap::from([
            (6, "hel".to_string()),
            (100, " world".to_string()),
            (101, "rld!".to_string()),
            (500, "hello".to_string()),
        ]);

        Merge::merge(&mut parent, child);

        {
            let merge_expected = HashMap::from([
                (5, "he".to_string()),
                (6, "hel".to_string()),
                (100, "hello world".to_string()),
                (101, "hello world!".to_string()),
                (500, "hello".to_string()),
            ]);

            assert_eq!(parent, merge_expected);
        }

        let child = HashMap::from([
            (1, "hello w".to_string()),
            (500, " wo".to_string()),
            (501, "hello wor".to_string()),
        ]);

        Merge::merge(&mut parent, child);

        {
            let merge_expected = HashMap::from([
                (1, "hello w".to_string()),
                (5, "he".to_string()),
                (6, "hel".to_string()),
                (100, "hello world".to_string()),
                (101, "hello world!".to_string()),
                (500, "hello wo".to_string()),
                (501, "hello wor".to_string()),
            ]);

            assert_eq!(parent, merge_expected);
        }
    }
}
