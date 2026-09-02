use super::*;

fn word_at(doc: &Document, pos: &Value) -> Option<String> {
    let offset = offset(&doc.text, pos)?;
    let tokens = lex(&doc.text).ok()?;
    for (index, token) in tokens.iter().enumerate() {
        if token.offset <= offset
            && offset <= token.end
            && let Kind::Ident(name) = &token.kind
        {
            if index > 0 && matches!(tokens[index - 1].kind, Kind::Symbol('?' | '@')) {
                return None;
            }
            return Some(name.clone());
        }
    }
    None
}
fn symbol_kind(kind: DeclarationKind) -> u8 {
    match kind {
        DeclarationKind::Module => 2,
        DeclarationKind::Sort | DeclarationKind::Datatype | DeclarationKind::Domain => 5,
        DeclarationKind::Record => 23,
        DeclarationKind::Constructor => 9,
        _ => 12,
    }
}
pub(super) fn feature(
    method: &str,
    params: &Value,
    doc: &Document,
    uri: &str,
) -> Result<Value, String> {
    Ok(match method {
        "textDocument/completion" => {
            let mut items: BTreeMap<String, Value> = KEYWORDS.iter().map(|word| (word.to_string(), json!({"label":word, "kind":14}))).collect();
            for primitive in soda_ast::Primitive::ALL {
                items.insert(primitive.name().to_owned(), json!({"label":primitive.name(), "kind":3,
                    "detail":format!("builtin; {} arguments", primitive.arity())}));
            }
            for symbol in &doc.symbols {
                items.insert(symbol.name.clone(), json!({"label":symbol.name, "kind":3, "detail":symbol.detail}));
            }
            json!(items.into_values().collect::<Vec<_>>())
        }
        "textDocument/hover" => {
            let word = word_at(doc, &params["position"]);
            if let Some(symbol) = doc.symbols.iter().find(|s| Some(&s.name) == word.as_ref()) {
                json!({"contents":{"language":"soda", "value":symbol.detail}})
            } else if let Some(primitive) = word.as_deref().and_then(soda_ast::Primitive::from_name) {
                json!({"contents":{"kind":"plaintext", "value":format!("{}: builtin with {} arguments", primitive.name(), primitive.arity())}})
            } else { Value::Null }
        }
        "textDocument/definition" => {
            let word = word_at(doc, &params["position"]);
            json!(doc.symbols.iter().filter(|s| Some(&s.name) == word.as_ref())
                .map(|s| json!({"uri":s.uri, "range":s.selection})).collect::<Vec<_>>())
        }
        "textDocument/references" => {
            let word = word_at(doc, &params["position"]);
            let resolved = doc.symbols.iter().filter(|s| Some(&s.name) == word.as_ref()).collect::<Vec<_>>();
            if resolved.len() != 1 { json!([]) } else {
                let tokens = lex(&doc.text).map_err(|e| e.to_string())?;
                let include_declaration = params["context"]["includeDeclaration"].as_bool().unwrap_or(false);
                json!(tokens.iter().enumerate().filter_map(|(index, token)| {
                    if !matches!(&token.kind, Kind::Ident(name) if Some(name) == word.as_ref())
                        || index > 0 && matches!(tokens[index - 1].kind, Kind::Symbol('?' | '@')) { return None; }
                    let range = range(&doc.text, Span::new(token.offset, token.end));
                    if !include_declaration && resolved[0].uri == uri && range == resolved[0].selection { return None; }
                    Some(json!({"uri":uri,"range":range}))
                }).collect::<Vec<_>>())
            }
        }
        "textDocument/signatureHelp" => signature_help(doc, &params["position"]),
        "textDocument/documentSymbol" => json!(doc.symbols.iter().filter(|s| s.uri == uri)
            .map(|s| json!({"name":s.name, "kind":symbol_kind(s.kind), "range":s.range, "selectionRange":s.selection})).collect::<Vec<_>>()),
        "textDocument/foldingRange" => json!(doc.symbols.iter().filter(|s| s.uri == uri && s.range["end"]["line"].as_u64() > s.range["start"]["line"].as_u64())
            .map(|s| json!({"startLine":s.range["start"]["line"], "endLine":s.range["end"]["line"]})).collect::<Vec<_>>()),
        "textDocument/formatting" => {
            let formatted = soda_parser::format_source(&doc.text).map_err(|e| e.to_string())?;
            if formatted == doc.text { json!([]) } else {
                json!([{"range":range(&doc.text, Span::new(0, doc.text.len())), "newText":formatted}])
            }
        }
        "textDocument/codeAction" => json!(doc.actions.iter().filter(|action| {
            let start = action["diagnostics"][0]["range"]["start"]["line"].as_u64().unwrap_or(0);
            start >= params["range"]["start"]["line"].as_u64().unwrap_or(0)
                && start <= params["range"]["end"]["line"].as_u64().unwrap_or(u64::MAX)
        }).collect::<Vec<_>>()),
        _ => return Err("method not found".into()),
    })
}

fn signature_help(doc: &Document, pos: &Value) -> Value {
    let Some(cursor) = offset(&doc.text, pos) else {
        return Value::Null;
    };
    let Ok(tokens) = lex(&doc.text) else {
        return Value::Null;
    };
    let mut stack = Vec::new();
    for (index, token) in tokens
        .iter()
        .enumerate()
        .take_while(|(_, t)| t.offset < cursor)
    {
        match token.kind {
            Kind::Symbol('(' | '[' | '{') => {
                let name = if matches!(token.kind, Kind::Symbol('(')) && index > 0 {
                    if let Kind::Ident(name) = &tokens[index - 1].kind {
                        Some(name.as_str())
                    } else {
                        None
                    }
                } else {
                    None
                };
                stack.push((name, 0usize));
            }
            Kind::Symbol(')' | ']' | '}') => {
                stack.pop();
            }
            Kind::Symbol(',') => {
                if let Some((_, argument)) = stack.last_mut() {
                    *argument += 1;
                }
            }
            _ => {}
        }
    }
    let Some((Some(name), active)) = stack.into_iter().rev().find(|(name, _)| name.is_some())
    else {
        return Value::Null;
    };
    let signature = doc
        .symbols
        .iter()
        .find(|s| s.name == name && s.parameters.is_some())
        .map(|s| (s.detail.clone(), s.parameters.clone().unwrap()))
        .or_else(|| {
            soda_ast::Primitive::from_name(name).map(|p| {
                let args = (1..=p.arity())
                    .map(|i| format!("arg{i}"))
                    .collect::<Vec<_>>();
                (format!("{name}({})", args.join(", ")), args)
            })
        });
    match signature {
        Some((label, args)) => json!({"signatures":[{"label":label,
            "parameters":args.iter().map(|label| json!({"label":label})).collect::<Vec<_>>() }],
            "activeSignature":0,"activeParameter":active.min(args.len().saturating_sub(1))}),
        None => Value::Null,
    }
}
