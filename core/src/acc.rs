use crate::{CapturedContext, Contextual, ContextualIntoIter, ContextualIter, NoLoc, Prioritized};

#[cfg(feature = "alloc")]
use smallvec::SmallVec;

#[cfg(feature = "alloc")]
use crate::VEC_SIZE;

/// Determines how items are accumulated.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AccumulatorKind {
    /// Accumulates all items pushed into it.
    #[cfg(feature = "alloc")]
    All,
    /// Accumulates only a single item (e.g. the first or highest priority).
    Most,
}

/// The internal state of an accumulator.
#[derive(Debug, Clone)]
pub enum AccumulatorState<T, C: CapturedContext = NoLoc> {
    /// Contains all accumulated items.
    #[cfg(feature = "alloc")]
    All(SmallVec<Contextual<T, C>, VEC_SIZE>),
    /// Contains at most a single item.
    Most(Option<Contextual<T, C>>),
}

impl<T, C: CapturedContext> AccumulatorState<T, C> {
    /// Creates a new, empty accumulator state of the specified kind.
    #[inline]
    pub const fn new(kind: AccumulatorKind) -> Self {
        match kind {
            #[cfg(feature = "alloc")]
            AccumulatorKind::All => Self::All(SmallVec::new()),
            AccumulatorKind::Most => Self::Most(None),
        }
    }

    /// Returns the kind of this accumulator.
    #[inline]
    pub const fn kind(&self) -> AccumulatorKind {
        match self {
            #[cfg(feature = "alloc")]
            Self::All(_) => AccumulatorKind::All,
            Self::Most(_) => AccumulatorKind::Most,
        }
    }

    /// Returns `true` if the accumulator contains no items.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        match self {
            #[cfg(feature = "alloc")]
            Self::All(vec) => vec.is_empty(),
            Self::Most(option) => option.is_none(),
        }
    }

    /// Returns the number of items currently in the accumulator.
    #[inline]
    pub const fn len(&self) -> usize {
        match self {
            #[cfg(feature = "alloc")]
            Self::All(vec) => vec.len(),
            Self::Most(option) => {
                if option.is_some() {
                    1
                } else {
                    0
                }
            }
        }
    }

    /// Returns an iterator over the accumulated contextual items.
    #[inline]
    pub const fn iter(&'_ self) -> ContextualIter<'_, T, C> {
        ContextualIter::new(self)
    }

    /// Maps the accumulated values using the given closure.
    #[inline]
    pub fn map<U>(self, map: impl FnMut(T) -> U) -> AccumulatorState<U, C> {
        match self {
            #[cfg(feature = "alloc")]
            Self::All(vec) => {
                if vec.is_empty() {
                    return AccumulatorState::All(SmallVec::new());
                }

                let mut map = map;
                AccumulatorState::All(vec.into_iter().map(|ct| ct.map(&mut map)).collect())
            }
            Self::Most(option) => AccumulatorState::Most(option.map(|ct| ct.map(map))),
        }
    }

    /// Reserves capacity for at least `additional` more elements to be inserted.
    #[inline]
    #[cfg(feature = "alloc")]
    pub fn reserve(&mut self, additional: usize) {
        if let Self::All(vec) = self {
            vec.reserve(additional);
        }
    }

    /// Pushes a value into the accumulator without checking priorities.
    /// Returns `true` if the item was added, or `false` if it was ignored
    /// (e.g., when pushing to an already-occupied `Most` state).
    #[inline]
    pub fn push_naive(&mut self, value: Contextual<T, C>) -> bool {
        match self {
            #[cfg(feature = "alloc")]
            Self::All(vec) => {
                vec.push(value);
                true
            }

            Self::Most(option) if option.is_none() => {
                *option = Some(value);
                true
            }

            _ => false,
        }
    }

    /// Appends the contents of another state into this one naively (ignoring priorities).
    /// Returns the number of items that were ignored.
    #[inline]
    pub fn append_naive(&mut self, other: Self) -> usize {
        if other.is_empty() {
            return 0;
        }
        
        match (self, other) {
            #[cfg(feature = "alloc")]
            (Self::All(vec), Self::All(mut other)) => {
                vec.append(&mut other);
                0
            }

            #[cfg(feature = "alloc")]
            (Self::All(vec), Self::Most(option)) => {
                vec.extend(option);
                0
            }

            (Self::Most(Some(_)), other) => other.len(),

            (Self::Most(this), other) if !other.is_empty() => {
                let len = other.len();
                *this = Some(other.into_iter().next().unwrap_or_else(|| unreachable!()));
                len - 1
            }

            (Self::Most(_), _) => 0,
        }
    }
}

impl<T: Prioritized, C: CapturedContext> AccumulatorState<T, C> {
    /// Pushes a value into the accumulator, respecting item priorities.
    /// In a `Most` state, an item will overwrite the existing item if it has a strictly higher priority.
    /// Returns `true` if the item was stored, `false` otherwise.
    #[inline]
    pub fn push(&mut self, value: Contextual<T, C>) -> bool {
        match self {
            #[cfg(feature = "alloc")]
            Self::All(vec) => {
                vec.push(value);
                true
            }

            Self::Most(Some(old)) if old.priority() < value.priority() => {
                *old = value;
                true
            }

            Self::Most(option) if option.is_none() => {
                *option = Some(value);
                true
            }

            _ => false,
        }
    }

    /// Appends the contents of another state into this one, respecting priorities.
    /// Returns the number of items that were ignored.
    #[inline]
    pub fn append(&mut self, other: Self) -> usize {
        if other.is_empty() {
            return 0;
        }
        
        match (self, other) {
            #[cfg(feature = "alloc")]
            (Self::All(vec), Self::All(mut other_vec)) => {
                vec.append(&mut other_vec);
                0
            }

            #[cfg(feature = "alloc")]
            (Self::All(vec), Self::Most(option)) => {
                vec.extend(option);
                0
            }

            (this, other) => {
                let mut count: usize = 0;
                for item in other {
                    if !this.push(item) {
                        count += 1;
                    }
                }

                count
            }
        }
    }
}

impl<T, C: CapturedContext> IntoIterator for AccumulatorState<T, C> {
    type Item = Contextual<T, C>;
    type IntoIter = ContextualIntoIter<T, C>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.into()
    }
}

impl<'a, T, C: CapturedContext> IntoIterator for &'a AccumulatorState<T, C> {
    type Item = Contextual<&'a T, &'a C>;
    type IntoIter = ContextualIter<'a, T, C>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
