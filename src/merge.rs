use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, btree_map, hash_map},
    hash::{BuildHasher, Hash},
    mem::replace,
};

pub trait Merge {
    fn merge(parent: &mut Self, child: Self);
}

pub trait MergeInv: Merge {
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

#[cfg(feature = "hashbrown")]
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

        let merge_expected = HashMap::from([
            (5, "he".to_string()),
            (6, "hel".to_string()),
            (100, "hello world".to_string()),
            (101, "hello world!".to_string()),
            (500, "hello".to_string()),
        ]);

        assert_eq!(parent, merge_expected);
    }
}
