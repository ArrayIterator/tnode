use std::fmt::{self, Display};

#[derive(Debug, Clone)]
pub struct Func {
    name: String,
    arguments: Vec<String>,
}

impl Func {
    pub fn new<N, A>(name: N, arguments: A) -> Self
    where
        N: Into<String>,
        A: Into<FuncArgs>,
    {
        let args = arguments.into();

        Self {
            name: name.into(),
            arguments: args.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FuncArgs(Vec<String>);

impl From<&str> for FuncArgs {
    fn from(value: &str) -> Self {
        FuncArgs(vec![value.to_string()])
    }
}

impl From<String> for FuncArgs {
    fn from(value: String) -> Self {
        FuncArgs(vec![value])
    }
}

impl From<Vec<&str>> for FuncArgs {
    fn from(values: Vec<&str>) -> Self {
        FuncArgs(values.into_iter().map(|v| v.to_string()).collect())
    }
}

impl From<Vec<String>> for FuncArgs {
    fn from(values: Vec<String>) -> Self {
        FuncArgs(values)
    }
}

impl Display for Func {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}({})",
            self.name,
            self.arguments.join(", ")
        )
    }
}