use crate::Substitution;
use std::{collections::BTreeSet, fmt};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Term {
    Var(String),
    /// An object-language variable, distinct from a rule metavariable.
    Name(String),
    Binder {
        parameter: String,
        sort: String,
        body: Box<Term>,
    },
    Int(i64),
    Bool(bool),
    App(String, Vec<Term>),
}

impl Term {
    pub fn app(name: impl Into<String>, args: Vec<Self>) -> Self {
        Self::App(name.into(), args)
    }

}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Var(name) => write!(f, "?{name}"),
            Self::Name(name) => write!(f, "@{name}"),
            Self::Binder {
                parameter,
                sort,
                body,
            } => write!(f, "bind({parameter}: {sort}, {body})"),
            Self::Int(value) => write!(f, "{value}"),
            Self::Bool(value) => write!(f, "{value}"),
            Self::App(name, args) => {
                write!(f, "{name}")?;
                if !args.is_empty() {
                    write!(f, "(")?;
                    for (index, arg) in args.iter().enumerate() {
                        if index > 0 {
                            write!(f, ", ")?;
                        }

                        write!(f, "{arg}")?;
                    }

                    write!(f, ")")?;
                }

                Ok(())
            }
        }
    }
}
