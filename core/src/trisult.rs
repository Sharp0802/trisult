use core::error::Error;
use core::fmt::{Debug, Display, Formatter};

#[cfg(feature = "alloc")]
use smallvec::SmallVec;

#[cfg(feature = "alloc")]
use crate::VEC_SIZE;

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

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Contextual<T, C: CapturedContext = NoLoc> {
    pub context: C,
    pub value: T,
}

impl<T, C: CapturedContext> Contextual<T, C> {
    #[inline]
    pub const fn new(context: C, value: T) -> Self {
        Self { context, value }
    }

    #[inline]
    pub fn map<U, F>(self, mut map: F) -> Contextual<U, C>
    where
        F: FnMut(T) -> U,
    {
        Contextual {
            context: self.context,
            value: map(self.value),
        }
    }

    #[inline]
    pub const fn as_ref(&self) -> Contextual<&T, &C> {
        Contextual {
            context: &self.context,
            value: &self.value,
        }
    }
}

impl<T: Prioritized, C: CapturedContext> Prioritized for Contextual<T, C> {
    type Priority = T::Priority;

    #[inline]
    fn priority(&self) -> Self::Priority {
        self.value.priority()
    }
}

impl<T: Display, C: CapturedContext> Display for Contextual<T, C> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: {}", self.context, self.value)
    }
}

impl<T: Error + 'static, C: CapturedContext> Error for Contextual<T, C> {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.value.source()
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

#[derive(Debug, Clone)]
enum AccumulatorState<T, C: CapturedContext = NoLoc> {
    #[cfg(feature = "alloc")]
    All(SmallVec<Contextual<T, C>, VEC_SIZE>),
    Most(Option<Contextual<T, C>>),
}

impl<T, C: CapturedContext> AccumulatorState<T, C> {
    #[inline]
    pub const fn new(kind: AccumulatorKind) -> Self {
        match kind {
            #[cfg(feature = "alloc")]
            AccumulatorKind::All => Self::All(SmallVec::new()),
            AccumulatorKind::Most => Self::Most(None),
        }
    }

    #[inline]
    pub const fn kind(&self) -> AccumulatorKind {
        match self {
            #[cfg(feature = "alloc")]
            Self::All(_) => AccumulatorKind::All,
            Self::Most(_) => AccumulatorKind::Most,
        }
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        match self {
            #[cfg(feature = "alloc")]
            Self::All(vec) => vec.is_empty(),
            Self::Most(option) => option.is_none(),
        }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        match self {
            #[cfg(feature = "alloc")]
            Self::All(vec) => vec.len(),
            Self::Most(option) => {
                if option.is_some() {
                    1
                } else {
                    0
                }
            }
        }
    }

    #[inline]
    pub const fn iter(&'_ self) -> ContextualIter<'_, T, C> {
        ContextualIter::new(self)
    }

    #[inline]
    pub fn map<U>(self, map: impl FnMut(T) -> U) -> AccumulatorState<U, C> {
        match self {
            #[cfg(feature = "alloc")]
            Self::All(vec) => {
                let mut map = map;
                AccumulatorState::All(vec.into_iter().map(|ct| ct.map(&mut map)).collect())
            }
            Self::Most(option) => AccumulatorState::Most(option.map(|ct| ct.map(map))),
        }
    }

    #[inline]
    #[cfg(feature = "alloc")]
    pub fn reserve(&mut self, additional: usize) {
        if let Self::All(vec) = self {
            vec.reserve(additional);
        }
    }

    #[inline]
    pub fn push_naive(&mut self, value: Contextual<T, C>) -> bool {
        match self {
            #[cfg(feature = "alloc")]
            Self::All(vec) => {
                vec.push(value);
                true
            }

            Self::Most(option) if option.is_none() => {
                *option = Some(value);
                true
            }

            _ => false,
        }
    }

    #[inline]
    pub fn append_naive(&mut self, other: Self) -> usize {
        match (self, other) {
            #[cfg(feature = "alloc")]
            (Self::All(vec), Self::All(mut other)) => {
                vec.append(&mut other);
                0
            }

            #[cfg(feature = "alloc")]
            (Self::All(vec), Self::Most(option)) => {
                vec.extend(option);
                0
            }

            (Self::Most(Some(_)), other) => other.len(),

            (Self::Most(this), other) if !other.is_empty() => {
                let len = other.len();
                *this = Some(other.into_iter().next().unwrap());
                len - 1
            }

            (Self::Most(_), _) => 0,
        }
    }
}

impl<T: Prioritized, C: CapturedContext> AccumulatorState<T, C> {
    #[inline]
    pub fn push(&mut self, value: Contextual<T, C>) -> bool {
        match self {
            #[cfg(feature = "alloc")]
            Self::All(vec) => {
                vec.push(value);
                true
            }

            Self::Most(Some(old)) if old.priority() < value.priority() => {
                *old = value;
                true
            }

            Self::Most(option) if option.is_none() => {
                *option = Some(value);
                true
            }

            _ => false,
        }
    }

