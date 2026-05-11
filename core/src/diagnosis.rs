use crate::{CapturedContext, Contextual, Contextuals, MapDiagnosis, NoLoc, Prioritized};
use core::error::Error;
use core::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub enum Diagnosis<W, E> {
    Warning(W),
    Error(E),
}

impl<W, E> Diagnosis<W, E> {
    #[inline]
    pub const fn as_error(&self) -> Option<&E> {
        match self {
            Self::Warning(_) => None,
            Self::Error(error) => Some(error),
        }
    }

    #[inline]
    pub const fn as_warning(&self) -> Option<&W> {
        match self {
            Self::Warning(warn) => Some(warn),
            Self::Error(_) => None,
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

pub type ContextualDiagnosis<W, E, C> = Contextual<Diagnosis<W, E>, C>;

impl<W, E, C: CapturedContext> ContextualDiagnosis<W, E, C> {
    #[inline]
    pub const fn as_warning(&self) -> Option<Contextual<&W, &C>> {
        match &self.value {
            Diagnosis::Warning(value) => Some(Contextual::new(&self.context, value)),
            Diagnosis::Error(_) => None,
        }
    }

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

pub type Diagnoses<W, E, C> = Contextuals<Diagnosis<W, E>, C>;

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

#[derive(Debug, Clone)]
pub struct Diagnosed<T, W, C: CapturedContext = NoLoc>(pub T, pub Contextuals<W, C>);
