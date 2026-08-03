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

/// Shares constraints within each definition, keeping quantified definitions separate.
pub fn validate_module(module: &Module) -> Result<TypeReport, SortError> {
    validate_module_all(module).map_err(|errors| {
        errors
            .into_iter()
            .next()
            .expect("a failed validation has an error")
    })
}

pub fn validate_module_all(module: &Module) -> Result<TypeReport, Vec<SortError>> {
    let checker = TypeChecker::new(module).map_err(|error| vec![error])?;
    let mut report = TypeReport {
        typed: checker.is_typed(),
        checked_declarations: 0,
    };

    if !report.typed {
        return Ok(report);
    }

    let mut errors = Vec::new();
    let mut record = |name: &str, result: Result<(), SortError>| {
        report.checked_declarations += 1;

        if let Err(error) = result {
            errors.push(error.in_declaration(name));
        }
    };

    for rule in &module.rules {
        let result = (|| {
            if let Term::App(name, _) = &rule.lhs
                && checker.inductive_constructors.contains(name)
            {
                return Err(SortError::new(format!(
                    "rewrite rule `{}` cannot redefine inductive constructor `{name}`",
                    rule.name
                ))
                .at(&rule.lhs));
            }

            let mut scope = inference::Scope::new(&checker);
            scope.equation(&rule.lhs, &rule.rhs)?;
            if let Some(condition) = &rule.condition {
                scope.boolean(condition)?;
            }

            scope.finish()
        })();
        record(&rule.name, result);
    }

    for equation in &module.equations {
        record(
            &equation.name,
            checker.check_equation(&equation.lhs, &equation.rhs),
        );
    }

    let mut equations = module.equations.clone();

    for proof in &module.proofs {
        record(&proof.name, proofs::check(&checker, &equations, proof));
        equations.push(Equation {
            name: proof.name.clone(),
            lhs: proof.lhs.clone(),
            rhs: proof.rhs.clone(),
        });
    }

    for clause in &module.clauses {
        let result = (|| {
            let mut scope = inference::Scope::new(&checker);
            scope.judgment(&clause.conclusion)?;
            for premise in &clause.premises {
                scope.judgment(premise)?;
            }

            for detail in module
                .clause_details
                .iter()
                .filter(|detail| detail.clause == clause.name)
            {
                for binding in &detail.bindings {
                    scope.equation(&binding.pattern, &binding.value)?;
                }
                for condition in &detail.conditions {
                    scope.boolean(condition)?;
                }
            }

            scope.finish()
        })();
        record(&clause.name, result);
    }

    for term in &module.evaluations {
        record("", checker.check_term(term).map(|_| ()));
    }

    for query in &module.queries {
        record("", checker.check_judgment(query));
    }

    if errors.is_empty() {
        Ok(report)
    } else {
        Err(errors)
    }
}
