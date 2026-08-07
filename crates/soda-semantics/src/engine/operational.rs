use super::{Attempt, Context, Engine, Search, argument_path, within_depth};
use crate::{Config, Evaluation, Limit, Status, Strategy};
use soda_ast::Term;

impl Engine {
    pub fn evaluate(&self, term: &Term, config: &Config) -> Evaluation {
        let config = config.bounded();
        let mut context = Context {
            config: &config,
            steps: 0,
            trace: Vec::new(),
            last_rule: None,
        };
        let (term, status) = self.operational(term.clone(), &mut context, 0, &[], true);

        Evaluation {
            term,
            status,
            steps: context.steps,
            trace: context.trace,
            last_rule: context.last_rule,
        }
    }

    pub(super) fn search(
        &self,
        term: &mut Term,
        context: &mut Context<'_>,
        depth: usize,
        path: &[usize],
        record: bool,
    ) -> Result<Search, Status> {
        if depth > context.config.max_depth {
            return Err(Status::Limit(Limit::Depth));
        }

        let mut blocked = None;

        if context.config.strategy == Strategy::Outermost {
            match self.attempt(term, context, depth)? {
                Attempt::Ready {
                    term: replacement,
                    rule,
                } => {
                    *term = self.contract(term, replacement, rule, context, path, record)?;

                    return Ok(Search::Changed {
                        exceeds_depth: !within_depth(term, depth, context.config.max_depth),
                    });
                }
                Attempt::Idle(reason) => blocked = reason,
            }
        }

        if let Term::App(name, args) = term {
            let count = self.evaluated_arguments(name, args);

            for (index, arg) in args.iter_mut().take(count).enumerate() {
                let child_path = if record && context.config.trace {
                    argument_path(path, index)
                } else {
                    Vec::new()
                };

                match self.search(arg, context, depth + 1, &child_path, record)? {
                    changed @ Search::Changed { .. } => return Ok(changed),
                    Search::Idle(reason) => {
                        if reason.is_some() {
                            blocked = reason;
                        }
                    }
                }
            }
        }

        if context.config.strategy == Strategy::Innermost {
            match self.attempt(term, context, depth)? {
                Attempt::Ready {
                    term: replacement,
                    rule,
                } => {
                    *term = self.contract(term, replacement, rule, context, path, record)?;

                    return Ok(Search::Changed {
                        exceeds_depth: !within_depth(term, depth, context.config.max_depth),
                    });
                }
                Attempt::Idle(reason) => {
                    if reason.is_some() {
                        blocked = reason;
                    }
                }
            }
        }

        Ok(Search::Idle(blocked))
    }

    pub(super) fn operational(
        &self,
        mut term: Term,
        context: &mut Context<'_>,
        depth: usize,
        path: &[usize],
        record: bool,
    ) -> (Term, Status) {
        if !within_depth(&term, depth, context.config.max_depth) {
            return (term, Status::Limit(Limit::Depth));
        }

        loop {
            match self.search(&mut term, context, depth, path, record) {
                Ok(Search::Changed {
                    exceeds_depth: true,
                }) => return (term, Status::Limit(Limit::Depth)),
                Ok(Search::Changed {
                    exceeds_depth: false,
                }) => {}
                Ok(Search::Idle(reason)) => {
                    return (term, reason.map_or(Status::NormalForm, Status::Stuck));
                }
                Err(status) => return (term, status),
            }
        }
    }
}
