use std::fmt;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct OrderBy {
    pre: &'static str,
    separator: &'static str,
    post: &'static str,
    parts: Vec<String>,
}

impl OrderBy {
    pub fn new(sort: Option<&str>, order: Option<&str>) -> Self {
        let mut ob = Self {
            pre: "",
            separator: ", ",
            post: "",
            parts: Vec::new(),
        };

        if let Some(sort) = sort {
            ob.add(sort, order);
        }

        ob
    }

    pub fn add<S: AsRef<str>>(&mut self, sort: S, order: Option<&str>) -> &mut Self {
        let order = order.unwrap_or("ASC");
        self.parts
            .push(format!("{} {}", sort.as_ref(), order));
        self
    }

    pub fn count(&self) -> usize {
        self.parts.len()
    }
}

impl Display for OrderBy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.pre,
            self.parts.join(self.separator),
            self.post
        )
    }
}
