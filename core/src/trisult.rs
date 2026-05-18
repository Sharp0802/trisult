use crate::{
    Acc, AccState, CapturedContext, DefaultAcc, Diagnosed, Diagnoses, Diagnosis,
    MapDiagnosis, NoLoc,
};
use core::fmt::Debug;

/// The core result type of the library, designed to accumulate multiple issues rather than
/// short-circuiting on the first failure.
///
/// A `Trisult` evaluates to either:
/// - `Ok(Diagnosed)`: Containing a successful value and potentially some non-fatal warnings.
/// - `Err(Diagnoses)`: Containing accumulated failures (both errors and warnings).
#[must_use]
// NOTE: Not aliasing Result is intended; Trisult MUST be accumulated, NOT be fast failed.
pub enum Trisult<
    T,
    W,
    E,
    C: CapturedContext = NoLoc,
    A: AccState<Diagnosis<W, E>, C> = DefaultAcc<Diagnosis<W, E>, C>,
> {
    /// Represents a success, paired with any warnings that occurred during execution.
    Ok(Diagnosed<T, W, C, <A::Alloc as Acc>::Acc<W, C>>),
    /// Represents a failure, paired with all accumulated diagnostics (errors and warnings).
    Err(Diagnoses<W, E, C, A>),
}

impl<T, W, E, C: CapturedContext, A: AccState<Diagnosis<W, E>, C>> Trisult<T, W, E, C, A> {
    /// Returns `true` if the trisult is an `Err` value.
    #[inline]
    pub const fn is_err(&self) -> bool {
        matches!(self, Self::Err(..))
    }

    /// Returns `true` if the trisult is an `Ok` value.
    #[inline]
    pub const fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(..))
    }

    /// Converts from `Trisult<T, W, E, C>` to `Option<Diagnoses<W, E, C>>`.
    /// Returns the `Err` value, consuming the `self` value, or `None` if it was an `Ok`.
    #[inline]
    pub fn err(self) -> Option<Diagnoses<W, E, C, A>> {
        if let Self::Err(diags) = self {
            Some(diags)
        } else {
            None
        }
    }

    /// Converts from `Trisult<T, W, E, C, A>` to `Option<Diagnosed<T, W, C, <A::Alloc as AccAlloc>::Acc<W, C>>>`.
    /// Returns the `Ok` value, consuming the `self` value, or `None` if it was an `Err`.
    #[inline]
    pub fn ok(self) -> Option<Diagnosed<T, W, C, <A::Alloc as Acc>::Acc<W, C>>> {
        if let Self::Ok(diagnosed) = self {
            Some(diagnosed)
        } else {
            None
        }
    }

    /// Calls `then` if the trisult is `Ok`, otherwise returns the `Err` value of `self`.
    /// This function accumulates diagnostics, ensuring that warnings from the first step
    /// are not lost during the chain.
    #[inline]
    pub fn and_then<U, F>(self, then: F) -> Trisult<U, W, E, C, A>
    where
        F: FnOnce(T) -> Trisult<U, W, E, C, A>,
    {
        match self {
            Self::Ok(Diagnosed(value, mut diags)) => match then(value) {
                new if diags.is_empty() => new,

                Trisult::Ok(Diagnosed(value, new_diags)) => {
                    diags.append_naive(new_diags);
                    Trisult::Ok(Diagnosed(value, diags))
                }

                Trisult::Err(new_diags) => {
                    let mut diags = diags.map(Diagnosis::Warning);
                    diags.append(new_diags);
                    Trisult::Err(diags)
                }
            },

            Self::Err(diags) => Trisult::Err(diags),
        }
    }

    /// Maps a `Trisult<T, W, E, C, A>` to `Trisult<U, W, E, C, A>` by applying a function to a
    /// contained `Ok` success value, leaving an `Err` value untouched.
    #[inline]
    pub fn map<U, F>(self, map: F) -> Trisult<U, W, E, C, A>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Ok(Diagnosed(value, diags)) => Trisult::Ok(Diagnosed(map(value), diags)),
            Self::Err(diags) => Trisult::Err(diags),
        }
    }

    /// Unpacks the `Trisult` into a tuple consisting of an optional success value and its
    /// associated diagnostics. If the result was `Ok`, the diagnostics will contain only warnings.
    #[inline]
    pub fn unpack(self) -> (Option<T>, Diagnoses<W, E, C, A>) {
        match self {
            Self::Ok(Diagnosed(value, diags)) => (Some(value), diags.map(Diagnosis::Warning)),
            Self::Err(value) => (None, value),
        }
    }

    #[doc(hidden)]
    #[inline(always)]
    #[allow(clippy::inline_always)]
    pub fn __macro_tri_unpack(
        self,
        diags: &mut Diagnoses<W, E, C, A>,
        has_errors: &mut bool,
    ) -> Option<T> {
        match self {
            Self::Ok(Diagnosed(value, warn)) => {
                // NOTE: LLVM fails to inline this call:
                // diags.append_warnings(warn);

                if !warn.is_empty() {
                    diags.extend(warn.into_iter().map(|diag| diag.map(Diagnosis::Warning)));
                }

                Some(value)
            }

            Self::Err(err) => {
                *has_errors = true;
                if diags.is_empty() {
                    *diags = err;
                } else {
                    diags.append(err);
                }

                None
            }
        }
    }
}

