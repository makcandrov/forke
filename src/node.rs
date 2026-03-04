use crate::{NodeData, NodeGuard, inner::Handle};

#[derive(Debug)]
pub struct Node<T: NodeData> {
    handle: Handle<T>,
}

impl<T: NodeData> Node<T> {
    #[inline]
    pub fn root(data: T) -> Self {
        Self {
            handle: Handle::root(data),
        }
    }

    #[inline]
    pub fn add_child(&self, data: T) -> Self {
        Self {
            handle: self.handle.create_child(data),
        }
    }

    #[inline]
    pub fn add_children(&self, data: impl IntoIterator<Item = T>) -> Vec<Self> {
        self.handle
            .create_children(data)
            .map(|handle| Self { handle })
            .collect()
    }

    pub fn read(&self) -> NodeGuard<'_, T> {
        self.handle.node_guard()
    }

    pub fn search<U, F>(&self, f: F) -> Option<U>
    where
        F: Fn(&T) -> Option<U>,
    {
        self.read().search(f)
    }
}

impl<T: NodeData> Drop for Node<T> {
    fn drop(&mut self) {
        self.handle.try_drop(true);
    }
}
