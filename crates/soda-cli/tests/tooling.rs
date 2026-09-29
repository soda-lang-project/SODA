#[allow(dead_code)]
mod support;
use serde_json::{Value, json};
use std::io::{BufRead, Read, Write};

fn frame(value: Value) -> Vec<u8> {
    let body = serde_json::to_vec(&value).unwrap();
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend(body);
    frame
}

#[test]
fn stdio_lifecycle_unsaved_edits_features_and_diagnostic_clearing() {
    let uri = url::Url::from_file_path(std::env::current_dir().unwrap().join("test buffer.soda"))
        .unwrap()
        .to_string();
    let source = "module Test;\nsort Expr;\nfunction f(Int): Int;\nrule bad: f(true) => 1;\n";
    let mut child = support::command()
        .arg("lsp")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let requests = [
        json!({"id":1,"method":"initialize","params":{}}),
        json!({"method":"initialized","params":{}}),
        json!({"method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":3,"character":11}}}),
        json!({"id":3,"method":"textDocument/definition","params":{"textDocument":{"uri":uri},"position":{"line":3,"character":11}}}),
        json!({"id":4,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":{"line":3,"character":11}}}),
        json!({"id":5,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":uri}}}),
        json!({"id":6,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri},"options":{"tabSize":4,"insertSpaces":true}}}),
        json!({"method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":2},"contentChanges":[{"text":source.replace("f(true)", "f(1)")}]}}),
        json!({"method":"textDocument/didClose","params":{"textDocument":{"uri":uri}}}),
        json!({"id":7,"method":"shutdown"}),
        json!({"method":"exit"}),
    ];
    for mut message in requests {
        message["jsonrpc"] = json!("2.0");
        input.write_all(&frame(message)).unwrap();
    }
    drop(input);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let mut reader = std::io::Cursor::new(output.stdout);
    let mut messages = Vec::new();
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).unwrap() == 0 {
            break;
        }
        let length: usize = header
            .trim()
            .strip_prefix("Content-Length: ")
            .unwrap()
            .parse()
            .unwrap();
        header.clear();
        reader.read_line(&mut header).unwrap();
        assert_eq!(header, "\r\n");
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        messages.push(serde_json::from_slice::<Value>(&body).unwrap());
    }
    let response = |id| messages.iter().find(|m| m["id"] == id).unwrap()["result"].clone();
    assert_eq!(
        response(json!(1))["capabilities"]["positionEncoding"],
        "utf-16"
    );
    assert!(
        response(json!(2))
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["label"] == "f")
    );
    assert_eq!(response(json!(3))[0]["range"]["start"]["line"], 2);
    assert!(
        response(json!(4))["contents"]["value"]
            .as_str()
            .unwrap()
            .contains("function f")
    );
    assert!(response(json!(5)).as_array().unwrap().len() >= 3);
    assert!(response(json!(6)).is_array());
    let diagnostics = messages
        .iter()
        .filter(|m| m["method"] == "textDocument/publishDiagnostics")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 3);
    assert!(
        !diagnostics[0]["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        diagnostics[1]["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        diagnostics[2]["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

