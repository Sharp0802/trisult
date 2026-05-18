use crate::{CapturedContext, Contextual, ContextualDiagnosis, Diagnosis};

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

/// An iterator over references to items within an `AccState`.
#[derive(Debug)]
#[non_exhaustive]
pub struct ContextualIter<'a, T, C> {
    source: &'a [Contextual<T, C>],
    index: usize,
}

impl<'a, T, C> ContextualIter<'a, T, C> {
    #[inline]
    pub(crate) const fn new(source: &'a [Contextual<T, C>]) -> Self {
        Self { source, index: 0 }
    }
}

impl<'a, T, C> Iterator for ContextualIter<'a, T, C> {
    type Item = Contextual<&'a T, &'a C>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let value = self.source.get(self.index)?;
        self.index += 1;
        Some(value.as_ref())
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<T, C> ExactSizeIterator for ContextualIter<'_, T, C> {
    #[inline]
    fn len(&self) -> usize {
        self.source.len() - self.index
    }
}
