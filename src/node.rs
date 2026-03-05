use crate::{
    AncestorGuards, AncestorIter, NodeData, NodeGuard, StaticNodeGuard, inner::StrongHandle,
};

#[derive(Debug)]
pub struct Node<T: NodeData> {
    handle: StrongHandle<T>,
}

impl<T: NodeData> Node<T> {
    #[inline]
    pub fn root(data: T) -> Self {
        Self {
            handle: StrongHandle::root(data),
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

    pub fn guard(&self) -> NodeGuard<'_, T> {
        self.handle.node_guard()
    }

    pub fn static_guard(&self) -> StaticNodeGuard<T> {
        self.handle.clone().static_node_guard()
    }

    pub fn iter<'a>(&self, storage: &'a mut AncestorGuards<T>) -> AncestorIter<'a, T> {
        AncestorIter::new(&self.handle, storage)
    }

    pub fn search<U, F>(&self, f: F) -> Option<U>
    where
        F: Fn(&T) -> Option<U>,
    {
        self.iter(&mut AncestorGuards::new()).find_map(f)
    }
}

impl<T: NodeData> Drop for Node<T> {
    fn drop(&mut self) {
        self.handle.try_drop(true);
    }
}
