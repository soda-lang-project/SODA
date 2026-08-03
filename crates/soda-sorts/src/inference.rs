mod builtins;
use std::collections::BTreeMap;

use soda_ast::Term;

use crate::{Sort, SortError, TypeChecker};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Type {
    Variable(usize),
    Named(String),
    Function(Box<Type>, Box<Type>),
}

impl From<&Sort> for Type {
    fn from(sort: &Sort) -> Self {
        match sort {
            Sort::Named(name) => Self::Named(name.clone()),
            Sort::Function(input, output) => Self::Function(
                Box::new(Self::from(input.as_ref())),
                Box::new(Self::from(output.as_ref())),
            ),
        }
    }
}

pub(crate) struct Scope<'a> {
    checker: &'a TypeChecker,
    variables: BTreeMap<String, Type>,
    names: BTreeMap<String, Type>,
    assignments: BTreeMap<usize, Type>,
    next: usize,
}

impl<'a> Scope<'a> {
    pub fn new(checker: &'a TypeChecker) -> Self {
        Self {
            checker,
            variables: BTreeMap::new(),
            names: BTreeMap::new(),
            assignments: BTreeMap::new(),
            next: 0,
        }
    }

    fn fresh(&mut self) -> Type {
        let ty = Type::Variable(self.next);
        self.next += 1;

        ty
    }

    fn resolve(&self, ty: &Type) -> Type {
        match ty {
            Type::Variable(id) => self
                .assignments
                .get(id)
                .map_or_else(|| ty.clone(), |value| self.resolve(value)),
            Type::Function(input, output) => Type::Function(
                Box::new(self.resolve(input)),
                Box::new(self.resolve(output)),
            ),
            _ => ty.clone(),
        }
    }

    fn unify(&mut self, left: &Type, right: &Type) -> Result<(), SortError> {
        let left = self.resolve(left);
        let right = self.resolve(right);

        if left == right {
            return Ok(());
        }

        match (&left, &right) {
            (Type::Variable(id), ty) | (ty, Type::Variable(id)) => {
                if contains(ty, *id) {
                    return Err(SortError::new("infinite function sort".into()));
                }

                self.assignments.insert(*id, ty.clone());

                Ok(())
            }
            (Type::Function(a, b), Type::Function(c, d)) => {
                self.unify(a, c)?;

                self.unify(b, d)
            }
            _ => Err(SortError::mismatch(describe(&right), describe(&left))),
        }
    }

    pub fn materialize(&self, ty: &Type) -> Result<Sort, SortError> {
        match self.resolve(ty) {
            Type::Named(name) => Ok(Sort::Named(name)),
            Type::Function(input, output) => Ok(Sort::Function(
                Box::new(self.materialize(&input)?),
                Box::new(self.materialize(&output)?),
            )),
            Type::Variable(_) => Err(SortError::new(
                "ambiguous sort; constrain variables with a declared signature".into(),
            )),
        }
    }

    pub fn finish(&self) -> Result<(), SortError> {
        for (name, ty) in &self.variables {
            self.materialize(ty)
                .map_err(|error| error.at(&Term::Var(name.clone())))?;
        }

        Ok(())
    }

    pub fn variable_sorts(&self) -> Result<BTreeMap<String, Sort>, SortError> {
        self.variables
            .iter()
            .map(|(name, ty)| {
                Ok((
                    name.clone(),
                    self.materialize(ty)
                        .map_err(|error| error.at(&Term::Var(name.clone())))?,
                ))
            })
            .collect()
    }

    pub fn expected(&mut self, term: &Term, sort: &Sort) -> Result<(), SortError> {
        let actual = self.term(term)?;

        self.unify(&actual, &Type::from(sort))
            .map_err(|error| error.at(term))
    }

    pub fn enter_position(&mut self, term: &Term, path: &[usize]) -> Result<(), SortError> {
        let Some((index, rest)) = path.split_first() else {
            return Ok(());
        };

        match term {
            Term::App(_, args) => {
                let child = args
                    .get(*index)
                    .ok_or_else(|| SortError::new("invalid proof position".into()))?;

                self.enter_position(child, rest)
            }
            Term::Binder {
                parameter,
                sort,
                body,
            } if *index == 0 => {
                self.names.insert(
                    parameter.clone(),
                    Type::from(&self.checker.parse_sort(sort)?),
                );

                self.enter_position(body, rest)
            }
            _ => Err(SortError::new("invalid proof position".into())),
        }
    }

