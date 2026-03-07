use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, btree_map, hash_map},
    hash::{BuildHasher, Hash},
    mem::replace,
};

/// Defines how a child node's data is folded into its parent when the child
/// is removed from the tree.
pub trait Merge {
    /// Merges `child` into `parent`.
    fn merge(parent: &mut Self, child: Self);
}

/// Inverse merge direction — folds a parent's data into its child. This is
/// auto-implemented for all [`Merge`] types by swapping then merging.
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

impl<T> Merge for Option<T>
where
    T: Merge,
{
    fn merge(parent: &mut Self, child: Self) {
        match (parent, child) {
            (_, None) => {}
            (parent @ None, child) => *parent = child,
            (Some(parent), Some(child)) => Merge::merge(parent, child),
        }
    }
}

impl<T> Merge for Vec<T> {
    #[inline]
    fn merge(parent: &mut Self, child: Self) {
        parent.extend(child);
    }
}

impl Merge for String {
    #[inline]
    fn merge(parent: &mut Self, child: Self) {
        parent.push_str(&child);
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

impl<T: Ord> Merge for BTreeSet<T> {
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

const _: () = {
    use hashbrown::{HashMap, HashSet, hash_map};

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
