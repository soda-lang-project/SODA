use crate::{ParseError, ParseErrorKind};
use soda_diagnostics::{Diagnostic, SourceId, Span, Suggestion, nearest};

impl ParseError {
    pub fn diagnostic(&self, source: SourceId, text: &str) -> Diagnostic {
        let start = self.offset.min(text.len());
        let end = text
            .get(start..)
            .and_then(|rest| rest.chars().next())
            .map_or(start, |ch| start + ch.len_utf8());
        let end = crate::lex(text)
            .ok()
            .and_then(|tokens| {
                tokens
                    .into_iter()
                    .find(|token| token.offset == start)
                    .map(|token| token.end)
            })
            .unwrap_or(end);
        let mut diagnostic = Diagnostic::error("E0001", "could not parse this source").primary(
            source,
            Span::new(start, end),
            &self.message,
        );

        if let ParseErrorKind::ExpectedToken(token) = &self.kind {
            diagnostic
                .help
                .push(format!("expected `{token}` before this token"));
            if token == ";" {
                let insertion = text[..start].trim_end().len();
                diagnostic.suggestions.push(Suggestion {
                    source,
                    span: Span::new(insertion, insertion),
                    replacement: ";".into(),
                    message: "terminate the preceding declaration with `;`".into(),
                });
            }

            if [")", "]", "}"].contains(&token.as_str()) {
                diagnostic
                    .notes
                    .push("check the opening delimiter and any commas in this expression".into());
            }
        }

        if self.kind == ParseErrorKind::IntegerRange {
            diagnostic.code = "E0002".into();
            diagnostic.help.push(format!(
                "integer literals range from {} to {}",
                i64::MIN,
                i64::MAX
            ));
        }

        if self.kind == ParseErrorKind::NestingLimit {
            diagnostic.code = "E0003".into();
            diagnostic
                .help
                .push("split this definition into smaller named functions or rules".into());
        }

        if self.kind == ParseErrorKind::UnterminatedString {
            diagnostic.code = "E0004".into();
            diagnostic.title = "this string has no closing quote".into();
            diagnostic
                .help
                .push("close the string with `\"`; escape an embedded quote as `\\\"`".into());
        }

        if let ParseErrorKind::DuplicateDeclaration(name) = &self.kind {
            diagnostic.code = "E0005".into();
            diagnostic.title = "this declaration name is already in use".into();
            if let Ok(tokens) = crate::lex(text)
                    && let Some(previous) = tokens.windows(2).find(|pair| pair[1].offset < start && pair[1].kind == crate::Kind::Ident(name.clone()) && matches!(&pair[0].kind, crate::Kind::Ident(kind) if ["rule", "axiom", "proof", "infer", "induction"].contains(&kind.as_str()))) {
                        diagnostic = diagnostic.secondary(source, Span::new(previous[1].offset, previous[1].end), "first declared here");
                    }

            diagnostic
                .help
                .push("choose a unique name for each rule, axiom, and proof".into());
        }
        // Use tokens so delimiters in comments and quoted notation are ignored.

        if let Ok(tokens) = crate::lex(text) {
            let mut stack = Vec::new();

            for token in tokens.iter().filter(|token| token.offset < start) {
                match token.kind {
                    crate::Kind::Symbol(ch @ ('(' | '[' | '{')) => {
                        stack.push((ch, Span::new(token.offset, token.end)))
                    }
                    crate::Kind::Symbol(')' | ']' | '}') => {
                        stack.pop();
                    }
                    _ => {}
                }
            }

            if let Some((opening, span)) = stack.last()
                && matches!(
                    self.kind,
                    ParseErrorKind::ExpectedSyntax | ParseErrorKind::ExpectedToken(_)
                )
            {
                diagnostic = diagnostic.secondary(
                    source,
                    *span,
                    format!("this `{opening}` delimiter is still open"),
                );
            }
        }

        if let ParseErrorKind::UnknownDeclaration(name) = &self.kind
            && let Some(candidate) = nearest(
                name,
                crate::DeclarationKind::TOP_LEVEL
                    .iter()
                    .map(|kind| kind.as_str()),
            )
            && let Some(index) = text[..start].rfind(name.as_str())
        {
            let span = Span::new(index, index + name.len());
            diagnostic.labels[0].span = span;
            diagnostic.suggestions.push(Suggestion {
                source,
                span,
                replacement: candidate.into(),
                message: format!("did you mean `{candidate}`?"),
            });
        }

        diagnostic
    }
}
