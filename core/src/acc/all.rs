use crate::{
    Acc, AccState, CapturedContext, Contextual, ContextualIter, Prioritized, VEC_SIZE,
};
use smallvec::SmallVec;

/// An allocator for an accumulator that collects all items.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct All;

/// An accumulator that collects all items.
pub type AllState<T, C> = SmallVec<Contextual<T, C>, VEC_SIZE>;

impl Acc for All {
    type Acc<T, C: CapturedContext> = AllState<T, C>;

    #[inline]
    fn create_state<T, C: CapturedContext>() -> Self::Acc<T, C> {
        SmallVec::new()
    }
}

impl<T, C: CapturedContext> AccState<T, C> for AllState<T, C> {
    type Alloc = All;

    #[inline]
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    #[inline]
    fn len(&self) -> usize {
        self.len()
    }

    #[inline]
    fn iter(&'_ self) -> ContextualIter<'_, T, C> {
        ContextualIter::new(self)
    }

    #[inline]
    fn map<U>(self, mut map: impl FnMut(T) -> U) -> <Self::Alloc as Acc>::Acc<U, C> {
        if self.is_empty() {
            return SmallVec::new();
        }

        self.into_iter().map(|ct| ct.map(&mut map)).collect()
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }

    #[inline]
    fn push_naive(&mut self, value: Contextual<T, C>) -> bool {
        self.push(value);
        true
    }

    #[inline]
    fn append_naive(&mut self, mut other: Self) -> usize {
        self.append(&mut other);
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