    pub fn clear_names(&mut self) {
        self.names.clear();
    }

    pub fn equation(&mut self, lhs: &Term, rhs: &Term) -> Result<(), SortError> {
        let left = self.term(lhs)?;
        let right = self.term(rhs)?;

        self.unify(&right, &left).map_err(|error| error.at(rhs))
    }

    pub fn boolean(&mut self, term: &Term) -> Result<(), SortError> {
        let ty = self.term(term)?;

        self.unify(&ty, &Type::Named("Bool".into()))
            .map_err(|error| error.at(term))
    }

    pub fn judgment(&mut self, term: &Term) -> Result<(), SortError> {
        let Term::App(name, args) = term else {
            return Err(SortError::new("expected a declared judgment application".into()).at(term));
        };

        let expected = self
            .checker
            .judgments
            .get(name)
            .ok_or_else(|| {
                SortError::new(format!("undeclared judgment `{name}`"))
                    .symbol(name)
                    .at(term)
            })?
            .clone();

        self.arguments(name, args, &expected)
            .map_err(|error| error.at(term))
    }

    fn arguments(&mut self, name: &str, args: &[Term], expected: &[Sort]) -> Result<(), SortError> {
        arity(name, args, expected.len())?;
        for (argument, sort) in args.iter().zip(expected) {
            let actual = self.term(argument)?;
            self.unify(&actual, &Type::from(sort))
                .map_err(|error| error.at(argument))?;
        }

        Ok(())
    }

    pub fn term(&mut self, term: &Term) -> Result<Type, SortError> {
        self.term_inner(term).map_err(|error| error.at(term))
    }

    fn term_inner(&mut self, term: &Term) -> Result<Type, SortError> {
        match term {
            Term::Int(_) => Ok(Type::Named("Int".into())),
            Term::Bool(_) => Ok(Type::Named("Bool".into())),
            Term::Var(name) => {
                if let Some(ty) = self.variables.get(name) {
                    return Ok(ty.clone());
                }

                let ty = self.fresh();
                self.variables.insert(name.clone(), ty.clone());

                Ok(ty)
            }
            Term::Name(name) => self.names.get(name).cloned().ok_or_else(|| {
                SortError::new(format!("unbound object name `@{name}`"))
                    .with_kind(crate::SortErrorKind::UnboundName)
            }),
            Term::Binder {
                parameter,
                sort,
                body,
            } => {
                let input = Type::from(&self.checker.parse_sort(sort)?);
                let previous = self.names.insert(parameter.clone(), input.clone());
                let result = self.term(body);

                if let Some(previous) = previous {
                    self.names.insert(parameter.clone(), previous);
                } else {
                    self.names.remove(parameter);
                }

                Ok(Type::Function(Box::new(input), Box::new(result?)))
            }
            Term::App(name, args) => {
                if let Some((expected, result)) = self.checker.signatures.get(name).cloned() {
                    self.arguments(name, args, &expected)?;

                    return Ok(Type::from(&result));
                }

                if self.checker.judgments.contains_key(name) {
                    return Err(SortError::new(format!(
                        "judgment `{name}` cannot be used as a data expression"
                    )));
                }

                self.builtin(name, args)
            }
        }
    }
}

fn arity(name: &str, args: &[Term], expected: usize) -> Result<(), SortError> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(SortError::new(format!(
            "`{name}` expects {expected} arguments, found {}",
            args.len()
        ))
        .with_kind(crate::SortErrorKind::Arity))
    }
}

fn contains(ty: &Type, id: usize) -> bool {
    match ty {
        Type::Variable(other) => *other == id,
        Type::Function(input, output) => contains(input, id) || contains(output, id),
        _ => false,
    }
}

fn describe(ty: &Type) -> String {
    match ty {
        Type::Variable(id) => format!("?sort{id}"),
        Type::Named(name) => name.clone(),
        Type::Function(input, output) => format!("Fn({}, {})", describe(input), describe(output)),
    }
}
