use std::fmt;

#[derive(Debug, Clone)]
pub struct Literal(String);

impl Literal {
    pub fn new<S: Into<String>>(s: S) -> Self {
        Self(s.into())
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
