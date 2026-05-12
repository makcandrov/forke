use hashbrown::HashMap;
use litemap::LiteMap;
use quick_impl::quick_impl_all;
use rustc_hash::FxBuildHasher;

use crate::{NodeData, inner::WeakHandle};

type FxMap<T> = HashMap<u64, WeakHandle<T>, FxBuildHasher>;

/// `Many` → `Huge` once `len > PROMOTE_AT` after an insert.
const PROMOTE_AT: usize = 32;

/// `Huge` → `Many` once `len < DEMOTE_AT` after a remove. The gap with
/// `PROMOTE_AT` is hysteresis so a node oscillating around the boundary
/// doesn't reallocate on every op.
const DEMOTE_AT: usize = 16;

const _: () = assert!(DEMOTE_AT * 2 <= PROMOTE_AT, "insufficient hysteresis gap");

#[derive(Debug, Clone, Default)]
#[quick_impl_all(pub as_ref, pub into, set)]
pub(crate) enum ChildrenMap<T: NodeData> {
    #[default]
    None,
    Single {
        index: u64,
        handle: WeakHandle<T>,
    },
    Many(LiteMap<u64, WeakHandle<T>>),
    Huge(FxMap<T>),
}

impl<T: NodeData> ChildrenMap<T> {
    pub fn new() -> Self {
        Self::None
    }

    pub fn insert(&mut self, index: u64, handle: WeakHandle<T>) {
        match self {
            Self::None => {
                *self = Self::Single { index, handle };
            }
            Self::Single { .. } => {
                let (o_index, o_handle) = self.set_none().into_single().unwrap();

                if o_index == index {
                    panic!("index duplicate: {index}");
                }

                let mut map = LiteMap::with_capacity(2);
                map.insert(o_index, o_handle);
                map.insert(index, handle);
                *self = Self::Many(map);
            }
            Self::Many(map) => {
                if map.insert(index, handle).is_some() {
                    panic!("index duplicate: {index}");
                }
                if map.len() > PROMOTE_AT {
                    let lite = self.set_none().into_many().unwrap();
                    let mut huge = FxMap::with_capacity_and_hasher(lite.len(), FxBuildHasher);
                    huge.extend(lite);
                    *self = Self::Huge(huge);
                }
            }
            Self::Huge(map) => {
                if map.insert(index, handle).is_some() {
                    panic!("index duplicate: {index}");
                }
            }
        }
    }

    pub fn remove(&mut self, index: &u64) -> Option<WeakHandle<T>> {
        match self {
            Self::None => None,
            Self::Single { index: o_index, .. } => {
                if index != o_index {
                    return None;
                }
                let (_, handle) = self.set_none().into_single().unwrap();
                Some(handle)
            }
            Self::Many(map) => {
                let res = map.remove(index);
                if res.is_some() && map.len() == 1 {
                    let map = self.set_none().into_many().unwrap();
                    let (index, handle) = map.into_iter().next().unwrap();
                    *self = Self::Single { index, handle };
                }
                res
            }
            Self::Huge(map) => {
                let res = map.remove(index);
                if res.is_some() && map.len() < DEMOTE_AT {
                    let huge = self.set_none().into_huge().unwrap();
                    let mut entries: Vec<(u64, WeakHandle<T>)> = huge.into_iter().collect();
                    entries.sort_unstable_by_key(|(k, _)| *k);
                    *self = Self::Many(LiteMap::from_sorted_store_unchecked(entries));
                }
                res
            }
        }
    }
}
