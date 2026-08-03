use super::{Scope, Type, arity};
use crate::{Sort, SortError};
use soda_ast::Term;

impl Scope<'_> {
    pub(super) fn builtin(&mut self, name: &str, args: &[Term]) -> Result<Type, SortError> {
        let Some(primitive) = soda_ast::Primitive::from_name(name) else {
            return Err(SortError::new(format!("undeclared symbol `{name}`")).symbol(name));
        };

        use soda_ast::Primitive::*;

        match primitive {
            Add | Subtract | Multiply | Divide | Remainder | Less | LessEqual | Greater
            | GreaterEqual => {
                self.arguments(
                    name,
                    args,
                    &[Sort::Named("Int".into()), Sort::Named("Int".into())],
                )?;

                Ok(Type::Named(
                    if matches!(primitive, Less | LessEqual | Greater | GreaterEqual) {
                        "Bool"
                    } else {
                        "Int"
                    }
                    .into(),
                ))
            }
            And | Or | Not => {
                self.arguments(
                    name,
                    args,
                    &vec![Sort::Named("Bool".into()); primitive.arity()],
                )?;

                Ok(Type::Named("Bool".into()))
            }
            Equal | AlphaEq => {
                arity(name, args, primitive.arity())?;
                self.equation(&args[0], &args[1])?;

                Ok(Type::Named("Bool".into()))
            }
            If => {
                arity(name, args, primitive.arity())?;
                self.boolean(&args[0])?;
                self.equation(&args[1], &args[2])?;

                self.term(&args[1])
            }
            Apply => {
                arity(name, args, primitive.arity())?;
                let function = self.term(&args[0])?;
                let input = self.term(&args[1])?;
                let output = self.fresh();
                self.unify(
                    &Type::Function(Box::new(input), Box::new(output.clone())),
                    &function,
                )
                .map_err(|error| error.at(&args[1]))?;

                Ok(output)
            }
            Substitute => self.substitution(args),
            Fresh => {
                arity(name, args, primitive.arity())?;
                if !matches!(args[0], Term::Name(_)) {
                    return Err(SortError::new("fresh target must be an object name".into()));
                }

                self.term(&args[1])?;

                Ok(Type::Named("Bool".into()))
            }
        }
    }

    fn substitution(&mut self, args: &[Term]) -> Result<Type, SortError> {
        arity("subst", args, 3)?;
        let Term::Name(name) = &args[1] else {
            return Err(SortError::new("subst target must be an object name".into()));
        };

        let replacement = self.term(&args[2])?;

        if let Some(existing) = self.names.get(name).cloned() {
            self.unify(&existing, &replacement)?;
        }

        let previous = self.names.insert(name.clone(), replacement);
        let result = self.term(&args[0]);

        if let Some(previous) = previous {
            self.names.insert(name.clone(), previous);
        } else {
            self.names.remove(name);
        }

        result
    }
}
