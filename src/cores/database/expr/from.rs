use std::fmt::{self, Display};

#[derive(Debug, Clone)]
pub struct From {
    from: String,
    alias: String,
    index_by: Option<String>,
}

impl From {
    pub fn new<F, A, I>(from: F, alias: A, index_by: Option<I>) -> Self
    where
        F: Into<String>,
        A: Into<String>,
        I: Into<String>,
    {
        Self {
            from: from.into(),
            alias: alias.into(),
            index_by: index_by.map(|i| i.into()),
        }
    }

    pub fn from(&self) -> &str {
        &self.from
    }

    pub fn alias(&self) -> &str {
        &self.alias
    }

    pub fn index_by(&self) -> Option<&str> {
        self.index_by.as_deref()
    }
}

impl Display for From {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.from, self.alias)?;

        if let Some(index) = &self.index_by {
            write!(f, " INDEX BY {}", index)?;
        }

        Ok(())
    }
}
