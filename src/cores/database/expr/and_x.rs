use crate::cores::database::expr::composite::Composite;
use crate::cores::database::expr::expr::Expression;
use crate::cores::database::expr::or_x::OrX;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub struct AndX {
    pub inner: Composite,
    pub parts: Vec<Box<dyn Expression>>,
}

impl AndX {
    pub fn new(parts: Vec<Box<dyn Expression>>) -> Self {
        Self {
            inner: Composite::and(),
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

    pub fn count(&self) -> usize {
        self.inner.base.count()
    }
}

impl Display for AndX {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let rendered: Vec<String> = self
            .parts
            .iter()
            .map(|e| {
                let s = e.to_string();
                format!("({s})")
            })
            .collect();

        write!(f, "{}", rendered.join(" AND "))
    }
}
