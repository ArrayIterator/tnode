use crate::cores::database::expr::composite::{BaseExpr, ExprPart};
use std::fmt::{self, Display};

#[derive(Debug, Clone)]
pub struct GroupBy {
    base: BaseExpr,
}
impl GroupBy {
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

    pub fn add<S: Into<String>>(&mut self, part: S) -> &mut Self {
        self.base.add(ExprPart::Raw(part.into()));
        self
    }

    pub fn add_multiple<I, S>(&mut self, parts: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for p in parts {
            self.add(p);
        }
        self
    }
}

impl Display for GroupBy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.base.count() == 1 {
            if let Some(ExprPart::Raw(s)) = self.base.parts.first() {
                return write!(f, "{s}");
            }
        }

        let rendered: Vec<String> = self.base.parts.iter().map(|p| {
            match p {
                ExprPart::Raw(s) => s.clone(),
                ExprPart::Composite(c) => c.to_string(),
            }
        }).collect();

        write!(f, "{}{}{}",
               self.base.pre,
               rendered.join(self.base.sep),
               self.base.post
        )
    }
}
