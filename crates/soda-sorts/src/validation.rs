use crate::{SortError, TypeChecker, inference, proofs};
use soda_ast::{Equation, Module, Term};

/// Checks binder annotations iteratively to bound native stack use.
pub fn requires_sort_check(term: &Term) -> bool {
    let mut pending = vec![term];

    while let Some(term) = pending.pop() {
        match term {
            Term::Binder { .. } => return true,
            Term::App(_, arguments) => pending.extend(arguments),
            _ => {}
        }
    }

    false
}

pub(crate) fn bindings_require_sort_check(module: &Module) -> bool {
    let mut terms = Vec::new();

    for rule in &module.rules {
        terms.extend([&rule.lhs, &rule.rhs]);
        terms.extend(rule.condition.as_ref());
    }

    for equation in &module.equations {
        terms.extend([&equation.lhs, &equation.rhs]);
    }

    for proof in &module.proofs {
        terms.extend([&proof.lhs, &proof.rhs]);

        for step in &proof.steps {
            terms.extend(step.substitution.values());
        }
    }

    for clause in &module.clauses {
        terms.push(&clause.conclusion);
        terms.extend(&clause.premises);
    }

    for detail in &module.clause_details {
        terms.extend(&detail.conditions);
        for binding in &detail.bindings {
            terms.extend([&binding.pattern, &binding.value]);
        }
    }

    for induction in &module.inductions {
        terms.extend([&induction.lhs, &induction.rhs]);

        for case in &induction.cases {
            for step in &case.steps {
                terms.extend(step.substitution.values());
            }
        }
    }

    terms.extend(&module.evaluations);
    terms.extend(&module.queries);

    terms.into_iter().any(requires_sort_check)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TypeReport {
    pub typed: bool,
    pub checked_declarations: usize,
}

