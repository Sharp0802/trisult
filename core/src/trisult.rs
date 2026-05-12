use crate::{CapturedContext, Diagnosed, Diagnoses, Diagnosis, MapDiagnosis, NoLoc};
use core::fmt::Debug;

/// The core result type of the library, designed to accumulate multiple issues rather than
/// short-circuiting on the first failure.
///
/// A `Trisult` evaluates to either:
/// - `Ok(Diagnosed)`: Containing a successful value and potentially some non-fatal warnings.
/// - `Err(Diagnoses)`: Containing accumulated failures (both errors and warnings).
#[derive(Debug, Clone)]
#[must_use]
// NOTE: Not aliasing Result is intended; Trisult MUST be accumulated, NOT be fast failed.
pub enum Trisult<T, W, E, C: CapturedContext = NoLoc> {
    /// Represents a success, paired with any warnings that occurred during execution.
    Ok(Diagnosed<T, W, C>),
    /// Represents a failure, paired with all accumulated diagnostics (errors and warnings).
    Err(Diagnoses<W, E, C>),
}

impl<T, W, E, C: CapturedContext> Trisult<T, W, E, C> {
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
    pub fn err(self) -> Option<Diagnoses<W, E, C>> {
        if let Self::Err(diags) = self {
            Some(diags)
        } else {
            None
        }
    }

    /// Converts from `Trisult<T, W, E, C>` to `Option<Diagnosed<T, W, C>>`.
    /// Returns the `Ok` value, consuming the `self` value, or `None` if it was an `Err`.
    #[inline]
    pub fn ok(self) -> Option<Diagnosed<T, W, C>> {
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
    pub fn and_then<U, F>(self, then: F) -> Trisult<U, W, E, C>
    where
        F: FnOnce(T) -> Trisult<U, W, E, C>,
    {
        match self {
            Self::Ok(Diagnosed(value, mut diags)) => {
                match then(value) {
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
                }
            },

            Self::Err(diags) => Trisult::Err(diags),
        }
    }

    /// Maps a `Trisult<T, W, E, C>` to `Trisult<U, W, E, C>` by applying a function to a
    /// contained `Ok` success value, leaving an `Err` value untouched.
    #[inline]
    pub fn map<U, F>(self, map: F) -> Trisult<U, W, E, C>
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
    pub fn unpack(self) -> (Option<T>, Diagnoses<W, E, C>) {
        match self {
            Self::Ok(Diagnosed(value, diags)) => (Some(value), diags.map(Diagnosis::Warning)),
            Self::Err(value) => (None, value),
        }
    }
}

impl<T, W, E, C: CapturedContext> MapDiagnosis<W, E> for Trisult<T, W, E, C> {
    type Target<UW, UE, FW, FE>
        = Trisult<T, UW, UE, C>
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
            Self::Err(err) => Trisult::Err(err.map_diagnosis(fw, fe)),
        }
    }
}

impl<T, W, E, C: CapturedContext> From<Trisult<T, W, E, C>>
    for Result<Diagnosed<T, W, C>, Diagnoses<W, E, C>>
{
    #[inline]
    fn from(val: Trisult<T, W, E, C>) -> Self {
        match val {
            Trisult::Ok(ok) => Ok(ok),
            Trisult::Err(err) => Err(err),
        }
    }
}

impl<T, W: Debug, E: Debug, C: CapturedContext> Trisult<T, W, E, C> {
    /// Returns the contained [`Diagnosed`] value, consuming the `self` value.
    ///
    /// # Panics
    ///
    /// Panics if the value is an `Err`, with a panic message including the
    /// passed message, and the content of the `Err`.
    #[allow(clippy::missing_panics_doc)]
    #[inline]
    #[track_caller]
    pub fn expect(self, msg: &str) -> Diagnosed<T, W, C> {
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
    pub fn unwrap(self) -> Diagnosed<T, W, C> {
        self.expect("called `Trisult::unwrap()` on an `Err` value")
    }
}
