use std::fmt;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum ExprPart {
    Raw(String),
    Composite(Box<Composite>),
}

#[derive(Debug, Clone)]
pub struct Composite {
    pub(crate) base: BaseExpr,
    separator: Separator,
}

#[derive(Debug, Clone, Copy)]
pub enum Separator {
    And,
    Or,
}

impl Separator {
    pub fn as_str(&self) -> &'static str {
        match self {
            Separator::And => " AND ",
            Separator::Or => " OR ",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BaseExpr {
    pub pre: &'static str,
    pub sep: &'static str,
    pub post: &'static str,
    pub parts: Vec<ExprPart>,
}

impl BaseExpr {
    pub fn new(sep: &'static str) -> Self {
        Self {
            pre: "(",
            sep,
            post: ")",
            parts: Vec::new(),
        }
    }

    pub fn add(&mut self, part: ExprPart) -> &mut Self {
        self.parts.push(part);
        self
    }

    pub fn add_multiple(&mut self, parts: Vec<ExprPart>) -> &mut Self {
        for part in parts {
            self.add(part);
        }
        self
    }

    pub fn count(&self) -> usize {
        self.parts.len()
    }
}

impl Composite {
    pub fn and() -> Self {
        Self {
            base: BaseExpr::new(" AND "),
            separator: Separator::And,
        }
    }

    pub fn or() -> Self {
        Self {
            base: BaseExpr::new(" OR "),
            separator: Separator::Or,
        }
    }

    pub fn add_raw<S>(&mut self, s: S) -> &mut Self
    where
        S: Into<String>,
    {
        self.base.add(ExprPart::Raw(s.into()));
        self
    }

    pub fn add_expr(&mut self, expr: Composite) -> &mut Self {
        self.base
            .add(ExprPart::Composite(Box::new(expr)));
        self
    }
}

impl Display for Composite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // single element → no wrapping
        if self.base.count() == 1 {
            return match &self.base.parts[0] {
                ExprPart::Raw(s) => {
                    write!(f, "{s}")
                }
                ExprPart::Composite(c) => {
                    write!(f, "{c}")
                }
            }
        }

        let mut rendered: Vec<String> = Vec::with_capacity(self.base.parts.len());

        for part in &self.base.parts {
            let mut s = match part {
                ExprPart::Raw(raw) => raw.clone(),
                ExprPart::Composite(comp) => {
                    let inner = comp.to_string();
                    if comp.base.count() > 1 {
                        format!("({inner})")
                    } else {
                        inner
                    }
                }
            };

            // Doctrine-like behavior:
            // auto-wrap if AND/OR is detected
            if s.contains(" AND ") || s.contains(" OR ") {
                s = format!("({s})");
            }

            rendered.push(s);
        }

        write!(f, "{}", rendered.join(self.separator.as_str()))
    }
}
