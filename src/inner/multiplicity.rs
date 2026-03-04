use quick_impl::quick_impl_all;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[quick_impl_all(pub const is, pub into, pub try_into)]
pub(crate) enum Multiplicity<T> {
    None,
    Single(T),
    Multiple,
}

impl<T> FromIterator<T> for Multiplicity<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut iter = iter.into_iter();
        match iter.next() {
            None => Self::None,
            Some(first) => {
                if iter.next().is_some() {
                    Self::Multiple
                } else {
                    Self::Single(first)
                }
            }
        }
    }
}
