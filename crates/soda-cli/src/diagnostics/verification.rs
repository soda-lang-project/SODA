use super::Context;
use soda_ast::Term;
use soda_diagnostics::{Diagnostic, Report, Span, nearest};

impl Context {
    pub fn verification(&self, error: &soda_verifier::VerificationError) -> Diagnostic {
        let located = self.declaration_site(&error.declaration);
        let source = located.map(|(source, _)| source).unwrap_or(self.source);
        let site = located.map(|(_, site)| site);
        let locations = self.source_locations(source);
        let span = match (site, error.step) {
            (Some(site), Some(step)) => locations
                .steps
                .iter()
                .filter(|span| site.span.contains(**span))
                .nth(step)
                .copied()
                .unwrap_or(site.name_span),
            (Some(site), _) => site.name_span,
            _ => Span::new(0, 0),
        };

        let mut diagnostic =
            Diagnostic::error("E2001", format!("could not verify `{}`", error.declaration))
                .primary(source, span, &error.message);

        if let (Some(expected), Some(found)) = (&error.expected, &error.found) {
            diagnostic.labels[0].message =
                "this step does not establish the required equality".into();
            diagnostic.notes.push(format!("expected: {expected}"));
            diagnostic.notes.push(format!("found:    {found}"));
        }

        if let Some(step) = error.step {
            diagnostic.notes.push(format!(
                "certificate step {} was rejected; earlier steps were checked successfully",
                step + 1
            ));

            if let Some(proof) = self
                .module
                .proofs
                .iter()
                .find(|proof| proof.name == error.declaration)
                && let Some(step) = proof.steps.get(step)
            {
                if let Some(expected) = &error.expected {
                    let equation = self
                        .module
                        .equations
                        .iter()
                        .find(|equation| equation.name == step.axiom);

                    if let Some(equation) = equation {
                        let pattern = if step.reverse {
                            &equation.rhs
                        } else {
                            &equation.lhs
                        };

                        if let Some(bindings) = soda_ast::match_term(pattern, expected) {
                            for (name, actual) in &step.substitution {
                                if bindings
                                    .get(name)
                                    .is_some_and(|expected| !expected.alpha_equivalent(actual))
                                {
                                    if let Some(span) = locations.term(actual, Some(span)) {
                                        diagnostic.labels[0].span = span;
                                    }
                                    diagnostic.labels[0].message =
                                        format!("`?{name}` has the wrong instance here");
                                    diagnostic.help.push(format!("instantiate `?{name}` with `{}` to match the selected subterm", bindings[name]));
                                    break;
                                }
                            }
                        }
                    }
                }

                if let Some((axiom_source, axiom)) = self.declaration_site(&step.axiom) {
                    diagnostic = diagnostic.secondary(
                        axiom_source,
                        axiom.name_span,
                        "the referenced equation is declared here",
                    );
                } else if let Some(candidate) = nearest(
                    &step.axiom,
                    self.module
                        .equations
                        .iter()
                        .map(|equation| equation.name.as_str())
                        .chain(self.module.proofs.iter().map(|proof| proof.name.as_str())),
                ) {
                    diagnostic.help.push(format!("did you mean `{candidate}`?"));
                }
            }
        }

        if error.kind == soda_verifier::VerificationErrorKind::Substitution {
            diagnostic.help.push(
                "supply `with {x = term, ...}` binding exactly every metavariable in the equation"
                    .into(),
            );
        }

        if error.kind == soda_verifier::VerificationErrorKind::Position {
            diagnostic.help.push(
                "paths are zero-based child indices; `at []` selects the whole expression".into(),
            );
        }

        diagnostic
    }

