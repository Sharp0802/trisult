use core::fmt::{Debug, Display, Formatter};

/// A marker trait for types that can represent a captured context (such as a source location).
pub trait CapturedContext: Debug + Display + Clone {}

impl<T: Debug + Display + Clone> CapturedContext for T {}

/// A trait for types that act as a stack or source of context, capable of capturing the current context state.
pub trait ContextStack {
    /// The type of context that is captured from this stack.
    type Captured: CapturedContext;
}

/// Extends [`ContextStack`] to allow pushing new segments onto the stack to form deeper, nested contexts.
pub trait ContextStackMut: ContextStack {
    /// The segment type that can be pushed onto this context stack.
    type Segment;

    /// Captures the current state of the context stack.
    fn capture(&self) -> Self::Captured;

    /// Pushes a new segment onto the context stack, returning the updated stack.
    #[must_use]
    fn push(&mut self, segment: Self::Segment) -> Self;
}

/// A zero-sized type representing the absence of contextual location information.
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
