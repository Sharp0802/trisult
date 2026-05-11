use crate::{
    AccumulatorKind, AccumulatorState, CapturedContext, ContextualIntoIter, ContextualIter, NoLoc,
    Prioritized,
};
use core::error::Error;
use core::fmt::{Display, Formatter};

/// A value paired with the context in which it was produced.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Contextual<T, C: CapturedContext = NoLoc> {
    /// The context associated with the value.
    pub context: C,
    /// The inner value.
    pub value: T,
}

impl<T, C: CapturedContext> Contextual<T, C> {
    /// Creates a new `Contextual` with the given context and value.
    #[inline]
    pub const fn new(context: C, value: T) -> Self {
        Self { context, value }
    }

    /// Maps the inner value to a new type using the provided closure, preserving the context.
    #[inline]
    pub fn map<U, F>(self, mut map: F) -> Contextual<U, C>
    where
        F: FnMut(T) -> U,
    {
        Contextual {
            context: self.context,
            value: map(self.value),
        }
    }

    /// Returns a new `Contextual` holding references to the inner context and value.
    #[inline]
    pub const fn as_ref(&self) -> Contextual<&T, &C> {
        Contextual {
            context: &self.context,
            value: &self.value,
        }
    }
}

impl<T: Prioritized, C: CapturedContext> Prioritized for Contextual<T, C> {
    type Priority = T::Priority;

    #[inline]
    fn priority(&self) -> Self::Priority {
        self.value.priority()
    }
}

impl<T: Display, C: CapturedContext> Display for Contextual<T, C> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: {}", self.context, self.value)
    }
}

impl<T: Error + 'static, C: CapturedContext> Error for Contextual<T, C> {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.value.source()
    }
}

/// An accumulator that collects `Contextual` items based on a specific `AccumulatorKind`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Contextuals<T, C: CapturedContext = NoLoc> {
    state: AccumulatorState<T, C>,
    ignored: usize,
}

impl<T, C: CapturedContext> Contextuals<T, C> {
    /// Creates a new, empty accumulator of the given kind.
    #[inline]
    #[must_use]
    pub const fn new(kind: AccumulatorKind) -> Self {
        Self {
            state: AccumulatorState::new(kind),
            ignored: 0,
        }
    }

    /// Returns the kind of accumulator this represents.
    #[inline]
    pub const fn kind(&self) -> AccumulatorKind {
        self.state.kind()
    }

    /// Returns `true` if the accumulator contains no items.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.state.is_empty()
    }

    /// Returns an iterator over references to the accumulated contextual items.
    #[inline]
    pub const fn iter(&'_ self) -> ContextualIter<'_, T, C> {
        self.state.iter()
    }

    /// Maps the accumulated values using the given closure.
    #[inline]
    pub fn map<U>(self, map: impl FnMut(T) -> U) -> Contextuals<U, C> {
        Contextuals {
            state: self.state.map(map),
            ignored: self.ignored,
        }
    }

    /// Appends another accumulator's contents into this one, ignoring item priorities.
    #[inline]
    pub fn append_naive(&mut self, other: Self) {
        self.ignored += self.state.append_naive(other.state) + other.ignored;
    }

    /// Pushes a value into the accumulator without checking priorities.
    #[inline]
    pub fn push_naive(&mut self, value: Contextual<T, C>) {
        if !self.state.push_naive(value) {
            self.ignored += 1;
        }
    }
}

impl<T: Prioritized, C: CapturedContext> Contextuals<T, C> {
    /// Appends another accumulator's contents into this one, respecting item priorities.
    #[inline]
    pub fn append(&mut self, other: Self) {
        let ignored = self.state.append(other.state);
        self.ignored += ignored + other.ignored;
    }

    /// Pushes a value into the accumulator, respecting item priorities.
    #[inline]
    pub fn push(&mut self, value: Contextual<T, C>) {
        if !self.state.push(value) {
            self.ignored += 1;
        }
    }
}

impl<T: Prioritized, C: CapturedContext> Extend<Contextual<T, C>> for Contextuals<T, C> {
    #[inline]
    fn extend<I: IntoIterator<Item = Contextual<T, C>>>(&mut self, iter: I) {
        let iter = iter.into_iter();

        #[cfg(feature = "alloc")]
        self.state.reserve(iter.size_hint().0);

        for item in iter {
            if !self.state.push(item) {
                self.ignored += 1;
            }
        }
    }
}

impl<T, C: CapturedContext> IntoIterator for Contextuals<T, C> {
    type Item = Contextual<T, C>;
    type IntoIter = ContextualIntoIter<T, C>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.state.into_iter()
    }
}

impl<'a, T, C: CapturedContext> IntoIterator for &'a Contextuals<T, C> {
    type Item = Contextual<&'a T, &'a C>;
    type IntoIter = ContextualIter<'a, T, C>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.state.iter()
    }
}

impl<T: Display, C: CapturedContext> Display for Contextuals<T, C> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        for (i, item) in self.iter().enumerate() {
            if i != 0 {
                writeln!(f)?;
            }

            write!(f, "{item}")?;
        }

        if self.ignored > 0 {
            if !self.is_empty() {
                writeln!(f)?;
            }

            write!(f, "... {} ignored", self.ignored)?;
        }

        Ok(())
    }
}

impl<T: Error, C: CapturedContext> Error for Contextuals<T, C> {
    // NOTE: fn source() cannot be implemented;
    //       An array of impl Error cannot be implicitly cast into dyn Error.
}
