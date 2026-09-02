use super::*;

pub(super) fn analyze(uri: &str, text: String, version: i64) -> Document {
    let mut symbols = Vec::new();
    let file = file_path(uri).unwrap_or_else(|| std::path::PathBuf::from("<editor>"));
    let report = match Context::from_source(&file.to_string_lossy(), &text) {
        Err(mut report) => {
            // Recover complete declarations from this buffer; older ranges may be stale.

            let safe_end = lex(&text).err().map_or(text.len(), |e| e.offset);
            if let Ok(tokens) = lex(&text[..safe_end]) {
                let mut depth = 0usize;
                let mut boundaries = Vec::new();
                for token in tokens {
                    match token.kind {
                        Kind::Symbol('(' | '[' | '{') => depth += 1,
                        Kind::Symbol(')' | ']' | '}') => {
                            depth = depth.saturating_sub(1);
                            if depth == 0 && matches!(token.kind, Kind::Symbol('}')) {
                                boundaries.push(token.end);
                            }
                        }
                        Kind::Symbol(';') if depth == 0 => boundaries.push(token.end),
                        _ => {}
                    }
                }
                for end in boundaries
                    .into_iter()
                    .rev()
                    .filter(|end| *end < text.len())
                    .take(8)
                {
                    if let Ok(context) = Context::from_source(&file.to_string_lossy(), &text[..end])
                    {
                        symbols = collect_symbols(&context, uri);
                        let mut id = 0;
                        while let Some(source) = context.report.sources.get(id) {
                            if source.name.starts_with("<soda.") {
                                report.sources.add(&source.name, &source.text);
                            }
                            id += 1;
                        }
                        break;
                    }
                }
            }
            report
        }
        Ok(mut context) => {
            symbols = collect_symbols(&context, uri);
            if let Err(errors) = soda_sorts::validate_module_all(&context.module) {
                context.report.diagnostics = errors
                    .iter()
                    .take(100)
                    .map(|e| context.sort_error(e))
                    .collect();
            } else if let Err(error) = soda_verifier::verify_module(&context.module) {
                context
                    .report
                    .diagnostics
                    .push(context.verification(&error));
            } else {
                let failure = soda_proof::verify_inductions(&context.module)
                    .map(|_| ())
                    .map_err(crate::error::CommandError::from)
                    .and_then(|()| crate::domains::CheckedDomains::new(&context.module).map(|_| ()))
                    .and_then(|()| {
                        soda_semantics::Engine::new(context.module.rules.clone())
                            .map(|_| ())
                            .map_err(Into::into)
                    });
                if let Err(error) = failure {
                    return document_from_report(
                        uri,
                        text,
                        version,
                        symbols,
                        context.finish_error(error),
                    );
                }
            }
            context.report
        }
    };
    document_from_report(uri, text, version, symbols, report)
}

fn document_from_report(
    uri: &str,
    text: String,
    version: i64,
    symbols: Vec<Symbol>,
    report: Report,
) -> Document {
    let mut virtual_sources = BTreeMap::new();
    let mut id = 0;
    while let Some(source) = report.sources.get(id) {
        if let Some(uri) = source_uri(&source.name).filter(|u| u.starts_with("soda-stdlib:")) {
            virtual_sources.insert(uri, source.text.clone());
        }
        id += 1;
    }
    let mut diagnostics = Vec::new();
    let mut actions = Vec::new();
    for diagnostic in &report.diagnostics {
        let primary = diagnostic
            .labels
            .iter()
            .find(|l| l.primary)
            .or(diagnostic.labels.first());
        let mut message = diagnostic.title.clone();
        for label in &diagnostic.labels {
            message.push_str(&format!("\n{}", label.message));
        }
        for note in diagnostic.notes.iter().chain(&diagnostic.help) {
            message.push_str(&format!("\n{note}"));
        }
        let site = primary.and_then(|l| report.sources.get(l.source).map(|s| (s, l.span)));
        let own = site.is_none_or(|(s, _)| source_uri(&s.name).as_deref() == Some(uri));
        let diagnostic_range = if own {
            site.map(|(_, span)| range(&text, span))
                .unwrap_or_else(|| range(&text, Span::default()))
        } else {
            if let Some((source, _)) = site {
                message = format!("{}: {message}", source.name);
            }
            range(&text, Span::default())
        };
        let item = json!({"range": diagnostic_range, "severity": match diagnostic.severity {
            soda_diagnostics::Severity::Error => 1, soda_diagnostics::Severity::Warning => 2, _ => 3 },
            "source":"soda", "code":diagnostic.code, "message":message});
        for suggestion in &diagnostic.suggestions {
            if let Some(source) = report.sources.get(suggestion.source)
                && source_uri(&source.name).as_deref() == Some(uri)
            {
                let mut changes = serde_json::Map::new();
                changes.insert(uri.to_owned(), json!([{"range":range(&text, suggestion.span), "newText":suggestion.replacement}]));
                actions.push(json!({"title":suggestion.message, "kind":"quickfix", "diagnostics":[item.clone()],
                    "edit":{"changes":changes}}));
            }
        }
        diagnostics.push(item);
    }
    Document {
        virtual_sources,
        text,
        version,
        symbols,
        diagnostics,
        actions,
    }
}

fn collect_symbols(context: &Context, uri: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    let mut id = 0;
    while let Some(source) = context.report.sources.get(id) {
        let symbol_uri = if id == context.source {
            Some(uri.to_owned())
        } else {
            source_uri(&source.name)
        };
        if let Some(symbol_uri) = symbol_uri {
            for site in &context.source_locations(id).declarations {
                if let Some(name) = &site.name {
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: site.kind,
                        uri: symbol_uri.clone(),
                        range: range(&source.text, site.span),
                        selection: range(&source.text, site.name_span),
                        detail: source.text[site.span.start..site.span.end]
                            .lines()
                            .next()
                            .unwrap_or("")
                            .trim()
                            .to_owned(),
                        parameters: context
                            .module
                            .signatures
                            .iter()
                            .find(|s| s.name == *name)
                            .map(|s| s.arguments.clone())
                            .or_else(|| {
                                context
                                    .module
                                    .judgments
                                    .iter()
                                    .find(|s| s.name == *name)
                                    .map(|s| s.arguments.clone())
                            }),
                    });
                }
            }
        }
        id += 1;
    }
    symbols
}
