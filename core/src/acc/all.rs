use crate::{
    Acc, AccState, CapturedContext, Contextual, ContextualIter, Prioritized, VEC_SIZE,
};
use smallvec::SmallVec;

/// An allocator for an accumulator that collects all items.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct All;

/// An accumulator that collects all items.
pub struct AllState<T, C>(SmallVec<Contextual<T, C>, VEC_SIZE>);

impl Acc for All {
    type Acc<T, C> = AllState<T, C>;

    #[inline]
    fn create_state<T, C>() -> Self::Acc<T, C> {
        AllState(SmallVec::new())
    }
}

impl<T, C> IntoIterator for AllState<T, C> {
    type Item = Contextual<T, C>;
    type IntoIter = smallvec::IntoIter<Self::Item, VEC_SIZE>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T, C> AccState for AllState<T, C> {
    type Type = T;
    type Context = C;
    type Alloc = All;

    #[inline]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    fn iter(&'_ self) -> ContextualIter<'_, T, C> {
        ContextualIter::new(&self.0)
    }

    #[inline]
    fn map<U>(self, mut map: impl FnMut(T) -> U) -> <Self::Alloc as Acc>::Acc<U, C> {
        if self.is_empty() {
            return AllState(SmallVec::new());
        }

        AllState(self.into_iter().map(|ct| ct.map(&mut map)).collect())
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    #[inline]
    fn push_naive(&mut self, value: Contextual<T, C>) -> bool {
        self.0.push(value);
        true
    }

    #[inline]
    fn append_naive(&mut self, mut other: Self) -> usize {
        self.0.append(&mut other.0);
        0
    }

    #[inline]
    fn push(&mut self, value: Contextual<T, C>) -> bool
    where
        T: Prioritized,
    {
        self.push_naive(value)
    }

    #[inline]
    fn append(&mut self, other: Self) -> usize
    where
        T: Prioritized,
    {
        self.append_naive(other)
    }
}
