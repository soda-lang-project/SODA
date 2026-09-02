use super::*;

pub(super) fn read_message(input: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut length = None;
    let mut header_bytes = 0;
    loop {
        let mut line = String::new();
        let read = (&mut *input).take(8193).read_line(&mut line)?;
        if read == 0 {
            return if header_bytes == 0 {
                Ok(None)
            } else {
                Err(io::ErrorKind::UnexpectedEof.into())
            };
        }
        header_bytes += read;
        if header_bytes > 8192 {
            return Err(io::Error::other("LSP header exceeds limit"));
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("Content-Length")
        {
            length = Some(value.trim().parse::<usize>().map_err(io::Error::other)?);
        }
    }
    let length = length
        .filter(|length| *length <= MAX_MESSAGE)
        .ok_or_else(|| io::Error::other("invalid LSP Content-Length"))?;
    let mut body = vec![0; length];
    input.read_exact(&mut body)?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(io::Error::other)
}
pub(super) fn send(output: &mut impl Write, value: Value) -> io::Result<()> {
    let body = serde_json::to_vec(&value)?;
    write!(output, "Content-Length: {}\r\n\r\n", body.len())?;
    output.write_all(&body)?;
    output.flush()
}
fn publish(output: &mut impl Write, uri: &str, doc: &Document) -> io::Result<()> {
    send(
        output,
        json!({"jsonrpc":"2.0", "method":"textDocument/publishDiagnostics",
        "params":{"uri":uri, "version":doc.version, "diagnostics":doc.diagnostics}}),
    )
}

pub(crate) fn serve() -> io::Result<()> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut documents: BTreeMap<String, Document> = BTreeMap::new();
    let mut initialized = false;
    let mut shutdown = false;
    while let Some(message) = read_message(&mut input)? {
        let method = message["method"].as_str().unwrap_or("");
        let params = &message["params"];
        let id = message.get("id").cloned();
        if method == "exit" {
            return if shutdown {
                Ok(())
            } else {
                Err(io::Error::other("exit before shutdown"))
            };
        }
        if let Some(id) = id {
            let result: Result<Value, (i32, String)> = if method == "initialize" && !initialized {
                initialized = true;
                Ok(
                    json!({"capabilities":{"positionEncoding":"utf-16", "textDocumentSync":{"openClose":true,"change":1,"save":true},
                    "completionProvider":{}, "hoverProvider":true, "definitionProvider":true, "referencesProvider":true,
                    "signatureHelpProvider":{"triggerCharacters":["(",","]}, "documentSymbolProvider":true,
                    "documentFormattingProvider":true, "foldingRangeProvider":true, "codeActionProvider":true},
                    "serverInfo":{"name":"soda", "version":env!("CARGO_PKG_VERSION")}}),
                )
            } else if !initialized {
                Err((-32002, "server not initialized".into()))
            } else if method == "shutdown" {
                shutdown = true;
                Ok(Value::Null)
            } else if shutdown || method == "initialize" {
                Err((-32600, "invalid lifecycle request".into()))
            } else if method == "soda/virtualDocument" {
                let uri = params["uri"].as_str().unwrap_or("");
                Ok(documents
                    .values()
                    .find_map(|d| d.virtual_sources.get(uri))
                    .map_or(Value::Null, |text| json!(text)))
            } else if !matches!(
                method,
                "textDocument/completion"
                    | "textDocument/hover"
                    | "textDocument/definition"
                    | "textDocument/references"
                    | "textDocument/signatureHelp"
                    | "textDocument/documentSymbol"
                    | "textDocument/foldingRange"
                    | "textDocument/formatting"
                    | "textDocument/codeAction"
            ) {
                Err((-32601, "method not found".into()))
            } else {
                let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
                if let Some(doc) = documents.get(uri) {
                    feature(method, params, doc, uri).map_err(|e| {
                        (
                            if e == "method not found" {
                                -32601
                            } else {
                                -32603
                            },
                            e,
                        )
                    })
                } else {
                    Err((-32602, "document is not open".into()))
                }
            };
            send(
                &mut output,
                match result {
                    Ok(result) => json!({"jsonrpc":"2.0", "id":id, "result":result}),
                    Err((code, message)) => {
                        json!({"jsonrpc":"2.0", "id":id, "error":{"code":code,"message":message}})
                    }
                },
            )?;
        } else if initialized && !shutdown {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            match method {
                "textDocument/didOpen" | "textDocument/didChange" => {
                    let text = if method.ends_with("didOpen") {
                        params["textDocument"]["text"].as_str()
                    } else {
                        params["contentChanges"]
                            .as_array()
                            .and_then(|c| c.last())
                            .and_then(|c| c["text"].as_str())
                    };
                    let version = params["textDocument"]["version"].as_i64().unwrap_or(0);
                    if let Some(text) = text
                        && documents.get(uri).is_none_or(|d| version > d.version)
                    {
                        let doc = analyze(uri, text.to_owned(), version);
                        publish(&mut output, uri, &doc)?;
                        documents.insert(uri.to_owned(), doc);
                    }
                }
                "textDocument/didClose" => {
                    documents.remove(uri);
                    send(
                        &mut output,
                        json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":uri,"diagnostics":[]}}),
                    )?;
                }
                "textDocument/didSave" | "workspace/didChangeWatchedFiles" => {
                    for (uri, doc) in &mut documents {
                        *doc = analyze(uri, doc.text.clone(), doc.version);
                        publish(&mut output, uri, doc)?;
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}
