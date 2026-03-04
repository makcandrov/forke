use crate::Merge;

pub trait NodeData: Merge + Send + Sync + 'static {}
impl<T> NodeData for T where T: Merge + Send + Sync + 'static {}
