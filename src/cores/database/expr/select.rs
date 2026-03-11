use crate::cores::database::expr::composite::{BaseExpr, Composite, ExprPart};
use std::fmt;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct Select {
    base: BaseExpr,
}

impl Select {
    pub fn new() -> Self {
        Self {
            base: BaseExpr {
                pre: "",
                sep: ", ",
                post: "",
                parts: Vec::new(),
            },
        }
    }

    pub fn add_raw<S: Into<String>>(&mut self, s: S) -> &mut Self {
        self.base.add(ExprPart::Raw(s.into()));
        self
    }

    pub fn add_expr(&mut self, expr: Composite) -> &mut Self {
        self.base.add(ExprPart::Composite(Box::new(expr)));
        self
    }

    pub fn count(&self) -> usize {
        self.base.count()
    }
}

impl Display for Select {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut rendered = Vec::new();

        for part in &self.base.parts {
            match part {
                ExprPart::Raw(s) => rendered.push(s.clone()),
                ExprPart::Composite(c) => rendered.push(c.to_string()),
            }
        }

        write!(f, "{}", rendered.join(self.base.sep))
    }
}
