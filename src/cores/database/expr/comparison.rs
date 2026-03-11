#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Comparison {
    left: String,
    operator: &'static str,
    right: String,
}

impl Comparison {
    pub const EQ: &'static str = "=";
    pub const NEQ: &'static str = "<>";
    pub const LT: &'static str = "<";
    pub const LTE: &'static str = "<=";
    pub const GT: &'static str = ">";
    pub const GTE: &'static str = ">=";

    pub fn new<L, R>(left: L, operator: &'static str, right: R) -> Self
    where
        L: Into<String>,
        R: Into<String>,
    {
        Self {
            left: left.into(),
            operator,
            right: right.into(),
        }
    }
}

impl std::fmt::Display for Comparison {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.left, self.operator, self.right)
    }
}
