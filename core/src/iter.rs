use crate::{AccumulatorState, CapturedContext, Contextual, ContextualDiagnosis, Diagnosis, NoLoc};

#[cfg(feature = "alloc")]
use crate::VEC_SIZE;

/// An iterator that maps the warning and error components of a stream of `ContextualDiagnosis` items.
pub struct MapIter<W, E, UW, UE, C, I, FW, FE>
where
    C: CapturedContext,
    I: Iterator<Item = ContextualDiagnosis<W, E, C>>,
    FW: FnMut(W) -> UW,
    FE: FnMut(E) -> UE,
{
    iter: I,
    fw: FW,
    fe: FE,
}

impl<W, E, UW, UE, C, I, FW, FE> MapIter<W, E, UW, UE, C, I, FW, FE>
where
    C: CapturedContext,
    I: Iterator<Item = ContextualDiagnosis<W, E, C>>,
    FW: FnMut(W) -> UW,
    FE: FnMut(E) -> UE,
{
    /// Creates a new `MapIter`.
    #[inline]
    pub const fn new(iter: I, fw: FW, fe: FE) -> Self {
        Self { iter, fw, fe }
    }
}

impl<W, E, UW, UE, C, I, FW, FE> Iterator for MapIter<W, E, UW, UE, C, I, FW, FE>
where
    C: CapturedContext,
    I: Iterator<Item = ContextualDiagnosis<W, E, C>>,
    FW: FnMut(W) -> UW,
    FE: FnMut(E) -> UE,
{
    type Item = ContextualDiagnosis<UW, UE, C>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let diagnosis = self.iter.next()?;

        Some(ContextualDiagnosis::new(
            diagnosis.context,
            match diagnosis.value {
                Diagnosis::Warning(value) => Diagnosis::Warning((self.fw)(value)),
                Diagnosis::Error(value) => Diagnosis::Error((self.fe)(value)),
            },
        ))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<W, E, UW, UE, C, I, FW, FE> ExactSizeIterator for MapIter<W, E, UW, UE, C, I, FW, FE>
where
    C: CapturedContext,
    I: ExactSizeIterator<Item = ContextualDiagnosis<W, E, C>>,
    FW: FnMut(W) -> UW,
    FE: FnMut(E) -> UE,
{
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

/// An iterator over references to items within an `AccumulatorState`.
#[derive(Debug)]
#[non_exhaustive]
pub struct ContextualIter<'a, T, C: CapturedContext = NoLoc> {
    source: &'a AccumulatorState<T, C>,
    index: usize,
}

impl<'a, T, C: CapturedContext> ContextualIter<'a, T, C> {
    #[inline]
    pub(crate) const fn new(source: &'a AccumulatorState<T, C>) -> Self {
        Self { source, index: 0 }
    }
}

impl<'a, T, C: CapturedContext> Iterator for ContextualIter<'a, T, C> {
    type Item = Contextual<&'a T, &'a C>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let value = match &self.source {
            #[cfg(feature = "alloc")]
            AccumulatorState::All(vec) => vec.get(self.index).map(|contextual| contextual.as_ref()),
            AccumulatorState::Most(Some(value)) if self.index == 0 => Some(value.as_ref()),
            _ => None,
        };

        self.index += 1;

        value
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<T, C: CapturedContext> ExactSizeIterator for ContextualIter<'_, T, C> {
    #[inline]
    fn len(&self) -> usize {
        self.source.len() - self.index
    }
}

/// An iterator that consumes an `AccumulatorState` and yields its `Contextual` items.
#[derive(Debug)]
#[non_exhaustive]
pub enum ContextualIntoIter<T, C: CapturedContext = NoLoc> {
    /// Iterates over all values stored in the `All` variant.
    #[cfg(feature = "alloc")]
    All(smallvec::IntoIter<Contextual<T, C>, VEC_SIZE>),
    /// Yields at most one value from the `Most` variant.
    Most(Option<Contextual<T, C>>),
}

impl<T, C: CapturedContext> From<AccumulatorState<T, C>> for ContextualIntoIter<T, C> {
    #[inline]
    fn from(value: AccumulatorState<T, C>) -> Self {
        match value {
            #[cfg(feature = "alloc")]
            AccumulatorState::All(vec) => Self::All(vec.into_iter()),
            AccumulatorState::Most(option) => Self::Most(option),
        }
    }
}

impl<T, C: CapturedContext> Iterator for ContextualIntoIter<T, C> {
    type Item = Contextual<T, C>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            #[cfg(feature = "alloc")]
            Self::All(iter) => iter.next(),
            Self::Most(option) => option.take(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<T, C: CapturedContext> ExactSizeIterator for ContextualIntoIter<T, C> {
    #[inline]
    fn len(&self) -> usize {
        match self {
            #[cfg(feature = "alloc")]
            Self::All(vec) => vec.len(),
            Self::Most(Some(_)) => 1,
            Self::Most(None) => 0,
        }
    }
}
