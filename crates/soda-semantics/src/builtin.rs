use crate::{RuntimeError, RuntimeErrorKind};
use soda_ast::{Primitive, Term};

pub(crate) enum Builtin {
    Unknown,
    Pending,
    Value(Term),
    Error(RuntimeError),
}

/// `apply` is primitive only when its function is an explicit binder.
pub(crate) fn handles(name: &str, args: &[Term], extensible_apply: bool) -> bool {
    Primitive::from_name(name).is_some()
        && (name != "apply"
            || !extensible_apply
            || matches!(args.first(), Some(Term::Binder { .. })))
}

pub(crate) fn apply(name: &str, args: &[Term], extensible_apply: bool) -> Builtin {
    if !handles(name, args, extensible_apply) {
        return Builtin::Unknown;
    }

    let primitive = Primitive::from_name(name).expect("handled primitive name");
    let arity = primitive.arity();

    if args.len() != arity {
        return Builtin::Error(
            RuntimeError::new(format!(
                "{name} expects {arity} arguments, got {}",
                args.len()
            ))
            .with_kind(RuntimeErrorKind::Arity),
        );
    }

    match primitive {
        Primitive::Apply => {
            return match &args[0] {
                Term::Binder {
                    parameter, body, ..
                } => Builtin::Value(body.substitute_name(parameter, &args[1])),
                Term::App(..) | Term::Var(_) => Builtin::Pending,
                _ => Builtin::Error("apply expects a binder as its first argument".into()),
            };
        }
        Primitive::Substitute => {
            return match &args[1] {
                Term::Name(name) => Builtin::Value(args[0].substitute_name(name, &args[2])),
                _ => Builtin::Error("subst expects an object name as its second argument".into()),
            };
        }
        Primitive::Fresh => {
            return match &args[0] {
                Term::Name(name) => {
                    if !args[1].variables().is_empty() {
                        Builtin::Pending
                    } else {
                        Builtin::Value(Term::Bool(!args[1].free_names().contains(name)))
                    }
                }
                _ => Builtin::Error("fresh expects an object name as its first argument".into()),
            };
        }
        Primitive::AlphaEq => {
            return if args.iter().any(|arg| !arg.variables().is_empty()) {
                Builtin::Pending
            } else {
                Builtin::Value(Term::Bool(args[0].alpha_equivalent(&args[1])))
            };
        }
        _ => {}
    }

    if primitive == Primitive::If {
        return match &args[0] {
            Term::Bool(value) => Builtin::Value(args[if *value { 1 } else { 2 }].clone()),
            Term::Int(_) => Builtin::Error("if expects a boolean condition".into()),
            _ => Builtin::Pending,
        };
    }

    if args
        .iter()
        .any(|arg| matches!(arg, Term::App(..) | Term::Var(_)))
    {
        return Builtin::Pending;
    }

    let error = || {
        Builtin::Error(RuntimeError::new(format!(
            "invalid scalar operands for {name}"
        )))
    };

    match (primitive, &args[0], args.get(1)) {
        (Primitive::Not, Term::Bool(value), _) => Builtin::Value(Term::Bool(!value)),
        (Primitive::And, Term::Bool(a), Some(Term::Bool(b))) => {
            Builtin::Value(Term::Bool(*a && *b))
        }
        (Primitive::Or, Term::Bool(a), Some(Term::Bool(b))) => Builtin::Value(Term::Bool(*a || *b)),
        (Primitive::Equal, Term::Int(a), Some(Term::Int(b))) => Builtin::Value(Term::Bool(a == b)),
        (Primitive::Equal, Term::Bool(a), Some(Term::Bool(b))) => {
            Builtin::Value(Term::Bool(a == b))
        }
        (
            Primitive::Less | Primitive::LessEqual | Primitive::Greater | Primitive::GreaterEqual,
            Term::Int(a),
            Some(Term::Int(b)),
        ) => Builtin::Value(Term::Bool(match primitive {
            Primitive::Less => a < b,
            Primitive::LessEqual => a <= b,
            Primitive::Greater => a > b,
            _ => a >= b,
        })),
        (
            Primitive::Add
            | Primitive::Subtract
            | Primitive::Multiply
            | Primitive::Divide
            | Primitive::Remainder,
            Term::Int(a),
            Some(Term::Int(b)),
        ) => {
            if matches!(primitive, Primitive::Divide | Primitive::Remainder) && *b == 0 {
                return Builtin::Error(
                    RuntimeError::new(format!("{name} by zero"))
                        .with_kind(RuntimeErrorKind::DivisionByZero),
                );
            }

            let value = match primitive {
                Primitive::Add => a.checked_add(*b),
                Primitive::Subtract => a.checked_sub(*b),
                Primitive::Multiply => a.checked_mul(*b),
                Primitive::Divide => a.checked_div(*b),
                _ => a.checked_rem(*b),
            };

            value.map_or_else(
                || {
                    Builtin::Error(
                        RuntimeError::new(format!("integer overflow in {name}"))
                            .with_kind(RuntimeErrorKind::Overflow),
                    )
                },
                |n| Builtin::Value(Term::Int(n)),
            )
        }
        _ => error(),
    }
}
