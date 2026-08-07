use super::{Attempt, Context, Engine, argument_path, within_depth};
use crate::{Config, Evaluation, Limit, Status, Strategy};
use soda_ast::Term;

impl Engine {
    /// Evaluates arguments before their parent rule, keeping `if` branches lazy.
    /// Independent of the operational strategy.
    pub fn denote(&self, term: &Term, config: &Config) -> Evaluation {
        let config = Config {
            strategy: Strategy::Innermost,
            ..config.bounded()
        };
        let mut context = Context {
            config: &config,
            steps: 0,
            trace: Vec::new(),
            last_rule: None,
        };
        let (term, status) = self.compositional(term.clone(), &mut context, 0, &[]);

        Evaluation {
            term,
            status,
            steps: context.steps,
            trace: context.trace,
            last_rule: context.last_rule,
        }
    }

    pub(super) fn compositional(
        &self,
        mut term: Term,
        context: &mut Context<'_>,
        depth: usize,
        path: &[usize],
    ) -> (Term, Status) {
        loop {
            if !within_depth(&term, depth, context.config.max_depth) {
                return (term, Status::Limit(Limit::Depth));
            }

            if let Term::App(name, args) = &mut term {
                let count = self.evaluated_arguments(name, args);

                for (index, argument) in args.iter_mut().take(count).enumerate() {
                    let child_path = if context.config.trace {
                        argument_path(path, index)
                    } else {
                        Vec::new()
                    };

                    let owned = std::mem::replace(argument, Term::Bool(false));
                    let (value, status) =
                        self.compositional(owned, context, depth + 1, &child_path);
                    *argument = value;

                    if status != Status::NormalForm {
                        return (term, status);
                    }
                }
            }

            match self.attempt(&term, context, depth) {
                Ok(Attempt::Ready {
                    term: replacement,
                    rule,
                }) => match self.contract(&term, replacement, rule, context, path, true) {
                    Ok(next) => term = next,
                    Err(status) => return (term, status),
                },
                Ok(Attempt::Idle(reason)) => {
                    return (term, reason.map_or(Status::NormalForm, Status::Stuck));
                }
                Err(status) => return (term, status),
            }
        }
    }
}
