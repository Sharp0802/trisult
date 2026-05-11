use std::fmt;
use std::ops::Deref;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(unused)]
pub struct Offset(pub usize);

impl fmt::Display for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at {}", self.0)
    }
}

#[allow(unused)]
pub struct Span<'a> {
    str: &'a str,
    at: usize,
}

impl<'a> Span<'a> {
    #[allow(unused)]
    pub fn new(str: &'a str, at: usize) -> Self {
        Self { str, at }
    }

    #[allow(unused)]
    pub fn at(&self) -> Offset {
        Offset(self.at)
    }
}

impl<'a> Deref for Span<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.str
    }
}