    pub fn finish_error(mut self, error: crate::error::CommandError) -> Report {
        let message = error.to_string();

        if !self.handled_error {
            // Rule validation errors carry the rule name in their message.

            let prefix = error.declaration().or_else(|| match &error {
                crate::error::CommandError::Validation(message) => message
                    .strip_prefix("rule ")
                    .and_then(|rest| rest.split_whitespace().next()),
                _ => None,
            });
            let located = prefix.and_then(|name| self.declaration_site(name));
            let source = located.map(|(source, _)| source).unwrap_or(self.source);
            let site = located.map(|(_, site)| site);
            use crate::error::CommandError;
            use soda_domains::DomainErrorKind;
            use soda_proof::InductionErrorKind;
            let (code, title, help) = match &error {
                CommandError::Domain(error) => match error.kind {
                    DomainErrorKind::NonMonotoneMap => (
                        "E3002",
                        "this semantic map does not preserve the domain order",
                        "if a <= b, a monotone map must satisfy f(a) <= f(b)",
                    ),
                    DomainErrorKind::NonTotalMap => (
                        "E3001",
                        "this semantic map is missing inputs",
                        "provide one output for every domain element, including `bottom`",
                    ),
                    DomainErrorKind::InvalidDefinition => (
                        "E3000",
                        "could not construct this denotational definition",
                        "check domain dependencies, map membership, and the finite-domain resource bounds",
                    ),
                },
                CommandError::Induction(error) => match error.kind {
                    InductionErrorKind::IncompleteCases => (
                        "E2101",
                        "this induction does not cover every constructor",
                        "add one case for each constructor, with no duplicates",
                    ),
                    InductionErrorKind::UnanchoredHypothesis => (
                        "E2102",
                        "this induction hypothesis was applied to the wrong term",
                        "an induction hypothesis applies only to its immediate recursive constructor field",
                    ),
                    InductionErrorKind::InvalidDefinition => (
                        "E0200",
                        "validation failed",
                        "check the highlighted definition and its referenced declarations",
                    ),
                },
                CommandError::Validation(_) | CommandError::Engine(_) => (
                    "E0200",
                    "validation failed",
                    "check the highlighted definition and its referenced declarations",
                ),
            };

            let reason = error.message();
            let mut diagnostic = if code == "E0200" {
                let title = if let Some(site) = site {
                    format!(
                        "{} `{}` failed validation: {reason}",
                        site.kind,
                        site.name.as_deref().unwrap_or_default()
                    )
                } else {
                    message.clone()
                };

                Diagnostic::error(code, title)
            } else {
                Diagnostic::error(code, title).note(&message).help(help)
            };

            if let Some(site) = site {
                diagnostic = diagnostic.primary(source, site.name_span, reason);
            } else if let Some((source, locations)) = &self.argument
                && let Some(site) = locations
                    .terms
                    .iter()
                    .max_by_key(|site| site.span.end - site.span.start)
            {
                diagnostic = diagnostic.primary(*source, site.span, &message);
            }

            if let Some(theorem) = prefix.and_then(|name| {
                self.module
                    .inductions
                    .iter()
                    .find(|theorem| theorem.name == name)
            }) {
                if matches!(&error, CommandError::Induction(error) if error.kind == InductionErrorKind::IncompleteCases)
                {
                    let missing = self
                        .module
                        .signatures
                        .iter()
                        .filter(|signature| {
                            signature.kind == soda_ast::SignatureKind::Constructor
                                && signature.result == theorem.sort
                                && !theorem
                                    .cases
                                    .iter()
                                    .any(|case| case.constructor == signature.name)
                        })
                        .map(|signature| signature.name.as_str())
                        .collect::<Vec<_>>();

                    if !missing.is_empty() {
                        diagnostic
                            .notes
                            .push(format!("missing cases: {}", missing.join(", ")));
                    }
                }

                if matches!(&error, CommandError::Induction(error) if error.kind == InductionErrorKind::UnanchoredHypothesis)
                {
                    let mut index = 0;

                    'cases: for case in &theorem.cases {
                        for step in &case.steps {
                            if let Some(anchor) = step.axiom.strip_prefix("ih_")
                                && step.substitution.get(anchor) != Some(&Term::Var(anchor.into()))
                            {
                                if let Some(site) = site
                                    && let Some(span) = self
                                        .source_locations(source)
                                        .steps
                                        .iter()
                                        .filter(|span| site.span.contains(**span))
                                        .nth(index)
                                {
                                    diagnostic.labels[0].span = *span;
                                }

                                diagnostic.help.push(format!(
                                    "keep the recursive field fixed with `{{{anchor} = ?{anchor}}}`"
                                ));
                                break 'cases;
                            }

                            index += 1;
                        }
                    }
                }
            }

            self.report.diagnostics.push(diagnostic);
        }

        self.report
    }
}
