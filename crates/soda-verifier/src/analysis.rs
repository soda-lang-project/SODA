use std::collections::BTreeMap;

use crate::validation::validate_term;
use soda_ast::{Rule, Substitution, Term};

/// A result for syntactic rewriting; failure to prove termination means unknown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminationAnalysis {
    pub proven: bool,
    pub unproven_rules: Vec<String>,
}

/// Proves unconditional rewriting decreases constructor/literal count without
/// increasing variable multiplicities. All symbols are uninterpreted.
pub fn analyze_termination(rules: &[Rule]) -> TerminationAnalysis {
    fn measure(term: &Term, variables: &mut BTreeMap<String, usize>) -> usize {
        match term {
            Term::Var(name) => {
                *variables.entry(name.clone()).or_default() += 1;

                0
            }
            Term::Binder { body, .. } => 1 + measure(body, variables),
            Term::App(_, args) => {
                1 + args
                    .iter()
                    .map(|arg| measure(arg, variables))
                    .sum::<usize>()
            }
            _ => 1,
        }
    }
    let unproven_rules = rules
        .iter()
        .filter(|rule| {
            if validate_term(&rule.lhs, &rule.name).is_err()
                || validate_term(&rule.rhs, &rule.name).is_err()
            {
                return true;
            }

            let mut left = BTreeMap::new();
            let mut right = BTreeMap::new();
            let left_size = measure(&rule.lhs, &mut left);
            let right_size = measure(&rule.rhs, &mut right);

            rule.condition.is_some()
                || left_size <= right_size
                || right
                    .iter()
                    .any(|(name, count)| left.get(name).unwrap_or(&0) < count)
        })
        .map(|rule| rule.name.clone())
        .collect::<Vec<_>>();

    TerminationAnalysis {
        proven: unproven_rules.is_empty(),
        unproven_rules,
    }
}

/// Unifies with an occurs check. Independent rules must be freshened first.
/// Binders must already be alpha-equivalent; solutions cannot depend on bound names.
pub fn unify(left: &Term, right: &Term) -> Option<Substitution> {
    let mut pending = vec![(left.clone(), right.clone())];
    let mut result = Substitution::new();

    while let Some((left, right)) = pending.pop() {
        let left = left.substitute(&result);
        let right = right.substitute(&result);

        if left.alpha_equivalent(&right) {
            continue;
        }

        match (left, right) {
            (Term::Var(name), term) | (term, Term::Var(name)) => {
                if term.variables().contains(&name) {
                    return None;
                }

                let binding = Substitution::from([(name.clone(), term.clone())]);

                for value in result.values_mut() {
                    *value = value.substitute(&binding);
                }

                result.insert(name, term);
            }
            (Term::App(left, xs), Term::App(right, ys))
                if left == right && xs.len() == ys.len() =>
            {
                pending.extend(xs.into_iter().zip(ys))
            }
            _ => return None,
        }
    }

    Some(result)
}
