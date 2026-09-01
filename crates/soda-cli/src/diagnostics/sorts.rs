use super::Context;
use soda_ast::Term;
use soda_diagnostics::{Diagnostic, Span, Suggestion, nearest};
use soda_sorts::SortErrorKind;

impl Context {
    pub fn sort_error(&self, error: &soda_sorts::SortError) -> Diagnostic {
        let (source, mut span) = self.locate(error.term.as_deref(), error.declaration.as_deref());
        let mut diagnostic = if let (Some(expected), Some(found)) = (&error.expected, &error.found)
        {
            Diagnostic::error("E1001", "this expression has the wrong sort")
                .primary(
                    source,
                    span,
                    format!("expected `{expected}`, found `{found}`"),
                )
                .help(format!(
                    "pass an expression of sort `{expected}` in this position"
                ))
        } else {
            let code = if error.kind == SortErrorKind::UnboundName {
                "E1101"
            } else if error.symbol.is_some() {
                "E1002"
            } else {
                "E1003"
            };

            Diagnostic::error(code, &error.message).primary(source, span, "in this expression")
        };

        if let Some(name) = error.declaration.as_deref().filter(|name| !name.is_empty()) {
            let kind = self
                .declaration_site(name)
                .map(|(_, site)| site.kind.as_str())
                .unwrap_or("definition");
            diagnostic.title = format!("{kind} `{name}`: {}", diagnostic.title);
        }

        if error.kind == SortErrorKind::DuplicateDeclaration {
            diagnostic.code = "E1004".into();
            if let Some(name) = error.declaration.as_deref() {
                let sites = self
                    .imported_sources
                    .iter()
                    .map(|(source, locations)| (*source, locations))
                    .chain(std::iter::once((self.source, &self.locations)))
                    .flat_map(|(source, locations)| {
                        locations
                            .declarations
                            .iter()
                            .map(move |site| (source, site))
                    })
                    .filter(|(_, site)| site.name.as_deref() == Some(name))
                    .collect::<Vec<_>>();

                if let Some((id, last)) = sites.last() {
                    diagnostic.labels[0].source = *id;
                    diagnostic.labels[0].span = last.name_span;
                }

                if sites.len() > 1 {
                    diagnostic = diagnostic.secondary(
                        sites[0].0,
                        sites[0].1.name_span,
                        "first declared here",
                    );
                }

                diagnostic
                    .help
                    .push("give this declaration a unique name or remove the duplicate".into());
            }

            return diagnostic;
        }

        if let Some(symbol) = &error.symbol {
            if error.term.is_none()
                && let Some(site) = error
                    .declaration
                    .as_deref()
                    .and_then(|name| self.source_locations(source).declaration(name))
                && let Some(file) = self.report.sources.get(source)
                && let Some(index) = file.text[site.span.start..site.span.end].find(symbol)
            {
                span = Span::new(
                    site.span.start + index,
                    site.span.start + index + symbol.len(),
                );
                diagnostic.labels[0].span = span;
            }

            if let Some(file) = self.report.sources.get(source)
                && let Some(index) = file
                    .text
                    .get(span.start..span.end)
                    .and_then(|text| text.find(symbol))
            {
                span = Span::new(span.start + index, span.start + index + symbol.len());
                diagnostic.labels[0].span = span;
            }

            let mut candidates = self
                .module
                .signatures
                .iter()
                .map(|signature| signature.name.as_str())
                .chain(
                    self.module
                        .judgments
                        .iter()
                        .map(|judgment| judgment.name.as_str()),
                )
                .chain(self.module.sorts.iter().map(|sort| sort.name.as_str()))
                .collect::<Vec<_>>();
            candidates.extend([
                "Int", "Bool", "Name", "add", "sub", "mul", "div", "mod", "eq", "lt", "le", "gt",
                "ge", "and", "or", "not", "if", "apply", "subst", "alpha_eq", "fresh",
            ]);

            if let Some(candidate) = nearest(symbol, candidates) {
                diagnostic.suggestions.push(Suggestion {
                    source,
                    span,
                    replacement: candidate.into(),
                    message: format!("did you mean `{candidate}`?"),
                });
            } else {
                diagnostic.help.push("declare this symbol with `constructor`, `function`, `judgment`, or `sort`, as appropriate".into());
            }
        }

        if error.kind == SortErrorKind::UnboundName {
            diagnostic.help.push("object names must be introduced by a binder, for example `bind(x: Int, @x)`; rule metavariables use `?x`".into());
        }

        if error.kind == SortErrorKind::Arity {
            diagnostic
                .help
                .push("match the argument count in the declared signature".into());
        }

        let locations = self.source_locations(source);

        if let Some(parent) = locations
            .terms
            .iter()
            .filter(|site| site.span.contains(span))
            .filter_map(|site| match &site.term {
                Term::App(name, _) => Some((site.span.end - site.span.start, name)),
                _ => None,
            })
            .filter(|(_, name)| {
                self.module
                    .signatures
                    .iter()
                    .any(|signature| signature.name == **name)
            })
            .min_by_key(|(size, _)| *size)
            && let Some((signature_source, site)) = self.declaration_site(parent.1)
        {
            diagnostic = diagnostic.secondary(
                signature_source,
                site.name_span,
                format!("`{}` is declared here", parent.1),
            );
        }

        diagnostic
    }
}
