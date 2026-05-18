use crate::{Acc, AccState, CapturedContext, Contextual, ContextualIter, Prioritized};

/// An allocator for an accumulator that collects only a single item (highest priority).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Most;

/// An accumulator that collects only a single item (highest priority).
pub type MostAcc<T, C> = Option<Contextual<T, C>>;

impl Acc for Most {
    type Acc<T, C: CapturedContext> = MostAcc<T, C>;

    #[inline]
    fn create_state<T, C: CapturedContext>() -> Self::Acc<T, C> {
        None
    }
}

impl<T, C: CapturedContext> AccState<T, C> for MostAcc<T, C> {
    type Alloc = Most;

    #[inline]
    fn is_empty(&self) -> bool {
        self.is_none()
    }

    #[inline]
    fn len(&self) -> usize {
        self.as_slice().len()
    }

    #[inline]
    fn iter(&'_ self) -> ContextualIter<'_, T, C> {
        ContextualIter::new(self.as_slice())
    }

    #[inline]
    fn map<U>(self, map: impl FnMut(T) -> U) -> <Self::Alloc as Acc>::Acc<U, C> {
        self.map(|ct| ct.map(map))
    }

    #[inline]
    fn reserve(&mut self, _additional: usize) {}

    #[inline]
    fn push_naive(&mut self, value: Contextual<T, C>) -> bool {
        if self.is_none() {
            *self = Some(value);
            true
        } else {
            false
        }
    }

    #[inline]
    fn append_naive(&mut self, other: Self) -> usize {
        match self {
            Some(_) => other.len(),
            this => {
                *this = other;
                0
            }
        }
    }

    #[inline]
    fn push(&mut self, value: Contextual<T, C>) -> bool
    where
        T: Prioritized,
    {
        match self {
            Some(old) if old.priority() < value.priority() => {
                *old = value;
                false
            }
            Some(_) => false,
            this => {
                *this = Some(value);
                true
            }
        }
    }

    #[inline]
    fn append(&mut self, other: Self) -> usize
    where
        T: Prioritized,
    {
        let Some(other) = other else {
            return 0;
        };

        (!self.push(other)).into()
    }
}
