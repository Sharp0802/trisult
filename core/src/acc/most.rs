use crate::{Acc, AccState, Contextual, ContextualIter, Prioritized};

/// An allocator for an accumulator that collects only a single item (highest priority).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Most;

/// An accumulator that collects only a single item (highest priority).
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MostAcc<T, C>(Option<Contextual<T, C>>);

impl Acc for Most {
    type Acc<T, C> = MostAcc<T, C>;

    #[inline]
    fn create_state<T, C>() -> Self::Acc<T, C> {
        MostAcc(None)
    }
}

impl<T, C> AccState for MostAcc<T, C> {
    type Type = T;
    type Context = C;
    type Alloc = Most;

    #[inline]
    fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    #[inline]
    fn len(&self) -> usize {
        self.0.as_slice().len()
    }

    #[inline]
    fn iter(&'_ self) -> ContextualIter<'_, T, C> {
        ContextualIter::new(self.0.as_slice())
    }

    #[inline]
    fn map<U>(self, map: impl FnMut(T) -> U) -> <Self::Alloc as Acc>::Acc<U, C> {
        MostAcc(self.0.map(|ct| ct.map(map)))
    }

    #[inline]
    fn reserve(&mut self, _additional: usize) {}

    #[inline]
    fn push_naive(&mut self, value: Contextual<T, C>) -> bool {
        if self.0.is_none() {
            self.0 = Some(value);
            true
        } else {
            false
        }
    }

    #[inline]
    fn append_naive(&mut self, other: Self) -> usize {
        match &mut self.0 {
            Some(_) => other.len(),
            this => {
                *this = other.0;
                0
            }
        }
    }

    #[inline]
    fn push(&mut self, value: Contextual<T, C>) -> bool
    where
        T: Prioritized,
    {
        match &mut self.0 {
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
        let Some(other) = other.0 else {
            return 0;
        };

        (!self.push(other)).into()
    }
}

impl<T, C> IntoIterator for MostAcc<T, C> {
    type Item = Contextual<T, C>;
    type IntoIter = core::option::IntoIter<Contextual<T, C>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
