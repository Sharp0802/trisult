use crate::{
    AccumulatorState, CapturedContext, Contextual, Contextuals, MapDiagnosis, NoLoc, Prioritized,
};
use core::error::Error;
use core::fmt::{Display, Formatter};

/// The severity level of a diagnosis, determining its accumulation priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Indicates a non-fatal warning.
    Warning,
    /// Indicates a fatal error.
    Error,
}

/// A diagnostic message, representing either a non-fatal warning or a fatal error.
#[derive(Debug, Clone)]
pub enum Diagnosis<W, E> {
    /// A diagnostic warning.
    Warning(W),
    /// A diagnostic error.
    Error(E),
}

impl<W, E> Diagnosis<W, E> {
    /// Returns a reference to the error value if the diagnosis is an `Error`.
    #[inline]
    pub const fn as_error(&self) -> Option<&E> {
        match self {
            Self::Warning(_) => None,
            Self::Error(error) => Some(error),
        }
    }

    /// Returns a reference to the warning value if the diagnosis is a `Warning`.
    #[inline]
    pub const fn as_warning(&self) -> Option<&W> {
        match self {
            Self::Warning(warn) => Some(warn),
            Self::Error(_) => None,
        }
    }

    /// Returns the error value, consuming `self` value, if the diagnosis is an `Error`.
    #[inline]
    pub fn into_error(self) -> Option<E> {
        match self {
            Self::Error(error) => Some(error),
            Self::Warning(_) => None,
        }
    }

    /// Returns the warning value, consuming `self` value, if the diagnosis is a `Warning`.
    #[inline]
    pub fn into_warning(self) -> Option<W> {
        match self {
            Self::Error(_) => None,
            Self::Warning(warn) => Some(warn),
        }
    }
}

impl<W, E> Prioritized for Diagnosis<W, E> {
    type Priority = Severity;

    #[inline]
    fn priority(&self) -> Self::Priority {
        match self {
            Self::Warning(_) => Severity::Warning,
            Self::Error(_) => Severity::Error,
        }
    }
}

impl<W, E> MapDiagnosis<W, E> for Diagnosis<W, E> {
    type Target<UW, UE, FW, FE>
        = Diagnosis<UW, UE>
    where
        FW: FnMut(W) -> UW,
        FE: FnMut(E) -> UE;

    #[inline]
    fn map_diagnosis<UW, UE, FW, FE>(self, mut fw: FW, mut fe: FE) -> Self::Target<UW, UE, FW, FE>
    where
        FW: FnMut(W) -> UW,
        FE: FnMut(E) -> UE,
    {
        match self {
            Self::Warning(warn) => Diagnosis::Warning(fw(warn)),
            Self::Error(err) => Diagnosis::Error(fe(err)),
        }
    }
}

impl<W: Display, E: Display> Display for Diagnosis<W, E> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Warning(warn) => write!(f, "warning: {warn}"),
            Self::Error(err) => write!(f, "error: {err}"),
        }
    }
}

impl<W: Error + 'static, E: Error + 'static> Error for Diagnosis<W, E> {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match &self {
            Self::Warning(warn) => warn,
            Self::Error(err) => err,
        })
    }
}

/// A convenience alias for a `Contextual` holding a `Diagnosis`.
pub type ContextualDiagnosis<W, E, C> = Contextual<Diagnosis<W, E>, C>;

impl<W, E, C: CapturedContext> ContextualDiagnosis<W, E, C> {
    /// Returns the contained warning, coupled with its context, if it is a `Warning`.
    #[inline]
    pub const fn as_warning(&self) -> Option<Contextual<&W, &C>> {
        match &self.value {
            Diagnosis::Warning(value) => Some(Contextual::new(&self.context, value)),
            Diagnosis::Error(_) => None,
        }
    }

    /// Returns the contained error, coupled with its context, if it is an `Error`.
    #[inline]
    pub const fn as_error(&self) -> Option<Contextual<&E, &C>> {
        match &self.value {
            Diagnosis::Warning(_) => None,
            Diagnosis::Error(value) => Some(Contextual::new(&self.context, value)),
        }
    }
}

impl<W, E, C: CapturedContext> MapDiagnosis<W, E> for ContextualDiagnosis<W, E, C> {
    type Target<UW, UE, FW, FE>
        = ContextualDiagnosis<UW, UE, C>
    where
        FW: FnMut(W) -> UW,
        FE: FnMut(E) -> UE;

    #[inline]
    fn map_diagnosis<UW, UE, FW, FE>(self, fw: FW, fe: FE) -> Self::Target<UW, UE, FW, FE>
    where
        FW: FnMut(W) -> UW,
        FE: FnMut(E) -> UE,
    {
        Contextual::new(self.context, self.value.map_diagnosis(fw, fe))
    }
}

/// A convenience alias for an accumulator of `Diagnosis` items.
pub type Diagnoses<W, E, C> = Contextuals<Diagnosis<W, E>, C>;

impl<W, E, C: CapturedContext> Diagnoses<W, E, C> {
    /// Maps `Diagnosis<W, E>` into `W`.
    ///
    /// ## Panics
    ///
    /// Panics if it contains any error value.
    #[inline]
    pub fn unwrap_as_warnings(self) -> Contextuals<W, C> {
        if self.is_empty() {
            Contextuals::new(AccumulatorState::new(self.kind()))
        } else {
            self.map(|diag| diag.into_warning().unwrap())
        }
    }

    /// Appends warnings with mapping them to Diagnosis.
    #[inline]
    pub fn append_warnings(&mut self, warnings: Contextuals<W, C>) {
        if warnings.is_empty() {
            return;
        }

        self.extend(
            warnings
                .into_iter()
                .map(|diag| diag.map(Diagnosis::Warning)),
        );
    }
}

impl<W, E, C: CapturedContext> MapDiagnosis<W, E> for Diagnoses<W, E, C> {
    type Target<UW, UE, FW, FE>
        = Diagnoses<UW, UE, C>
    where
        FW: FnMut(W) -> UW,
        FE: FnMut(E) -> UE;

    #[inline]
    fn map_diagnosis<UW, UE, FW, FE>(self, mut fw: FW, mut fe: FE) -> Self::Target<UW, UE, FW, FE>
    where
        FW: FnMut(W) -> UW,
        FE: FnMut(E) -> UE,
    {
        self.map(|diagnosis| diagnosis.map_diagnosis(&mut fw, &mut fe))
    }
}

/// A successful value coupled with any accumulated warnings.
#[derive(Debug, Clone)]
pub struct Diagnosed<T, W, C: CapturedContext = NoLoc>(
    /// The successful value.
    pub T,
    /// Accumulated warnings that occurred during execution.
    pub Contextuals<W, C>,
);
