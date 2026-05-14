use crate::{
    AccAlloc, Accumulator, CapturedContext, ContextualIter, DefaultAcc, NoLoc, Prioritized,
};
use core::error::Error;
use core::fmt::{Debug, Display, Formatter};
use core::marker::PhantomData;

/// A value paired with the context in which it was produced.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Contextual<T, C = NoLoc>
where
    C: CapturedContext,
{
    /// The context associated with the value.
    pub context: C,
    /// The inner value.
    pub value: T,
}

impl<T, C> Contextual<T, C>
where
    C: CapturedContext,
{
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

impl<T, C> Prioritized for Contextual<T, C>
where
    T: Prioritized,
    C: CapturedContext,
{
    type Priority = T::Priority;

    #[inline]
    fn priority(&self) -> Self::Priority {
        self.value.priority()
    }
}

impl<T, C> Display for Contextual<T, C>
where
    T: Display,
    C: CapturedContext,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: {}", self.context, self.value)
    }
}

impl<T, C> Error for Contextual<T, C>
where
    T: Error + 'static,
    C: CapturedContext,
{
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.value.source()
    }
}

/// An contextual accumulator that collects `Contextual` items based on a specific `Accumulator`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Contextuals<T, C = NoLoc, A = DefaultAcc<T, C>>
where
    C: CapturedContext,
    A: Accumulator<T, C>,
{
    state: A,
    ignored: usize,
    _dummy: PhantomData<(T, C)>,
}

impl<T, C, A> Contextuals<T, C, A>
where
    C: CapturedContext,
    A: Accumulator<T, C>,
{
    /// Creates a new, empty accumulator with the given state.
    #[inline]
    #[must_use]
    pub const fn new(state: A) -> Self {
        Self {
            state,
            ignored: 0,
            _dummy: PhantomData,
        }
    }
}

impl<T, C, A> Contextuals<T, C, A>
where
    C: CapturedContext,
    A: Accumulator<T, C>,
{
    /// Returns `true` if the accumulator contains no items.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.state.is_empty()
    }

    /// Returns an iterator over the accumulated contextual items.
    #[inline]
    pub fn iter(&'_ self) -> ContextualIter<'_, T, C> {
        self.state.iter()
    }

    /// Maps the accumulated values using the given closure.
    #[inline]
    pub fn map<U>(
        self,
        map: impl FnMut(T) -> U,
    ) -> Contextuals<U, C, <A::Alloc as AccAlloc>::Acc<U, C>> {
        Contextuals {
            state: self.state.map(map),
            ignored: self.ignored,
            _dummy: PhantomData,
        }
    }

    /// Reserves capacity for at least `additional` more elements to be inserted.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.state.reserve(additional);
    }

    /// Pushes a value into the accumulator without checking priorities.
    #[inline]
    pub fn push_naive(&mut self, value: Contextual<T, C>) {
        if !self.state.push_naive(value) {
            self.ignored += 1;
        }
    }

    /// Appends the contents of another state into this one naively (ignoring priorities).
    #[inline]
    pub fn append_naive(&mut self, other: Self) {
        self.ignored += self.state.append_naive(other.state) + other.ignored;
    }

    /// Pushes a value into the accumulator, respecting item priorities.
    /// In a `Most` state, an item will overwrite the existing item if it has a strictly higher priority.
    #[inline]
    pub fn push(&mut self, value: Contextual<T, C>)
    where
        T: Prioritized,
    {
        if !self.state.push(value) {
            self.ignored += 1;
        }
    }

    /// Appends the contents of another state into this one, respecting priorities.
    #[inline]
    pub fn append(&mut self, other: Self)
    where
        T: Prioritized,
    {
        let ignored = self.state.append(other.state);
        self.ignored += ignored + other.ignored;
    }
}

impl<T, C, A> Extend<Contextual<T, C>> for Contextuals<T, C, A>
where
    T: Prioritized,
    C: CapturedContext,
    A: Accumulator<T, C>,
{
    #[inline]
    fn extend<I: IntoIterator<Item = Contextual<T, C>>>(&mut self, iter: I) {
        let iter = iter.into_iter();

        self.state.reserve(iter.size_hint().0);

        for item in iter {
            if !self.state.push(item) {
                self.ignored += 1;
            }
        }
    }
}

impl<T, C, A> IntoIterator for Contextuals<T, C, A>
where
    C: CapturedContext,
    A: Accumulator<T, C>,
{
    type Item = Contextual<T, C>;
    type IntoIter = <A as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.state.into_iter()
    }
}

impl<'a, T, C, A> IntoIterator for &'a Contextuals<T, C, A>
where
    C: CapturedContext,
    A: Accumulator<T, C>,
{
    type Item = Contextual<&'a T, &'a C>;
    type IntoIter = ContextualIter<'a, T, C>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.state.iter()
    }
}

impl<T, C, A> Display for Contextuals<T, C, A>
where
    T: Display,
    C: CapturedContext,
    A: Accumulator<T, C>,
{
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

impl<T, C, A> Error for Contextuals<T, C, A>
where
    T: Error,
    C: CapturedContext,
    A: Accumulator<T, C> + Debug + Display,
{
    // NOTE: fn source() cannot be implemented;
    //       An array of impl Error cannot be implicitly cast into dyn Error.
}
