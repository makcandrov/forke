#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![doc = include_str!("../README.md")]

mod inner;
pub use inner::NodeGuard;
use inner::SelfHandle;

mod utils;

pub trait Merge {
    fn merge(parent: Self, child: &mut Self);
}

#[derive(Debug)]
pub struct Node<T: Merge> {
    inner: SelfHandle<T>,
}

impl<T: Merge> Node<T> {
    #[inline]
    pub fn root(data: T) -> Self {
        Self {
            inner: SelfHandle::root(data),
        }
    }

    #[inline]
    pub fn add_child(&self, data: T) -> Self {
        Self {
            inner: self.inner.create_child(data),
        }
    }

    #[inline]
    pub fn add_children(&self, data: impl IntoIterator<Item = T>) -> Vec<Self> {
        self.inner
            .create_children(data)
            .map(|inner| Self { inner })
            .collect()
    }

    pub fn read(&self) -> NodeGuard<'_, T> {
        self.inner.read()
    }

    pub fn search<U, F>(&self, f: F) -> Option<U>
    where
        F: Fn(&T) -> Option<U>,
    {
        self.read().search(f)
    }
}
