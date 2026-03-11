use std::fmt::{self, Display};

#[derive(Debug, Clone, Copy)]
pub enum JoinType {
    Inner,
    Left,
}

impl JoinType {
    fn as_str(&self) -> &'static str {
        match self {
            JoinType::Inner => "INNER",
            JoinType::Left => "LEFT",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ConditionType {
    On,
    With,
}

impl ConditionType {
    fn as_str(&self) -> &'static str {
        match self {
            ConditionType::On => "ON",
            ConditionType::With => "WITH",
        }
    }
}
#[derive(Debug, Clone)]
pub struct Join {
    join_type: JoinType,
    join: String,
    alias: Option<String>,
    condition_type: Option<ConditionType>,
    condition: Option<String>,
    index_by: Option<String>,
}
impl Join {
    pub fn new<J: Into<String>>(join_type: JoinType, join: J) -> Self {
        Self {
            join_type,
            join: join.into(),
            alias: None,
            condition_type: None,
            condition: None,
            index_by: None,
        }
    }

    pub fn alias<A: Into<String>>(mut self, alias: A) -> Self {
        self.alias = Some(alias.into());
        self
    }

    pub fn on<C: Into<String>>(mut self, condition: C) -> Self {
        self.condition_type = Some(ConditionType::On);
        self.condition = Some(condition.into());
        self
    }

    pub fn with<C: Into<String>>(mut self, condition: C) -> Self {
        self.condition_type = Some(ConditionType::With);
        self.condition = Some(condition.into());
        self
    }

    pub fn index_by<I: Into<String>>(mut self, index: I) -> Self {
        self.index_by = Some(index.into());
        self
    }
}

impl Display for Join {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} JOIN {}", self.join_type.as_str(), self.join)?;

        if let Some(alias) = &self.alias {
            write!(f, " {}", alias)?;
        }

        if let (Some(ct), Some(cond)) = (&self.condition_type, &self.condition) {
            write!(f, " {} {}", ct.as_str(), cond)?;
        }

        if let Some(index) = &self.index_by {
            write!(f, " INDEX BY {}", index)?;
        }

        Ok(())
    }
}

