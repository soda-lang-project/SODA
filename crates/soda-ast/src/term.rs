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

    pub fn variables(&self) -> BTreeSet<String> {
        let mut result = BTreeSet::new();
        self.collect_variables(&mut result);

        result
    }

    fn collect_variables(&self, result: &mut BTreeSet<String>) {
        match self {
            Self::Var(name) => {
                result.insert(name.clone());
            }
            Self::Binder { body, .. } => body.collect_variables(result),
            Self::App(_, args) => {
                for arg in args {
                    arg.collect_variables(result);
                }
            }
            _ => {}
        }
    }

    /// Simultaneous substitution; inserted terms are not substituted again.
    pub fn substitute(&self, substitution: &Substitution) -> Self {
        match self {
            Self::Var(name) => substitution
                .get(name)
                .cloned()
                .unwrap_or_else(|| self.clone()),
            Self::Binder {
                parameter,
                sort,
                body,
            } => {
                let variables = body.variables();
                let inserted: BTreeSet<String> = substitution
                    .iter()
                    .filter(|(name, _)| variables.contains(*name))
                    .flat_map(|(_, value)| value.free_names())
                    .collect();

                if inserted.contains(parameter) {
                    let (parameter, body) = Self::freshen(parameter, body, &inserted);

                    Self::Binder {
                        parameter,
                        sort: sort.clone(),
                        body: Box::new(body.substitute(substitution)),
                    }
                } else {
                    Self::Binder {
                        parameter: parameter.clone(),
                        sort: sort.clone(),
                        body: Box::new(body.substitute(substitution)),
                    }
                }
            }
            Self::App(name, args) => Self::App(
                name.clone(),
                args.iter().map(|a| a.substitute(substitution)).collect(),
            ),
            _ => self.clone(),
        }
    }

    pub fn at(&self, path: &[usize]) -> Option<&Self> {
        match path.split_first() {
            None => Some(self),
            Some((index, rest)) => match self {
                Self::App(_, args) => args.get(*index)?.at(rest),
                Self::Binder { body, .. } if *index == 0 => body.at(rest),
                _ => None,
            },
        }
    }

    /// Rejects replacement if either subtree depends on an enclosing bound name.
    pub fn replace(&self, path: &[usize], replacement: Self) -> Option<Self> {
        match path.split_first() {
            None => Some(replacement),
            Some((index, rest)) => {
                if let Self::Binder {
                    parameter,
                    sort,
                    body,
                } = self
                {
                    if *index != 0
                        || replacement.free_names().contains(parameter)
                        || body.at(rest)?.free_names().contains(parameter)
                    {
                        return None;
                    }

                    return Some(Self::Binder {
                        parameter: parameter.clone(),
                        sort: sort.clone(),
                        body: Box::new(body.replace(rest, replacement)?),
                    });
                }

                let Self::App(name, args) = self else {
                    return None;
                };

                let updated = args.get(*index)?.replace(rest, replacement)?;
                let mut args = args.clone();
                args[*index] = updated;

                Some(Self::App(name.clone(), args))
            }
        }
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
