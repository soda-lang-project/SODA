use super::{Domains, FiniteDomain, Shape};
use crate::DomainError;
use soda_ast::Term;
use std::collections::BTreeMap;

/// Bindings retain their domain identity and distinguish names from metavariables.
#[derive(Clone, Debug, Default)]
pub struct Environment {
    bindings: BTreeMap<String, (FiniteDomain, Term)>,
}

impl Environment {
    pub fn bind(
        &mut self,
        domains: &Domains,
        name: &str,
        domain: &str,
        value: &Term,
    ) -> Result<(), DomainError> {
        if name.trim().is_empty() {
            return Err("environment name must be nonempty".into());
        }

        let domain = domains.get(domain)?;
        let index = domain.require_index(value)?;
        self.bindings.insert(
            name.into(),
            (domain.clone(), domain.elements[index].clone()),
        );

        Ok(())
    }

    pub fn interpret(
        &self,
        domains: &Domains,
        expected_domain: &str,
        term: &Term,
    ) -> Result<Term, DomainError> {
        fn interpret(
            environment: &Environment,
            domain: &FiniteDomain,
            term: &Term,
        ) -> Result<Term, DomainError> {
            if let Term::Name(name) = term {
                let (snapshot, value) = environment
                    .bindings
                    .get(name)
                    .ok_or_else(|| DomainError::new(format!("unbound environment name @{name}")))?;

                if snapshot != domain {
                    return Err(DomainError::new(format!(
                        "@{name} belongs to domain {}, expected {}",
                        snapshot.name, domain.name
                    )));
                }

                return Ok(value.clone());
            }

            let interpreted = match (&domain.shape, term) {
                (Shape::Product(left, right), Term::App(head, args))
                    if head == "pair" && args.len() == 2 =>
                {
                    Term::app(
                        "pair",
                        vec![
                            interpret(environment, left, &args[0])?,
                            interpret(environment, right, &args[1])?,
                        ],
                    )
                }
                (Shape::Lifted(base), Term::App(head, args))
                    if head == "lift" && args.len() == 1 =>
                {
                    Term::app("lift", vec![interpret(environment, base, &args[0])?])
                }
                (Shape::Function(input, output), Term::App(head, args))
                    if head == "table" && args.len() == input.elements.len() =>
                {
                    Term::app(
                        "table",
                        args.iter()
                            .map(|value| interpret(environment, output, value))
                            .collect::<Result<Vec<_>, _>>()?,
                    )
                }
                _ => term.clone(),
            };

            Ok(domain.elements[domain.require_index(&interpreted)?].clone())
        }
        interpret(self, domains.get(expected_domain)?, term)
    }

    pub fn lookup(&self, name: &str) -> Option<(&str, &Term)> {
        self.bindings
            .get(name)
            .map(|(domain, value)| (domain.name.as_str(), value))
    }
}
