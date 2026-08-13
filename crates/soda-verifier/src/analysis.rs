use std::collections::BTreeMap;

use crate::validation::validate_term;
use soda_ast::{Rule, Substitution, Term};

/// A result for syntactic rewriting; failure to prove termination means unknown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminationAnalysis {
    pub proven: bool,
    pub unproven_rules: Vec<String>,
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
