use crate::cores::database::expr::and_x::AndX;
use crate::cores::database::expr::composite::Composite;
use crate::cores::database::expr::expr::Expression;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub struct OrX {
    pub inner: Composite,
    parts: Vec<Box<dyn Expression>>,
}

impl OrX {
    pub fn new(parts: Vec<Box<dyn Expression>>) -> Self {
        Self {
            inner: Composite::or(),
            parts
        }
    }

    pub fn add_raw<S: Into<String>>(&mut self, s: S) -> &mut Self {
        self.inner.add_raw(s);
        self
    }

    pub fn add_and(&mut self, and: AndX) -> &mut Self {
        self.inner.add_expr(and.inner);
        self
    }

    pub fn add_or(&mut self, orx: OrX) -> &mut Self {
        self.inner.add_expr(orx.inner);
        self
    }
}


impl Display for OrX {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let rendered: Vec<String> = self
            .parts
            .iter()
            .map(|e| {
                let s = e.to_string();
                format!("({s})")
            })
            .collect();

        write!(f, "{}", rendered.join(" OR "))
    }
}
