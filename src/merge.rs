use std::{
    hash::{BuildHasher, Hash},
    mem::replace,
};

pub trait Merge {
    fn merge(child: &mut Self, parent: Self);
}

impl<T> Merge for Option<T>
where
    T: Merge,
{
    fn merge(child: &mut Self, parent: Self) {
        match (child, parent) {
            (_, None) => {}
            (child @ None, parent) => *child = parent,
            (Some(child), Some(parent)) => Merge::merge(child, parent),
        }
    }
}

// std
const _: () = {
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, btree_map, hash_map};

    impl<T, S> Merge for HashSet<T, S>
    where
        T: Eq + Hash,
        S: BuildHasher,
    {
        fn merge(child: &mut Self, mut parent: Self) {
            if child.len() < parent.len() {
                parent = replace(child, parent);
            }
            child.extend(parent);
        }
    }

    impl<T: Ord> Merge for BTreeSet<T> {
        fn merge(child: &mut Self, mut parent: Self) {
            if child.len() < parent.len() {
                parent = replace(child, parent);
            }
            child.extend(parent);
        }
    }

    impl<K, V, S> Merge for HashMap<K, V, S>
    where
        K: Eq + Hash,
        V: Merge,
        S: BuildHasher,
    {
        fn merge(child: &mut Self, mut parent: Self) {
            if child.len() < parent.len() {
                parent = replace(child, parent);
            }
            for (k, v) in parent {
                match child.entry(k) {
                    hash_map::Entry::Occupied(mut e) => Merge::merge(e.get_mut(), v),
                    hash_map::Entry::Vacant(e) => {
                        e.insert(v);
                    }
                }
            }
        }
    }

    impl<K: Ord, V: Merge> Merge for BTreeMap<K, V> {
        fn merge(child: &mut Self, mut parent: Self) {
            if child.len() < parent.len() {
                parent = replace(child, parent);
            }
            for (k, v) in parent {
                match child.entry(k) {
                    btree_map::Entry::Occupied(mut e) => Merge::merge(e.get_mut(), v),
                    btree_map::Entry::Vacant(e) => {
                        e.insert(v);
                    }
                }
            }
        }
    }
};

// hashbrown
#[cfg(feature = "hashbrown")]
const _: () = {
    use hashbrown::{HashMap, HashSet, hash_map};

    impl<T, S> Merge for HashSet<T, S>
    where
        T: Eq + Hash,
        S: BuildHasher,
    {
        fn merge(child: &mut Self, mut parent: Self) {
            if child.len() < parent.len() {
                parent = replace(child, parent);
            }
            child.extend(parent);
        }
    }

    impl<K, V, S> Merge for HashMap<K, V, S>
    where
        K: Eq + Hash,
        V: Merge,
        S: BuildHasher,
    {
        fn merge(child: &mut Self, mut parent: Self) {
            if child.len() < parent.len() {
                parent = replace(child, parent);
            }
            for (k, v) in parent {
                match child.entry(k) {
                    hash_map::Entry::Occupied(mut e) => Merge::merge(e.get_mut(), v),
                    hash_map::Entry::Vacant(e) => {
                        e.insert(v);
                    }
                }
            }
        }
    }
};

// indexmap
#[cfg(feature = "indexmap")]
const _: () = {
    use indexmap::{IndexMap, IndexSet, map};

    impl<T, S> Merge for IndexSet<T, S>
    where
        T: Eq + Hash,
        S: BuildHasher,
    {
        fn merge(child: &mut Self, mut parent: Self) {
            if child.len() < parent.len() {
                parent = replace(child, parent);
            }
            child.extend(parent);
        }
    }

    impl<K, V, S> Merge for IndexMap<K, V, S>
    where
        K: Eq + Hash,
        V: Merge,
        S: BuildHasher,
    {
        fn merge(child: &mut Self, mut parent: Self) {
            if child.len() < parent.len() {
                parent = replace(child, parent);
            }
            for (k, v) in parent {
                match child.entry(k) {
                    map::Entry::Occupied(mut e) => Merge::merge(e.get_mut(), v),
                    map::Entry::Vacant(e) => {
                        e.insert(v);
                    }
                }
            }
        }
    }
};