    #[inline]
    pub fn append(&mut self, other: Self) -> usize {
        match (self, other) {
            #[cfg(feature = "alloc")]
            (Self::All(vec), Self::All(mut other_vec)) => {
                vec.append(&mut other_vec);
                0
            }

            #[cfg(feature = "alloc")]
            (Self::All(vec), Self::Most(option)) => {
                vec.extend(option);
                0
            }

            (this, other) => {
                let mut count: usize = 0;
                for item in other {
                    if !this.push(item) {
                        count += 1;
                    }
                }

                count
            }
        }
    }
}

impl<T, C: CapturedContext> IntoIterator for AccumulatorState<T, C> {
    type Item = Contextual<T, C>;
    type IntoIter = ContextualIntoIter<T, C>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.into()
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct ContextualIter<'a, T, C: CapturedContext = NoLoc> {
    source: &'a AccumulatorState<T, C>,
    index: usize,
}

impl<'a, T, C: CapturedContext> ContextualIter<'a, T, C> {
    #[inline]
    const fn new(source: &'a AccumulatorState<T, C>) -> Self {
        Self { source, index: 0 }
    }
}

impl<'a, T, C: CapturedContext> Iterator for ContextualIter<'a, T, C> {
    type Item = Contextual<&'a T, &'a C>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let value = match &self.source {
            #[cfg(feature = "alloc")]
            AccumulatorState::All(vec) => vec.get(self.index).map(|contextual| contextual.as_ref()),
            AccumulatorState::Most(Some(value)) if self.index == 0 => Some(value.as_ref()),
            _ => None,
        };

        self.index += 1;