impl<T, W, E, C: CapturedContext, A: AccState<Diagnosis<W, E>, C>> MapDiagnosis<W, E>
    for Trisult<T, W, E, C, A>
{
    type Target<UW, UE, FW, FE>
        = Trisult<T, UW, UE, C, <A::Alloc as Acc>::Acc<Diagnosis<UW, UE>, C>>
    where
        FW: FnMut(W) -> UW,
        FE: FnMut(E) -> UE;

    #[inline]
    fn map_diagnosis<UW, UE, FW, FE>(self, fw: FW, fe: FE) -> Self::Target<UW, UE, FW, FE>
    where
        FW: FnMut(W) -> UW,
        FE: FnMut(E) -> UE,
    {
        match self {
            Self::Ok(Diagnosed(value, diags)) => Trisult::Ok(Diagnosed(value, diags.map(fw))),
            Self::Err(err) => Trisult::Err(err.map_diagnosis::<UW, UE, FW, FE>(fw, fe)),
        }
    }
}

impl<T, W, E, C: CapturedContext, A: AccState<Diagnosis<W, E>, C>> From<Trisult<T, W, E, C, A>>
    for Result<Diagnosed<T, W, C, <A::Alloc as Acc>::Acc<W, C>>, Diagnoses<W, E, C, A>>
{
    #[inline]
    fn from(val: Trisult<T, W, E, C, A>) -> Self {
        match val {
            Trisult::Ok(ok) => Ok(ok),
            Trisult::Err(err) => Err(err),
        }
    }
}

impl<T, W: Debug, E: Debug, C: CapturedContext, A: AccState<Diagnosis<W, E>, C> + Debug>
    Trisult<T, W, E, C, A>
{
    /// Returns the contained [`Diagnosed`] value, consuming the `self` value.
    ///
    /// # Panics
    ///
    /// Panics if the value is an `Err`, with a panic message including the
    /// passed message, and the content of the `Err`.
    #[allow(clippy::missing_panics_doc)]
    #[inline]
    #[track_caller]
    pub fn expect(self, msg: &str) -> Diagnosed<T, W, C, <A::Alloc as Acc>::Acc<W, C>> {
        match self {
            Self::Ok(diag) => diag,
            Self::Err(err) => panic!("{msg}: {err:?}"),
        }
    }

    /// Returns the contained [`Diagnosed`] value, consuming the `self` value.
    ///
    /// # Panics
    ///
    /// Panics if the value is an `Err`, with a panic message provided by the
    /// `Err`'s value.
    #[allow(clippy::missing_panics_doc)]
    #[inline]
    #[track_caller]
    pub fn unwrap(self) -> Diagnosed<T, W, C, <A::Alloc as Acc>::Acc<W, C>> {
        self.expect("called `Trisult::unwrap()` on an `Err` value")
    }
}

impl<T, W, E, C, A> Debug for Trisult<T, W, E, C, A>
where
    C: CapturedContext,
    A: AccState<Diagnosis<W, E>, C>,
    Diagnosed<T, W, C, <A::Alloc as Acc>::Acc<W, C>>: Debug,
    Diagnoses<W, E, C, A>: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Ok(diagnosed) => f.debug_tuple("Ok").field(diagnosed).finish(),
            Self::Err(diagnoses) => f.debug_tuple("Err").field(diagnoses).finish(),
        }
    }
}

impl<T, W, E, C, A> Clone for Trisult<T, W, E, C, A>
where
    C: CapturedContext,
    A: AccState<Diagnosis<W, E>, C>,
    Diagnosed<T, W, C, <A::Alloc as Acc>::Acc<W, C>>: Clone,
    Diagnoses<W, E, C, A>: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        match self {
            Self::Ok(diagnosed) => Self::Ok(diagnosed.clone()),
            Self::Err(diagnoses) => Self::Err(diagnoses.clone()),
        }
    }
}
