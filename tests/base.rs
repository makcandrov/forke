use std::{
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, AtomicU64, Ordering::Relaxed},
    },
    thread::scope,
};

use forke::{Merge, Node};
use parking_lot::RwLock;

static ID: AtomicU64 = AtomicU64::new(0);

type Id = u64;

fn id() -> Id {
    ID.fetch_add(1, Relaxed)
}

#[derive(Debug)]
struct ContentDebug {
    id: Id,
    dropped: Arc<AtomicBool>,
    merged: Arc<RwLock<Vec<Id>>>,
}

#[derive(Debug)]
struct ContentObs {
    id: Id,
    dropped: Arc<AtomicBool>,
    merged: Arc<RwLock<Vec<Id>>>,
}

impl ContentDebug {
    pub fn new() -> Self {
        Self {
            dropped: Default::default(),
            id: id(),
            merged: Default::default(),
        }
    }

    pub fn obs(&self) -> ContentObs {
        ContentObs {
            id: self.id,
            dropped: self.dropped.clone(),
            merged: self.merged.clone(),
        }
    }

    pub fn new_with_obs() -> (Self, ContentObs) {
        let content = Self::new();
        let obs = content.obs();
        (content, obs)
    }
}

impl Drop for ContentDebug {
    fn drop(&mut self) {
        self.dropped.store(true, Relaxed);
    }
}

impl Merge for ContentDebug {
    fn merge(child: &mut Self, parent: Self) {
        child.merged.write().push(parent.id);
    }
}

impl ContentObs {
    fn dropped(&self) -> bool {
        self.dropped.load(Relaxed)
    }

    fn merges(&self) -> Vec<Id> {
        self.merged.read().clone()
    }
}

#[test]
fn test_root() {
    let (root_content, root_obs) = ContentDebug::new_with_obs();
    let root = Node::root(root_content);
    drop(root);
    assert!(root_obs.dropped());
}

#[test]
fn test_two_nodes1() {
    let (root_content, root_obs) = ContentDebug::new_with_obs();
    let (n1_content, n1_obs) = ContentDebug::new_with_obs();

    let root = Node::root(root_content);
    let n1 = root.add_child(n1_content);

    drop(root);
    assert!(root_obs.dropped());
    assert_eq!(root_obs.merges(), vec![]);
    assert_eq!(n1_obs.merges(), vec![root_obs.id]);

    drop(n1);
    assert!(n1_obs.dropped());
    assert_eq!(n1_obs.merges(), vec![root_obs.id]);
}

#[test]
fn test_two_nodes2() {
    let (root_content, root_obs) = ContentDebug::new_with_obs();
    let (n1_content, n1_obs) = ContentDebug::new_with_obs();

    let root = Node::root(root_content);
    let n1 = root.add_child(n1_content);

    drop(n1);
    assert!(n1_obs.dropped());
    assert_eq!(root_obs.merges(), vec![]);
    assert_eq!(n1_obs.merges(), vec![]);

    drop(root);
    assert!(root_obs.dropped());
    assert_eq!(root_obs.merges(), vec![]);
}

#[test]
fn test_two_concurrent() {
    let (root_content, root_obs) = ContentDebug::new_with_obs();
    let (n1_content, n1_obs) = ContentDebug::new_with_obs();

    let root = Node::root(root_content);
    let n1 = root.add_child(n1_content);

    let barrier = Barrier::new(2);

    scope(|s| {
        s.spawn(|| {
            barrier.wait();
            drop(root);
        });

        s.spawn(|| {
            barrier.wait();
            drop(n1);
        });
    });

    assert!(root_obs.dropped());
    assert!(n1_obs.dropped());
    assert_eq!(root_obs.merges(), vec![]);
    // assert_eq!(n1_obs.merges(), vec![root_obs.id]);
}

#[test]
fn test_readme() {
    let (a_content, _a_obs) = ContentDebug::new_with_obs();
    let (b_content, b_obs) = ContentDebug::new_with_obs();
    let (c_content, _c_obs) = ContentDebug::new_with_obs();
    let (d_content, d_obs) = ContentDebug::new_with_obs();
    let (e_content, e_obs) = ContentDebug::new_with_obs();
    let (f_content, _f_obs) = ContentDebug::new_with_obs();
    let (g_content, _g_obs) = ContentDebug::new_with_obs();
    let (h_content, _h_obs) = ContentDebug::new_with_obs();

    let a = Node::root(a_content);
    let b = a.add_child(b_content);
    let _c = a.add_child(c_content);
    let d = b.add_child(d_content);
    let e = b.add_child(e_content);
    let _f = e.add_child(f_content);
    let _g = e.add_child(g_content);
    let _h = e.add_child(h_content);

    drop(b);
    drop(e);

    assert!(!b_obs.dropped());
    assert!(!d_obs.dropped());
    assert!(!e_obs.dropped());

    drop(d);

    assert!(d_obs.dropped());
    assert!(b_obs.dropped());
    assert_eq!(e_obs.merges(), vec![b_obs.id]);
}