        value
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ContextualIntoIter<T, C: CapturedContext = NoLoc> {
    #[cfg(feature = "alloc")]
    All(smallvec::IntoIter<Contextual<T, C>, VEC_SIZE>),
    Most(Option<Contextual<T, C>>),
}

impl<T, C: CapturedContext> From<AccumulatorState<T, C>> for ContextualIntoIter<T, C> {
    #[inline]
    fn from(value: AccumulatorState<T, C>) -> Self {
        match value {
            #[cfg(feature = "alloc")]
            AccumulatorState::All(vec) => Self::All(vec.into_iter()),
            AccumulatorState::Most(option) => Self::Most(option),
        }
    }
}

impl<T, C: CapturedContext> Iterator for ContextualIntoIter<T, C> {
    type Item = Contextual<T, C>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            #[cfg(feature = "alloc")]
            Self::All(iter) => iter.next(),
            Self::Most(option) => option.take(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            #[cfg(feature = "alloc")]
            Self::All(vec) => vec.size_hint(),
            Self::Most(Some(_)) => (1, Some(1)),
            Self::Most(None) => (0, Some(0)),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AccumulatorKind {
    #[cfg(feature = "alloc")]
    All,
    Most,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Contextuals<T, C: CapturedContext = NoLoc> {
    state: AccumulatorState<T, C>,
    ignored: usize,
}

impl<T, C: CapturedContext> Contextuals<T, C> {
    #[inline]
    #[must_use]
    pub const fn new(kind: AccumulatorKind) -> Self {
        Self {
            state: AccumulatorState::new(kind),
            ignored: 0,
        }
    }

    #[inline]
    pub const fn kind(&self) -> AccumulatorKind {
        self.state.kind()
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.state.is_empty()
    }

    #[inline]
    pub const fn iter(&'_ self) -> ContextualIter<'_, T, C> {
        self.state.iter()
    }

    #[inline]
    pub fn map<U>(self, map: impl FnMut(T) -> U) -> Contextuals<U, C> {
        Contextuals {
            state: self.state.map(map),
            ignored: self.ignored,
        }
    }

    #[inline]
    pub fn append_naive(&mut self, other: Self) {
        self.ignored += self.state.append_naive(other.state) + other.ignored;
    }

    #[inline]
    pub fn push_naive(&mut self, value: Contextual<T, C>) {
        if !self.state.push_naive(value) {
            self.ignored += 1;
        }
    }
}

impl<T: Prioritized, C: CapturedContext> Contextuals<T, C> {
    #[inline]
    pub fn append(&mut self, other: Self) {
        let ignored = self.state.append(other.state);
        self.ignored += ignored + other.ignored;
    }

    #[inline]
    pub fn push(&mut self, value: Contextual<T, C>) {
        if !self.state.push(value) {
            self.ignored += 1;
        }
    }
}

impl<T: Prioritized, C: CapturedContext> Extend<Contextual<T, C>> for Contextuals<T, C> {
    #[inline]
    fn extend<I: IntoIterator<Item = Contextual<T, C>>>(&mut self, iter: I) {
        let iter = iter.into_iter();

        #[cfg(feature = "alloc")]
        self.state.reserve(iter.size_hint().0);

        for item in iter {
            if !self.state.push(item) {
                self.ignored += 1;
            }
        }
    }
}

impl<T, C: CapturedContext> IntoIterator for Contextuals<T, C> {
    type Item = Contextual<T, C>;
    type IntoIter = ContextualIntoIter<T, C>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.state.into_iter()
    }
}

impl<'a, T, C: CapturedContext> IntoIterator for &'a Contextuals<T, C> {
    type Item = Contextual<&'a T, &'a C>;
    type IntoIter = ContextualIter<'a, T, C>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.state.iter()
    }
}

impl<T: Display, C: CapturedContext> Display for Contextuals<T, C> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        for (i, item) in self.iter().enumerate() {
            if i != 0 {
                writeln!(f)?;
            }

            write!(f, "{item}")?;
        }

        if self.ignored > 0 {
            if !self.is_empty() {
                writeln!(f)?;
            }

            write!(f, "... {} ignored", self.ignored)?;
        }

        Ok(())
    }
}

impl<T: Error, C: CapturedContext> Error for Contextuals<T, C> {
    // NOTE: fn source() cannot be implemented;
    //       An array of impl Error cannot be implicitly cast into dyn Error.
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

#[derive(Debug, Clone)]
#[must_use]
// NOTE: Not aliasing Result is intended; Trisult MUST be accumulated, NOT be fast failed.
pub enum Trisult<T, W, E, C: CapturedContext = NoLoc> {
    Ok(Diagnosed<T, W, C>),
    Err(Diagnoses<W, E, C>),
}

impl<T, W, E, C: CapturedContext> Trisult<T, W, E, C> {
    #[inline]
    pub const fn is_err(&self) -> bool {
        matches!(self, Self::Err(..))
    }

    #[inline]
    pub const fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(..))
    }

    #[inline]
    pub fn err(self) -> Option<Diagnoses<W, E, C>> {
        if let Self::Err(diags) = self {
            Some(diags)
        } else {
            None
        }
    }

    #[inline]
    pub fn ok(self) -> Option<Diagnosed<T, W, C>> {
        if let Self::Ok(diagnosed) = self {
            Some(diagnosed)
        } else {
            None
        }
    }

    #[inline]
    pub fn and_then<U, F>(self, then: F) -> Trisult<U, W, E, C>
    where
        F: FnOnce(T) -> Trisult<U, W, E, C>,
    {
        match self {
            Self::Ok(Diagnosed(value, mut diags)) => match then(value) {
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
    #[allow(clippy::missing_panics_doc)]
    #[inline]
    #[track_caller]
    pub fn expect(self, msg: &str) -> Diagnosed<T, W, C> {
        match self {
            Self::Ok(diag) => diag,
            Self::Err(err) => panic!("{}: {:?}", msg, err),
        }
    }

    #[inline]
    #[track_caller]
    pub fn unwrap(self) -> Diagnosed<T, W, C> {
        self.expect("called `Trisult::unwrap()` on an `Err` value")
    }
}
