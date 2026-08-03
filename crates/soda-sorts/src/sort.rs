use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sort {
    Named(String),
    Function(Box<Sort>, Box<Sort>),
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(name) => f.write_str(name),
            Self::Function(input, output) => write!(f, "Fn({input}, {output})"),
        }
    }
}
