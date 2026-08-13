use std::collections::BTreeSet;

use soda_ast::{Equation, Module, Proof, Term};

use crate::VerificationError;

pub const MAX_VALIDATION_DEPTH: usize = 256;

pub(crate) fn insert_name<'a>(
    names: &mut BTreeSet<&'a str>,
    name: &'a str,
) -> Result<(), VerificationError> {
    if name.trim().is_empty() {
        return Err(VerificationError::declaration(
            name,
            "declaration name is empty",
        ));
    }

    if !names.insert(name) {
        return Err(VerificationError::declaration(
            name,
            "duplicate declaration name",
        ));
    }

    Ok(())
}

pub(crate) fn validate_term(term: &Term, declaration: &str) -> Result<(), VerificationError> {
    let mut pending = vec![(term, 0)];

    while let Some((term, depth)) = pending.pop() {
        if depth > MAX_VALIDATION_DEPTH {
            return Err(VerificationError::declaration(
                declaration,
                format!("term exceeds validation depth limit of {MAX_VALIDATION_DEPTH}"),
            ));
        }

        match term {
            Term::Var(name) | Term::Name(name) if name.trim().is_empty() => {
                return Err(VerificationError::declaration(
                    declaration,
                    "variable name is empty",
                ));
            }
            Term::App(name, _) if name.trim().is_empty() => {
                return Err(VerificationError::declaration(
                    declaration,
                    "constructor name is empty",
                ));
            }
            Term::Binder {
                parameter,
                sort,
                body,
            } => {
                if parameter.trim().is_empty() || sort.trim().is_empty() {
                    return Err(VerificationError::declaration(
                        declaration,
                        "binder parameter and sort must be nonempty",
                    ));
                }

                pending.push((body, depth + 1));
            }
            Term::App(_, args) => pending.extend(args.iter().map(|arg| (arg, depth + 1))),
            _ => {}
        }
    }

    Ok(())
}

pub(crate) fn validate_proof_header<'a>(
    names: &mut BTreeSet<&'a str>,
    proof: &'a Proof,
) -> Result<(), VerificationError> {
    insert_name(names, &proof.name)?;
    validate_term(&proof.lhs, &proof.name)?;

    validate_term(&proof.rhs, &proof.name)
}

pub(crate) fn validate_equations(equations: &[Equation]) -> Result<(), VerificationError> {
    let mut names = BTreeSet::new();

    for equation in equations {
        insert_name(&mut names, &equation.name)?;
        validate_term(&equation.lhs, &equation.name)?;
        validate_term(&equation.rhs, &equation.name)?;
    }

    Ok(())
}

/// Checks declaration uniqueness, rule variable safety and the 256-edge depth limit.
/// Certificates must instantiate all variables on both sides of an axiom.
pub fn validate_module(module: &Module) -> Result<(), VerificationError> {
    if !module.imports.is_empty() {
        return Err(VerificationError::declaration(
            &module.name,
            "unresolved imports: load the source with the module loader before validation",
        ));
    }
    let mut names = BTreeSet::new();
    validate_equations(&module.equations)?;
    for equation in &module.equations {
        insert_name(&mut names, &equation.name)?;
    }

    for rule in &module.rules {
        insert_name(&mut names, &rule.name)?;
        validate_term(&rule.lhs, &rule.name)?;
        validate_term(&rule.rhs, &rule.name)?;
        if !matches!(rule.lhs, Term::App(_, _)) {
            return Err(VerificationError::declaration(
                &rule.name,
                "rule left side must be a constructor application",
            ));
        }

        let bound = rule.lhs.variables();
        let mut required = rule.rhs.variables();

        if let Some(condition) = &rule.condition {
            validate_term(condition, &rule.name)?;
            required.extend(condition.variables());
        }

        let unbound: Vec<_> = required.difference(&bound).cloned().collect();

        if !unbound.is_empty() {
            return Err(VerificationError::declaration(
                &rule.name,
                format!("unbound rule variables: {}", unbound.join(", ")),
            ));
        }
    }

    for proof in &module.proofs {
        validate_proof_header(&mut names, proof)?;
    }

    for clause in &module.clauses {
        insert_name(&mut names, &clause.name)?;
        validate_term(&clause.conclusion, &clause.name)?;
        if !matches!(clause.conclusion, Term::App(_, _)) {
            return Err(VerificationError::declaration(
                &clause.name,
                "inference rule conclusion must be a judgment constructor",
            ));
        }
        // Premise-only variables are existential witnesses in backward search.

        for premise in &clause.premises {
            validate_term(premise, &clause.name)?;
        }
    }

    let mut detailed = BTreeSet::new();

    for detail in &module.clause_details {
        let clause = module
            .clauses
            .iter()
            .find(|clause| clause.name == detail.clause)
            .ok_or_else(|| {
                VerificationError::declaration(
                    &detail.clause,
                    "inference metadata refers to an unknown clause",
                )
            })?;

        if !detailed.insert(&detail.clause) {
            return Err(VerificationError::declaration(
                &detail.clause,
                "duplicate inference metadata",
            ));
        }

        if detail.premise_labels.len() != clause.premises.len() {
            return Err(VerificationError::declaration(
                &detail.clause,
                "premise label count does not match premise count",
            ));
        }

        let mut labels = BTreeSet::new();

        for label in detail.premise_labels.iter().flatten() {
            if label.trim().is_empty() || !labels.insert(label) {
                return Err(VerificationError::declaration(
                    &detail.clause,
                    "empty or duplicate premise label",
                ));
            }
        }

        let mut variables = clause.conclusion.variables();

        for premise in &clause.premises {
            variables.extend(premise.variables());
        }

        let mut last = 0;
        for binding in &detail.bindings {
            if binding.before_premise > clause.premises.len() || binding.before_premise < last {
                return Err(VerificationError::declaration(
                    &detail.clause,
                    "computed bindings must follow premise order",
                ));
            }
            last = binding.before_premise;
            validate_term(&binding.pattern, &detail.clause)?;
            validate_term(&binding.value, &detail.clause)?;
            if !matches!(binding.pattern, Term::Var(_)) {
                return Err(VerificationError::declaration(
                    &detail.clause,
                    "computed binding target must be a metavariable",
                ));
            }
            variables.extend(binding.pattern.variables());
            variables.extend(binding.value.variables());
        }

        for condition in &detail.conditions {
            validate_term(condition, &detail.clause)?;

            if !condition.variables().is_subset(&variables) {
                return Err(VerificationError::declaration(
                    &detail.clause,
                    "side condition uses variables absent from conclusion and premises",
                ));
            }
        }
    }

    for term in &module.evaluations {
        validate_term(term, &module.name)?;
    }

    for query in &module.queries {
        validate_term(query, &module.name)?;
    }

    Ok(())
}
