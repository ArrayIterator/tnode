use crate::cores::database::expr::and_x::AndX;
use crate::cores::database::expr::comparison::Comparison;
use crate::cores::database::expr::func::Func;
use crate::cores::database::expr::literal::Literal;
use std::fmt::{Debug, Display};

pub trait ExpressionClone {
    fn clone_box(&self) -> Box<dyn Expression>;
}

impl<T> ExpressionClone for T
where
    T: Expression + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn Expression> {
        Box::new(self.clone())
    }
}

pub trait Expression: Display + Debug + ExpressionClone {}

impl<T> Expression for T where
    T: Display + Debug + Clone + 'static {}

impl Clone for Box<dyn Expression> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Debug, Clone)]
pub struct Expr;

impl Expr {
    pub fn eq<L, R>(l: L, r: R) -> Comparison
    where
        L: Into<String>,
        R: Into<String>,
    {
        Comparison::new(l, Comparison::EQ, r)
    }

    pub fn neq<L, R>(l: L, r: R) -> Comparison
    where
        L: Into<String>,
        R: Into<String>,
    {
        Comparison::new(l, Comparison::NEQ, r)
    }

    pub fn and_x(parts: Vec<Box<dyn Expression>>) -> AndX {
        AndX::new(parts)
    }

    pub fn avg<X: Into<String>>(x: X) -> Func {
        Func::new("AVG", vec![x.into()])
    }

    pub fn count<X: Into<String>>(x: X) -> Func {
        Func::new("COUNT", vec![x.into()])
    }

    pub fn literal<X: Into<String>>(x: X) -> Literal {
        Literal::new(x)
    }
}
