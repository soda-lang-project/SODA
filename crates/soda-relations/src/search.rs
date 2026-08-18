use super::{Condition, Derivation, Search, compose};
use soda_ast::{Substitution, Term};
use soda_verifier::unify;

impl Search<'_> {
    fn fresh_name(&mut self) -> String {
        loop {
            let candidate = format!("_soda_internal_{}", self.fresh);
            self.fresh += 1;
            // Fresh counters never repeat; only source and query names can collide.

            if !self.used.contains(&candidate) {
                return candidate;
            }
        }
    }

    fn rename(&mut self, index: usize) -> Substitution {
        self.relations.variables[index]
            .iter()
            .cloned()
            .map(|name| (name, Term::Var(self.fresh_name())))
            .collect()
    }

    pub(super) fn goal(
        &mut self,
        goal: &Term,
        env: &Substitution,
        depth: usize,
    ) -> Vec<(Substitution, Derivation)> {
        if depth > self.config.max_depth.min(super::MAX_DERIVATION_DEPTH)
            || self.config.max_answers == 0
        {
            self.complete = false;

            return Vec::new();
        }

        let mut results = Vec::new();

        let instantiated_goal = goal.substitute(env);

        for (index, clause) in self.relations.clauses.iter().enumerate() {
            if self.attempts >= self.config.max_attempts {
                self.complete = false;
                break;
            }

            self.attempts += 1;
            if matches!((&clause.conclusion, &instantiated_goal),
                (Term::App(a, xs), Term::App(b, ys)) if a != b || xs.len() != ys.len())
            {
                // Preserve attempt and fresh-name counts without building an impossible instance.

                for _ in 0..self.relations.variables[index].len() {
                    self.fresh_name();
                }

                continue;
            }

            let renaming = self.rename(index);
            let conclusion = clause.conclusion.substitute(&renaming);
            let Some(binding) = unify(&conclusion, &instantiated_goal) else {
                continue;
            };

            let mut states = vec![(compose(env, &binding), Vec::new())];

            for position in 0..=clause.premises.len() {
                if let Some(detail) = self.relations.details.get(&clause.name) {
                    for binding in detail
                        .bindings
                        .iter()
                        .filter(|binding| binding.before_premise == position)
                    {
                        let mut next = Vec::new();
                        for (state, children) in states {
                            let input = binding.value.substitute(&renaming).substitute(&state);
                            let remaining = self.config.max_attempts.saturating_sub(self.attempts);
                            let (value, steps) = self.relations.compute(&input, remaining);
                            self.attempts += steps;
                            let Some(value) = value else {
                                self.complete = false;
                                continue;
                            };
                            let target = binding.pattern.substitute(&renaming).substitute(&state);
                            if let Some(updated) = unify(&target, &value) {
                                next.push((compose(&state, &updated), children));
                            }
                        }
                        states = next;
                    }
                }
                let Some(premise) = clause.premises.get(position) else {
                    break;
                };
                let premise = premise.substitute(&renaming);
                let mut next = Vec::new();

                for (state, children) in states {
                    for (updated, proof) in self.goal(&premise, &state, depth + 1) {
                        let mut children = children.clone();
                        children.push(proof);
                        next.push((updated, children));

                        if next.len() >= self.config.max_answers {
                            self.complete = false;
                            break;
                        }
                    }

                    if next.len() >= self.config.max_answers {
                        break;
                    }
                }
                states = next;

                if states.is_empty() {
                    break;
                }
            }

            for (state, children) in states {
                let conditions = self
                    .relations
                    .details
                    .get(&clause.name)
                    .map(|detail| detail.conditions.as_slice())
                    .unwrap_or_default();
                let mut satisfied = true;

                for condition in conditions {
                    let instantiated = condition.substitute(&renaming).substitute(&state);
                    let remaining = self.config.max_attempts.saturating_sub(self.attempts);
                    let (outcome, steps) = self.relations.check_condition(&instantiated, remaining);
                    self.attempts += steps;

                    match outcome {
                        Condition::True => {}
                        Condition::False => {
                            satisfied = false;
                            break;
                        }
                        Condition::Unknown => {
                            self.complete = false;
                            satisfied = false;
                            break;
                        }
                    }
                }

                if !satisfied {
                    continue;
                }

                let substitution = renaming
                    .iter()
                    .map(|(name, value)| (name.clone(), value.substitute(&state)))
                    .collect();
                results.push((
                    state.clone(),
                    Derivation {
                        rule: clause.name.clone(),
                        conclusion: conclusion.substitute(&state),
                        substitution,
                        premises: children,
                    },
                ));

                if results.len() >= self.config.max_answers {
                    self.complete = false;

                    return results;
                }
            }
        }

        results
    }
}
