use super::{Attempt, Context, Engine, argument_path, within_depth};
use crate::{Config, Limit, Status, Strategy, TraceStep, builtin};
use soda_ast::Term;

impl Engine {
    /// Enumerates contractions at the root, then in left-to-right children.
    /// Each candidate gets its own guard budget; incomplete guards abort discovery.
    pub fn successors(&self, term: &Term, config: &Config) -> Result<Vec<TraceStep>, Status> {
        let config = Config {
            strategy: Strategy::Innermost,
            ..config.bounded()
        };

        if !within_depth(term, 0, config.max_depth) {
            return Err(Status::Limit(Limit::Depth));
        }

        let mut result = Vec::new();
        self.collect_successors(term, &config, 0, &[], &mut result)?;

        Ok(result)
    }

    pub(super) fn collect_successors(
        &self,
        term: &Term,
        config: &Config,
        depth: usize,
        path: &[usize],
        result: &mut Vec<TraceStep>,
    ) -> Result<(), Status> {
        if depth > config.max_depth {
            return Err(Status::Limit(Limit::Depth));
        }

        let builtin = matches!(term, Term::App(name, args) if builtin::handles(name,args,self.heads.contains("apply")));

        if builtin {
            let mut context = Context {
                config,
                steps: 0,
                trace: Vec::new(),
                last_rule: None,
            };

            if let Attempt::Ready {
                term: replacement,
                rule,
            } = self.attempt(term, &mut context, depth)?
            {
                let after =
                    self.contract(term, replacement, rule.clone(), &mut context, path, false)?;
                result.push(TraceStep {
                    rule: rule.to_string(),
                    path: path.to_vec(),
                    before: term.clone(),
                    after,
                });
            }
        } else {
            for rule in self.candidates(term) {
                let mut context = Context {
                    config,
                    steps: 0,
                    trace: Vec::new(),
                    last_rule: None,
                };

                if let Attempt::Ready {
                    term: replacement,
                    rule,
                } = self.attempt_rule(rule, term, &mut context, depth)?
                {
                    let after =
                        self.contract(term, replacement, rule.clone(), &mut context, path, false)?;
                    result.push(TraceStep {
                        rule: rule.to_string(),
                        path: path.to_vec(),
                        before: term.clone(),
                        after,
                    });
                }
            }
        }

        if let Term::App(name, args) = term {
            for (index, argument) in args
                .iter()
                .take(self.evaluated_arguments(name, args))
                .enumerate()
            {
                let child_path = argument_path(path, index);
                self.collect_successors(argument, config, depth + 1, &child_path, result)?;
            }
        }

        Ok(())
    }
}
