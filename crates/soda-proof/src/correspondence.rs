use soda_ast::Term;
use soda_domains::{DomainError, Domains, SemanticFunction};
use soda_semantics::{Config, Engine, Status};

/// Exhaustive comparison over the represented finite input domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Correspondence {
    Established {
        cases: usize,
    },
    Counterexample {
        input: Term,
        expected: Term,
        actual: Term,
        status: Status,
    },
    Unknown {
        input: Term,
        status: Status,
    },
}

/// Resource limits return `Unknown`; stuck terms and unequal values are counterexamples.
pub fn check_correspondence(
    engine: &Engine,
    domains: &Domains,
    function: &SemanticFunction,
    operation: &str,
    config: &Config,
) -> Result<Correspondence, DomainError> {
    if operation.trim().is_empty() {
        return Err("correspondence requires a nonempty operation name".into());
    }

    let input = domains.get(&function.input)?;
    let output = domains.get(&function.output)?;

    for argument in input.elements() {
        let expected = domains.apply(function, argument)?;
        let evaluation = engine.evaluate(&Term::app(operation, vec![argument.clone()]), config);

        if matches!(evaluation.status, Status::Limit(_)) {
            return Ok(Correspondence::Unknown {
                input: argument.clone(),
                status: evaluation.status,
            });
        }

        let equal = output
            .index(&evaluation.term)
            .zip(output.index(&expected))
            .is_some_and(|(actual, expected)| actual == expected);

        if evaluation.status != Status::NormalForm || !equal {
            return Ok(Correspondence::Counterexample {
                input: argument.clone(),
                expected,
                actual: evaluation.term,
                status: evaluation.status,
            });
        }
    }

    Ok(Correspondence::Established {
        cases: input.elements().len(),
    })
}
