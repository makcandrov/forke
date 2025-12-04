use hashbrown::HashMap;

use crate::{
    Merge,
    inner::{NodeIndex, SelfHandle, handle::StrongHandle},
};

pub struct DropQueue<T: Merge> {
    queues: HashMap<NodeIndex, Vec<StrongHandle<T>>>,
}
