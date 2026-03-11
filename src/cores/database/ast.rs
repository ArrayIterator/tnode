use crate::cores::database::expr::comparison::Comparison;

#[derive(Debug, Clone)]
pub enum Expr {
    Raw(String),
    Cmp(Comparison),
    And(Vec<Expr>),
    Or(Vec<Expr>),
}
