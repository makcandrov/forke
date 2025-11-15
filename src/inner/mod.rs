use hashbrown::HashMap;

use crate::Merge;

mod guard;
pub use guard::NodeGuard;

mod handle;
pub(crate) use handle::SelfHandle;
use handle::{ChildHandle, ParentHandle};

#[derive(Debug)]
struct NodeInner<T: Merge> {
    parent: Option<ParentHandle<T>>,
    children: HashMap<usize, ChildHandle<T>>,
    alive: bool,
    next_index: usize,
    data: T,
}

impl<T: Merge> NodeInner<T> {
    pub fn root(data: T) -> Self {
        Self {
            parent: None,
            children: HashMap::new(),
            alive: true,
            next_index: 0,
            data,
        }
    }

    fn add_child_with<U, F>(&mut self, f: F) -> U
    where
        F: FnOnce(usize) -> (ChildHandle<T>, U),
    {
        let child_index = self.next_index;
        self.next_index += 1;
        let (child_handle, return_data) = f(child_index);
        let old = self.children.insert(child_index, child_handle);
        assert!(old.is_none(), "child index duplicate");
        return_data
    }

    #[inline]
    fn add_child(&mut self, child_handle: ChildHandle<T>) -> usize {
        self.add_child_with(|child_index| (child_handle, child_index))
    }
}
