use crate::{Merge, inner::SelfHandle};

#[derive(Debug)]
pub struct Node<T: Merge> {
    handle: SelfHandle<T>,
}

impl<T: Merge> Node<T> {
    #[inline]
    pub fn root(data: T) -> Self {
        Self {
            handle: SelfHandle::root(data),
        }
    }

    #[inline]
    pub fn add_child(&self, data: T) -> Self {
        Self {
            handle: SelfHandle {
                inner: self.handle.inner.create_child(data),
            },
        }
    }

    #[inline]
    pub fn add_children(&self, data: impl IntoIterator<Item = T>) -> Vec<Self> {
        self.handle
            .inner
            .create_children(data)
            .map(|inner| Self {
                handle: SelfHandle { inner },
            })
            .collect()
    }

    // pub fn read(&self) -> NodeGuard<'_, T> {
    //     self.handle.read()
    // }

    // pub fn search<U, F>(&self, f: F) -> Option<U>
    // where
    //     F: Fn(&T) -> Option<U>,
    // {
    //     self.read().search(f)
    // }
}
