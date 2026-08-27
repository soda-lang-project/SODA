mod error;
pub use error::{InductionError, InductionErrorKind};
mod correspondence;
pub use correspondence::{Correspondence, check_correspondence};

use soda_ast::{Equation, InductionProof, Module, Proof, SignatureKind, Substitution, Term};
use soda_sorts::TypeChecker;
use soda_verifier::{verify_module, verify_proof};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedTheorem {
    pub name: String,
    pub cases: usize,
    pub steps: usize,
}

pub fn verify_inductions(module: &Module) -> Result<Vec<CheckedTheorem>, InductionError> {
    verify_module(module).map_err(|error| InductionError::new(error.to_string()))?;
    let checker =
        TypeChecker::new(module).map_err(|error| InductionError::new(error.to_string()))?;

    if !module.inductions.is_empty() && !checker.is_typed() {
        return Err("induction requires declared datatypes and signatures".into());
    }

    let mut equations = module.equations.clone();
    equations.extend(module.proofs.iter().map(|proof| Equation {
        name: proof.name.clone(),
        lhs: proof.lhs.clone(),
        rhs: proof.rhs.clone(),
    }));
    let mut names = module
        .rules
        .iter()
        .map(|rule| rule.name.clone())
        .chain(module.clauses.iter().map(|clause| clause.name.clone()))
        .chain(equations.iter().map(|equation| equation.name.clone()))
        .collect::<BTreeSet<_>>();
    let mut report = Vec::new();

    for induction in &module.inductions {
        if induction.name.trim().is_empty() || !names.insert(induction.name.clone()) {
            return Err(InductionError::new(format!(
                "duplicate or empty theorem {}",
                induction.name
            )));
        }

        report.push(
            check_induction(module, induction, &equations, &checker)
                .map_err(|error| error.in_declaration(&induction.name))?,
        );
        equations.push(Equation {
            name: induction.name.clone(),
            lhs: induction.lhs.clone(),
            rhs: induction.rhs.clone(),
        });
    }

    Ok(report)
}

fn check_induction(
    module: &Module,
    theorem: &InductionProof,
    equations: &[Equation],
    checker: &TypeChecker,
) -> Result<CheckedTheorem, InductionError> {
    let declaration = module
        .sorts
        .iter()
        .find(|sort| sort.name == theorem.sort)
        .ok_or_else(|| InductionError::new(format!("unknown induction sort {}", theorem.sort)))?;

    if !declaration.inductive {
        return Err(InductionError::new(format!(
            "{} is not an inductive datatype",
            theorem.sort
        )));
    }

    let constructors = module
        .signatures
        .iter()
        .filter(|signature| {
            signature.kind == SignatureKind::Constructor && signature.result == theorem.sort
        })
        .collect::<Vec<_>>();

    if constructors.is_empty() {
        return Err("induction on empty datatype is not supported".into());
    }

    let variables = theorem
        .lhs
        .variables()
        .union(&theorem.rhs.variables())
        .cloned()
        .collect::<BTreeSet<_>>();

    if !variables.contains(&theorem.variable) {
        return Err("induction variable must occur in the theorem".into());
    }

    checker
        .check_equation_with_variable_sort(
            &theorem.lhs,
            &theorem.rhs,
            &theorem.variable,
            &theorem.sort,
        )
        .map_err(|error| InductionError::new(error.to_string()))?;
    let expected = constructors
        .iter()
        .map(|signature| signature.name.as_str())
        .collect::<BTreeSet<_>>();
    let supplied = theorem
        .cases
        .iter()
        .map(|case| case.constructor.as_str())
        .collect::<BTreeSet<_>>();

    if expected != supplied || supplied.len() != theorem.cases.len() {
        return Err(InductionError::new(format!(
            "{}: cases must cover each constructor exactly once",
            theorem.name
        ))
        .with_kind(InductionErrorKind::IncompleteCases));
    }

    let mut step_count = 0;

    for case in &theorem.cases {
        let signature = constructors
            .iter()
            .find(|signature| signature.name == case.constructor)
            .unwrap();
        let arguments = case.arguments.iter().collect::<BTreeSet<_>>();

        if signature.arguments.len() != case.arguments.len()
            || arguments.len() != case.arguments.len()
            || case
                .arguments
                .iter()
                .any(|name| name.trim().is_empty() || variables.contains(name))
        {
            return Err(InductionError::new(format!(
                "{}: case arguments must be fresh, distinct, and match constructor arity",
                case.constructor
            )));
        }

        let constructor = Term::app(
            &case.constructor,
            case.arguments
                .iter()
                .map(|name| Term::Var(name.clone()))
                .collect(),
        );
        let instance = Substitution::from([(theorem.variable.clone(), constructor)]);
        let lhs = theorem.lhs.substitute(&instance);
        let rhs = theorem.rhs.substitute(&instance);
        checker
            .check_equation(&lhs, &rhs)
            .map_err(|error| InductionError::new(error.to_string()))?;
        let mut local = equations.to_vec();
        let mut anchors = BTreeMap::new();

        for (argument, sort) in case.arguments.iter().zip(&signature.arguments) {
            if sort == &theorem.sort {
                let name = format!("ih_{argument}");

                if local.iter().any(|equation| equation.name == name) {
                    return Err(InductionError::new(format!(
                        "local induction hypothesis {name} shadows a lemma"
                    )));
                }

                let binding =
                    Substitution::from([(theorem.variable.clone(), Term::Var(argument.clone()))]);
                local.push(Equation {
                    name: name.clone(),
                    lhs: theorem.lhs.substitute(&binding),
                    rhs: theorem.rhs.substitute(&binding),
                });
                anchors.insert(name, argument.clone());
            }
        }

        for step in &case.steps {
            if let Some(anchor) = anchors.get(&step.axiom)
                && step.substitution.get(anchor) != Some(&Term::Var(anchor.clone()))
            {
                return Err(InductionError::new(format!(
                    "{}: induction hypothesis must remain anchored to ?{anchor}",
                    step.axiom
                ))
                .with_kind(InductionErrorKind::UnanchoredHypothesis));
            }
        }

        let proof = Proof {
            name: format!("{}__case_{}", theorem.name, case.constructor),
            lhs,
            rhs,
            steps: case.steps.clone(),
        };

        let typed = Module {
            equations: local.clone(),
            proofs: vec![proof.clone()],
            inductions: Vec::new(),
            ..module.clone()
        };
        soda_sorts::validate_module(&typed)
            .map_err(|error| InductionError::new(error.to_string()))?;
        verify_proof(&local, &proof).map_err(|error| InductionError::new(error.to_string()))?;
        step_count += case.steps.len();
    }

    Ok(CheckedTheorem {
        name: theorem.name.clone(),
        cases: theorem.cases.len(),
        steps: step_count,
    })
}
