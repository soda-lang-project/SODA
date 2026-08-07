use super::{Attempt, Context, Engine, RuleOrigin};
use crate::{
    Limit, Status, TraceStep,
    builtin::{self, Builtin},
};
use soda_ast::{Rule, Term, match_term};

impl Engine {
    pub(super) fn is_value(&self, term: &Term) -> bool {
        match term {
            Term::Var(_) => false,
            Term::Int(_) | Term::Bool(_) | Term::Name(_) | Term::Binder { .. } => true,
            Term::App(name, args) => {
                !self.heads.contains(name)
                    && !soda_ast::Primitive::from_name(name).is_some()
                    && args.iter().all(|arg| self.is_value(arg))
            }
        }
    }

    pub(super) fn attempt(
        &self,
        term: &Term,
        context: &mut Context<'_>,
        depth: usize,
    ) -> Result<Attempt, Status> {
        if depth > context.config.max_depth {
            return Err(Status::Limit(Limit::Depth));
        }

        if let Term::App(name, args) = term {
            let builtin =
                if name == "eq" && args.len() == 2 && args.iter().all(|arg| self.is_value(arg)) {
                    Builtin::Value(Term::Bool(args[0].alpha_equivalent(&args[1])))
                } else {
                    builtin::apply(name, args, self.heads.contains("apply"))
                };

            match builtin {
                Builtin::Value(term) => {
                    return Ok(Attempt::Ready {
                        term,
                        rule: RuleOrigin::Primitive(
                            soda_ast::Primitive::from_name(name).expect("recognized primitive"),
                        ),
                    });
                }
                Builtin::Error(message) => return Err(Status::Error(Box::new(message))),
                Builtin::Pending => {
                    return Ok(Attempt::Idle(Some(format!(
                        "{name} awaits valid value operands"
                    ))));
                }
                Builtin::Unknown => {}
            }
        }

        let mut blocked = None;

        for rule in self.candidates(term) {
            match self.attempt_rule(rule, term, context, depth) {
                Ok(ready @ Attempt::Ready { .. }) => return Ok(ready),
                Ok(Attempt::Idle(reason)) => {
                    if reason.is_some() {
                        blocked = reason;
                    }
                }
                Err(Status::Stuck(reason)) => blocked = Some(reason),
                Err(status) => return Err(status),
            }
        }

        if blocked.is_none() {
            blocked = match term {
                Term::Var(name) => Some(format!("free variable ?{name}")),
                Term::App(name, _) if self.heads.contains(name) => {
                    Some(format!("no applicable rule for {name}"))
                }
                _ => None,
            };
        }

        Ok(Attempt::Idle(blocked))
    }

    pub(super) fn attempt_rule(
        &self,
        rule: &Rule,
        term: &Term,
        context: &mut Context<'_>,
        depth: usize,
    ) -> Result<Attempt, Status> {
        let Some(substitution) = match_term(&rule.lhs, term) else {
            return Ok(Attempt::Idle(None));
        };

        if let Some(condition) = &rule.condition {
            let (value, status) = self.operational(
                condition.substitute(&substitution),
                context,
                depth + 1,
                &[],
                false,
            );

            match (status, value) {
                (Status::NormalForm, Term::Bool(true)) => {}
                (Status::NormalForm, Term::Bool(false)) => {
                    return Ok(Attempt::Idle(Some(format!(
                        "guard of {} is false",
                        rule.name
                    ))));
                }
                (Status::Error(message), _) => {
                    return Err(Status::Error(Box::new((*message).in_guard(&rule.name))));
                }
                (Status::Limit(limit), _) => return Err(Status::Limit(limit)),
                _ => {
                    return Err(Status::Stuck(format!(
                        "guard of {} did not produce a boolean",
                        rule.name
                    )));
                }
            }
        }

        Ok(Attempt::Ready {
            term: rule.rhs.substitute(&substitution),
            rule: RuleOrigin::User(rule.name.clone()),
        })
    }

    pub(super) fn contract(
        &self,
        before: &Term,
        term: Term,
        rule: RuleOrigin,
        context: &mut Context<'_>,
        path: &[usize],
        record: bool,
    ) -> Result<Term, Status> {
        if context.steps >= context.config.fuel {
            return Err(Status::Limit(Limit::Fuel));
        }

        context.steps += 1;
        if record && let RuleOrigin::User(name) = &rule {
            context.last_rule = Some(name.clone());
        }

        if record && context.config.trace {
            context.trace.push(TraceStep {
                rule: rule.to_string(),
                path: path.to_vec(),
                before: before.clone(),
                after: term.clone(),
            });
        }

        Ok(term)
    }
}
