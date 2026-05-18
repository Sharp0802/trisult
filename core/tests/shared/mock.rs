use thiserror::Error;
use trisult::{ContextStack, ContextStackMut, Default, Trisult};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[allow(unused)]
pub struct TraceStack {
    pub path: Vec<&'static str>,
}

impl ContextStack for TraceStack {
    type Captured = String;
}

impl ContextStackMut for TraceStack {
    type Segment = &'static str;

    fn capture(&self) -> Self::Captured {
        let mut buffer = String::new();
        for item in &self.path {
            buffer.push('/');
            buffer.push_str(item);
        }
        buffer
    }

    fn push(&mut self, segment: Self::Segment) {
        self.path.push(segment);
    }

    fn pop(&mut self) {
        self.path.pop();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[allow(unused)]
pub enum MockWarn {
    #[error("minor issue")]
    MinorIssue,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[allow(unused)]
pub enum MockErr {
    #[error("fatal issue")]
    FatalIssue,
}

#[allow(unused)]
pub type MockResult<T> = Trisult<T, MockWarn, MockErr, String, Default>;
