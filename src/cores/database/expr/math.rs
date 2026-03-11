use std::fmt::{self, Display};

#[derive(Debug, Clone)]
pub enum MathExpr {
    Raw(String),
    Math(Box<Math>),
}

impl From<&str> for MathExpr {
    fn from(v: &str) -> Self {
        MathExpr::Raw(v.to_string())
    }
}

impl From<String> for MathExpr {
    fn from(v: String) -> Self {
        MathExpr::Raw(v)
    }
}

impl From<Math> for MathExpr {
    fn from(v: Math) -> Self {
        MathExpr::Math(Box::new(v))
    }
}

#[derive(Debug, Clone)]
pub struct Math {
    left: MathExpr,
    operator: String,
    right: MathExpr,
}

impl Math {
    pub fn new<L, O, R>(left: L, operator: O, right: R) -> Self
    where
        L: Into<MathExpr>,
        O: Into<String>,
        R: Into<MathExpr>,
    {
        Self {
            left: left.into(),
            operator: operator.into(),
            right: right.into(),
        }
    }
}
impl Display for Math {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let left = match &self.left {
            MathExpr::Raw(s) => s.clone(),
            MathExpr::Math(m) => format!("({m})"),
        };

        let right = match &self.right {
            MathExpr::Raw(s) => s.clone(),
            MathExpr::Math(m) => format!("({m})"),
        };

        write!(f, "{} {} {}", left, self.operator, right)
    }
}
