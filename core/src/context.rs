use core::fmt::{Debug, Display, Formatter};

pub trait CapturedContext: Debug + Display + Clone {}

impl<T: Debug + Display + Clone> CapturedContext for T {}

pub trait ContextStack {
    type Captured: CapturedContext;
}

pub trait ContextStackMut: ContextStack {
    type Segment;

    fn capture(&self) -> Self::Captured;

    #[must_use]
    fn push(&mut self, segment: Self::Segment) -> Self;
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct NoLoc;

impl Display for NoLoc {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "no-location")
    }
}

impl ContextStack for NoLoc {
    type Captured = Self;
}

impl ContextStackMut for NoLoc {
    type Segment = ();

    #[inline]
    fn capture(&self) -> Self::Captured {
        Self
    }

    #[inline]
    fn push(&mut self, _segment: Self::Segment) -> Self {
        Self
    }
}

impl<'a, T: ContextStack> ContextStack for &'a T {
    type Captured = &'a T::Captured;
}
