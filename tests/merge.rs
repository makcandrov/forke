use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use forke::Merge;

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

/// Two `Tagged` values compare and hash equal when their keys match,
/// but stay distinguishable by their tag — so a set or map can be asked
/// which of two equal entries it kept.
#[derive(Debug, Clone, Copy, Eq)]
struct Tagged(i32, &'static str);

impl PartialEq for Tagged {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl std::hash::Hash for Tagged {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Ord for Tagged {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for Tagged {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Builds a parent and a child sharing key 0, padded so that the
/// requested side is the larger collection.
fn tagged_pair(parent_extra: i32, child_extra: i32) -> (Vec<Tagged>, Vec<Tagged>) {
    let parent = std::iter::once(Tagged(0, "parent"))
        .chain((1..=parent_extra).map(|i| Tagged(i, "parent")))
        .collect();
    let child = std::iter::once(Tagged(0, "child"))
        .chain((101..=100 + child_extra).map(|i| Tagged(i, "child")))
        .collect();
    (parent, child)
}

#[test]
fn merge_sets_keep_the_parent_element() {
    // Merging folds the child into the parent, so on an equal pair the
    // parent's element is the one that survives — whichever side
    // happens to hold more entries.
    fn hash_winner(parent_extra: i32, child_extra: i32) -> &'static str {
        let (p, c) = tagged_pair(parent_extra, child_extra);
        let mut parent: HashSet<Tagged> = p.into_iter().collect();
        Merge::merge(&mut parent, c.into_iter().collect());
        parent.get(&Tagged(0, "")).unwrap().1
    }

    fn btree_winner(parent_extra: i32, child_extra: i32) -> &'static str {
        let (p, c) = tagged_pair(parent_extra, child_extra);
        let mut parent: BTreeSet<Tagged> = p.into_iter().collect();
        Merge::merge(&mut parent, c.into_iter().collect());
        parent.get(&Tagged(0, "")).unwrap().1
    }

    assert_eq!(hash_winner(2, 0), "parent");
    assert_eq!(
        hash_winner(0, 2),
        "parent",
        "larger child overrode the parent"
    );
    assert_eq!(btree_winner(2, 0), "parent");
    assert_eq!(
        btree_winner(0, 2),
        "parent",
        "larger child overrode the parent"
    );
}

#[test]
fn merge_maps_keep_the_parent_key() {
    // Same rule for map keys, and the colliding value must still fold
    // parent-first — both regardless of the two maps' sizes.
    fn hash_winner(parent_extra: i32, child_extra: i32) -> (&'static str, String) {
        let (p, c) = tagged_pair(parent_extra, child_extra);
        let mut parent: HashMap<Tagged, String> =
            p.into_iter().map(|k| (k, "a".to_string())).collect();
        let child: HashMap<Tagged, String> = c.into_iter().map(|k| (k, "b".to_string())).collect();
        Merge::merge(&mut parent, child);
        let (key, value) = parent.iter().find(|(k, _)| k.0 == 0).unwrap();
        (key.1, value.clone())
    }

    fn btree_winner(parent_extra: i32, child_extra: i32) -> (&'static str, String) {
        let (p, c) = tagged_pair(parent_extra, child_extra);
        let mut parent: BTreeMap<Tagged, String> =
            p.into_iter().map(|k| (k, "a".to_string())).collect();
        let child: BTreeMap<Tagged, String> = c.into_iter().map(|k| (k, "b".to_string())).collect();
        Merge::merge(&mut parent, child);
        let (key, value) = parent.iter().find(|(k, _)| k.0 == 0).unwrap();
        (key.1, value.clone())
    }

    let expected = ("parent", "ab".to_string());
    assert_eq!(hash_winner(2, 0), expected);
    assert_eq!(
        hash_winner(0, 2),
        expected,
        "larger child overrode the parent"
    );
    assert_eq!(btree_winner(2, 0), expected);
    assert_eq!(
        btree_winner(0, 2),
        expected,
        "larger child overrode the parent"
    );
}

#[test]
fn merge_hashmap_successive_rounds() {
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
