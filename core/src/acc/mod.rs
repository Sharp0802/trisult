#[cfg(feature = "alloc")]
mod all;
mod most;

#[cfg(feature = "alloc")]
pub use all::{All, AllState};
pub use most::{Most, MostAcc};

use crate::{CapturedContext, Contextual, ContextualIter, Prioritized};

#[cfg(feature = "alloc")]
type DefaultImpl = all::All;
#[cfg(not(feature = "alloc"))]
type DefaultImpl = most::Most;

/// A default accumulators.
///
/// Default type is [`trisult::AllState`] with `alloc` feature,
/// Otherwise, default type is [`trisult::MostState`].
pub type DefaultAcc<T, C> = <DefaultImpl as AccAlloc>::Acc<T, C>;

/// A default allocator for accumulators.
///
/// Default type is [`trisult::All`] with `alloc` feature,
/// Otherwise, default type is [`trisult::Most`].
pub type Default = DefaultImpl;

/// A trait for statically passing accumulator policy.
pub trait AccAlloc {
    /// An accumulator type to allocate.
    type Acc<T, C: CapturedContext>: Accumulator<T, C, Alloc = Self>;

    /// Create new, empty accumulator state for given types.
    fn create_state<T, C: CapturedContext>() -> Self::Acc<T, C>;
}

/// The internal state of an accumulator.
pub trait Accumulator<T, C: CapturedContext>: IntoIterator<Item = Contextual<T, C>> {
    /// An allocator used to allocate this accumulator.
    type Alloc: AccAlloc<Acc<T, C> = Self>;

    /// Returns `true` if the accumulator contains no items.
    fn is_empty(&self) -> bool;

    /// Returns the number of item in the accumulator.
    fn len(&self) -> usize;

    /// Returns an iterator over the accumulated contextual items.
    fn iter(&'_ self) -> ContextualIter<'_, T, C>;

    /// Maps the accumulated values using the given closure.
    fn map<U>(self, map: impl FnMut(T) -> U) -> <Self::Alloc as AccAlloc>::Acc<U, C>;

    /// Reserves capacity for at least `additional` more elements to be inserted.
    fn reserve(&mut self, additional: usize);

    /// Pushes a value into the accumulator without checking priorities.
    /// Returns `true` if the item was added, or `false` if it was ignored
    /// (e.g., when pushing to an already-occupied `Most` state).
    fn push_naive(&mut self, value: Contextual<T, C>) -> bool;

    /// Appends the contents of another state into this one naively (ignoring priorities).
    /// Returns the number of items that were ignored.
    fn append_naive(&mut self, other: Self) -> usize;

    /// Pushes a value into the accumulator, respecting item priorities.
    /// In a `Most` state, an item will overwrite the existing item if it has a strictly higher priority.
    /// Returns `true` if the item was stored, `false` otherwise.
    fn push(&mut self, value: Contextual<T, C>) -> bool
    where
        T: Prioritized;

    /// Appends the contents of another state into this one, respecting priorities.
    /// Returns the number of items that were ignored.
    fn append(&mut self, other: Self) -> usize
    where
        T: Prioritized;
}

/// Defines a custom trisult type.
///
/// ## Examples
///
/// You can alias `Trisult<T, MyWarn, MyErr, NoLoc, A = Default>` as:
///
/// ```rust
/// use trisult::{custom_trisult, NoLoc};
///
/// #[derive(Debug)]
/// pub enum MyWarn { Deprecated, Unconventional }
///
/// #[derive(Debug)]
/// pub enum MyErr { MissingField, InvalidFormat }
///
/// custom_trisult!(MyTrisult1<T>(MyWarn, MyErr));
/// custom_trisult!(MyTrisult2<T>(MyWarn, MyErr, NoLoc)); // To inject your custom context type
/// custom_trisult!(MyTrisult3<'a, T, E = MyErr>(&'a str, E)); // Also, arbitary generics can be used
/// ```
#[macro_export]
macro_rules! custom_trisult {
    (@last $f:tt $(, $g:tt = $val:tt )*) => { $f };
    (@last $f:tt $(, $g:tt $(= $val:tt)? )*) => { custom_trisult!(@last $($g $(= $val)?),*) };

    ($name:ident< $($tt:tt $(= $val:tt)?),+ >($warn:ty, $err:ty, $ctx:ty)) => {
        #[allow(type_alias_bounds)]
        type $name<$($tt $(= $val)? ,)+ A: ::trisult::AccAlloc = ::trisult::Default> = ::trisult::Trisult<
            custom_trisult!(@last $($tt $(= $val)?),+),
            $warn,
            $err,
            $ctx,
            A::Acc<::trisult::Diagnosis<$warn, $err>, $ctx>,
        >;
    };

    ($name:ident< $($tt:tt $(= $val:tt)?),+ >($warn:ty, $err:ty)) => {
        ::trisult::custom_trisult!{ $name < $($tt $(= $val)?),+ > ($warn, $err, ::trisult::NoLoc) }
    };
}
