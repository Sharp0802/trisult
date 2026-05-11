use crate::{CapturedContext, ContextualDiagnosis, MapIter};

/// A trait for types that can map the warning and error components of a diagnostic.
pub trait MapDiagnosis<W, E> {
    /// The resulting type after mapping.
    type Target<UW, UE, FW, FE>
    where
        FW: FnMut(W) -> UW,
        FE: FnMut(E) -> UE;

    /// Maps the warning and error components of this type using the provided functions.
    fn map_diagnosis<UW, UE, FW, FE>(self, fw: FW, fe: FE) -> Self::Target<UW, UE, FW, FE>
    where
        FW: FnMut(W) -> UW,
        FE: FnMut(E) -> UE;

    /// Maps only the error component of this type, leaving the warning component unchanged.
    #[inline]
    fn map_errors<U, F: FnMut(E) -> U>(self, map: F) -> Self::Target<W, U, fn(W) -> W, F>
    where
        Self: Sized,
    {
        self.map_diagnosis(move |warn| warn, map)
    }

    /// Maps only the warning component of this type, leaving the error component unchanged.
    #[inline]
    fn map_warnings<U, F: FnMut(W) -> U>(self, map: F) -> Self::Target<U, E, F, fn(E) -> E>
    where
        Self: Sized,
    {
        self.map_diagnosis(map, move |err| err)
    }
}

impl<T, W, E, C> MapDiagnosis<W, E> for T
where
    T: Iterator<Item = ContextualDiagnosis<W, E, C>>,
    C: CapturedContext,
{
    type Target<UW, UE, FW, FE>
        = MapIter<W, E, UW, UE, C, T, FW, FE>
    where
        FW: FnMut(W) -> UW,
        FE: FnMut(E) -> UE;

    #[inline]
    fn map_diagnosis<UW, UE, FW, FE>(self, fw: FW, fe: FE) -> Self::Target<UW, UE, FW, FE>
    where
        FW: FnMut(W) -> UW,
        FE: FnMut(E) -> UE,
    {
        MapIter::new(self, fw, fe)
    }
}

/// A trait for types that have an inherent accumulation priority.
pub trait Prioritized {
    /// The priority type, used to determine relative precedence.
    type Priority: Ord + PartialOrd;

    /// Returns the priority of this item.
    fn priority(&self) -> Self::Priority;
}
