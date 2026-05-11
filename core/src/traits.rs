use crate::{CapturedContext, ContextualDiagnosis, MapIter};

pub trait MapDiagnosis<W, E> {
    type Target<UW, UE, FW, FE>
    where
        FW: FnMut(W) -> UW,
        FE: FnMut(E) -> UE;

    fn map_diagnosis<UW, UE, FW, FE>(self, fw: FW, fe: FE) -> Self::Target<UW, UE, FW, FE>
    where
        FW: FnMut(W) -> UW,
        FE: FnMut(E) -> UE;

    #[inline]
    fn map_errors<U, F: FnMut(E) -> U>(self, map: F) -> Self::Target<W, U, fn(W) -> W, F>
    where
        Self: Sized,
    {
        self.map_diagnosis(move |warn| warn, map)
    }

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

pub trait Prioritized {
    type Priority: Ord + PartialOrd;

    fn priority(&self) -> Self::Priority;
}
