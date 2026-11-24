//! **End-to-end** stdio tests of `himari-lsp`: really spawn the process, really send
//! Content-Length frames and really read the responses back.
//!
//! The unit tests cover the protocol layer (`handle` is a pure function); here we cover the half
//! they cannot reach: framing, moving bytes over stdin/stdout, and interactive behavior such as
//! "an unknown method does not hang the client".

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

fn frame(payload: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload)
}

/// Read one LSP frame (header + body) from the child process's stdout.
fn read_frame(r: &mut BufReader<std::process::ChildStdout>) -> String {
    let mut len: Option<usize> = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line).unwrap_or(0) == 0 {
            panic!("子进程在响应前关闭了 stdout");
        }
        let t = line.trim_end();
        if t.is_empty() {
            break;
        }
        if let Some(v) = t.strip_prefix("Content-Length:") {
            len = v.trim().parse().ok();
        }
    }
    let len = len.expect("响应缺 Content-Length");
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).expect("读响应体");
    String::from_utf8_lossy(&buf).to_string()
}

#[test]
fn lsp_speaks_the_protocol_over_stdio() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_himari-lsp"))
        // The assertion below pins the English rendering; pin the child's locale so the test
        // passes regardless of the developer's shell.
        .env("HIMARI_LANG", "en")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("起 himari-lsp");
    let mut stdin = child.stdin.take().unwrap();
    let mut out = BufReader::new(child.stdout.take().unwrap());

    // 1) initialize → capabilities
    stdin
        .write_all(
            frame(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#).as_bytes(),
        )
        .unwrap();
    stdin.flush().unwrap();
    let init = read_frame(&mut out);
    assert!(init.contains("textDocumentSync"), "{init}");
    assert!(init.contains("completionProvider"), "{init}");

    // 2) didOpen (with an error) → diagnostics pushed
    let bad = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///t.formula","text":"decl { x : d; }\nx + undeclared\n"}}}"#;
    stdin.write_all(frame(bad).as_bytes()).unwrap();
    stdin.flush().unwrap();
    let diag = read_frame(&mut out);
    assert!(diag.contains("publishDiagnostics"), "{diag}");
    // The child runs with HIMARI_LANG=en (pinned on spawn): the assertion is the English
    // rendering of the stable `undeclared-symbol` code.
    assert!(diag.contains("undeclared symbol"), "{diag}");

    // 3) Unknown method (with an id) → replies null and does not hang
    stdin
        .write_all(frame(r#"{"jsonrpc":"2.0","id":2,"method":"workspace/didChangeConfiguration","params":{}}"#).as_bytes())
        .unwrap();
    stdin.flush().unwrap();
    let unknown = read_frame(&mut out);
    assert!(unknown.contains("null"), "{unknown}");

    // 4) shutdown → null, then exit makes the process quit
    stdin
        .write_all(
            frame(r#"{"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}"#).as_bytes(),
        )
        .unwrap();
    stdin.flush().unwrap();
    let shutdown = read_frame(&mut out);
    assert!(shutdown.contains("result"), "{shutdown}");

    stdin
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit"}"#).as_bytes())
        .unwrap();
    stdin.flush().unwrap();
    drop(stdin);

    let status = child.wait().expect("等子进程退出");
    assert!(status.success(), "exit 之后应以 0 退出：{status:?}");
}
