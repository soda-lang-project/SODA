use std::collections::{BTreeMap, BTreeSet};

use soda_ast::{Equation, Module, Proof, Term};

use crate::validation::{validate_equations, validate_proof_header, validate_term};
use crate::{VerificationError, VerificationErrorKind, validate_module};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedProof {
    pub name: String,
    pub steps_checked: usize,
    pub conclusion: (Term, Term),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationReport {
    pub axioms_checked: usize,
    pub rules_checked: usize,
    pub proofs: Vec<VerifiedProof>,
}

pub fn verify_module(module: &Module) -> Result<VerificationReport, VerificationError> {
    validate_module(module)?;
    soda_sorts::validate_module(module)
        .map_err(|error| VerificationError::declaration(&module.name, error.to_string()))?;
    let mut equations = module.equations.clone();
    let mut proofs = Vec::new();

    for proof in &module.proofs {
        proofs.push(verify_proof(&equations, proof)?);
        equations.push(Equation {
            name: proof.name.clone(),
            lhs: proof.lhs.clone(),
            rhs: proof.rhs.clone(),
        });
    }

    Ok(VerificationReport {
        axioms_checked: module.equations.len(),
        rules_checked: module.rules.len(),
        proofs,
    })
}

/// Replays named axioms with symmetry, substitution and congruence.
pub fn verify_proof(
    equations: &[Equation],
    proof: &Proof,
) -> Result<VerifiedProof, VerificationError> {
    validate_equations(equations)?;
    let mut names = equations
        .iter()
        .map(|eq| eq.name.as_str())
        .collect::<BTreeSet<_>>();
    validate_proof_header(&mut names, proof)?;
    let axioms = equations
        .iter()
        .map(|eq| (eq.name.as_str(), eq))
        .collect::<BTreeMap<_, _>>();
    let mut current = proof.lhs.clone();

    for (index, step) in proof.steps.iter().enumerate() {
        let reject = |message| VerificationError::step(&proof.name, index, message);
        let axiom = axioms
            .get(step.axiom.as_str())
            .ok_or_else(|| reject(format!("unknown axiom: {}", step.axiom)))?;
        let mut required = axiom.lhs.variables();
        required.extend(axiom.rhs.variables());
        let supplied = step.substitution.keys().cloned().collect::<BTreeSet<_>>();

        if required != supplied {
            return Err(reject(format!(
                "substitution must bind exactly the axiom variables: {}",
                required.into_iter().collect::<Vec<_>>().join(", ")
            ))
            .with_kind(VerificationErrorKind::Substitution));
        }

        for value in step.substitution.values() {
            validate_term(value, &proof.name)?;
        }

        let (source, target) = if step.reverse {
            (&axiom.rhs, &axiom.lhs)
        } else {
            (&axiom.lhs, &axiom.rhs)
        };

        let source = source.substitute(&step.substitution);
        let selected = current.at(&step.path).ok_or_else(|| {
            reject(format!("invalid term position: {:?}", step.path))
                .with_kind(VerificationErrorKind::Position)
        })?;

        if !selected.alpha_equivalent(&source) {
            return Err(reject(format!(
                "axiom instance {source} does not match selected term {selected}"
            ))
            .comparison(selected.clone(), source));
        }

        current = current
            .replace(&step.path, target.substitute(&step.substitution))
            .ok_or_else(|| {
                reject("invalid replacement position".into())
                    .with_kind(VerificationErrorKind::Position)
            })?;
        validate_term(&current, &proof.name)?;
    }

    if !current.alpha_equivalent(&proof.rhs) {
        return Err(VerificationError::declaration(
            &proof.name,
            format!("certificate ends at {current}, expected {}", proof.rhs),
        )
        .comparison(proof.rhs.clone(), current));
    }

    Ok(VerifiedProof {
        name: proof.name.clone(),
        steps_checked: proof.steps.len(),
        conclusion: (proof.lhs.clone(), proof.rhs.clone()),
    })
}
