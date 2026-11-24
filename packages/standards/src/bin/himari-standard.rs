//! `himari-standard`: evaluate one standardised formula from a JSON or TOML request.
//!
//! Usage:
//!
//! ```text
//! himari-standard --input <path>        read the request from a file
//! himari-standard                       read the request from stdin
//! himari-standard --format json|toml    force the input format
//! himari-standard --list                print the supported standards and exit
//! ```
//!
//! The format is chosen in this order: `--format` if given, else the file extension
//! (`.json` / `.toml`), else the content (a leading `{` means JSON). Input from stdin is
//! never half-read by accident: a terminal on stdin is reported as an error rather than
//! waiting for an EOF that an interactive user did not intend.
//!
//! Output on success: the response document as pretty JSON on stdout, exit code 0.
//! Output on failure: the same stable-shape document with `errors` filled in and
//! `values` / `provenance` set to `null`, still as JSON on stdout (**non-empty stdout,
//! exit code 1**), plus a one-line diagnostic on stderr. Exit code 2 is reserved for
//! command-line misuse (unknown flag, unreadable file), where no request was evaluated.
//!
//! Requests may name any supported standard; see `--list` for the exact ids and editions.

use himari_standards::Format;
use std::io::{IsTerminal, Read};
use std::process::ExitCode;

const USAGE: &str = "usage: himari-standard [--input <path>] [--format json|toml] [--list]\n\
                     \x20      himari-standard [--format json|toml] < request.json";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut input: Option<String> = None;
    let mut format: Option<Format> = None;
    let mut list = false;

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--list" => list = true,
            "--help" | "-h" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "--input" | "-i" => {
                i += 1;
                let Some(path) = args.get(i) else {
                    eprintln!("himari-standard: --input needs a path");
                    return ExitCode::from(2);
                };
                if input.is_some() {
                    eprintln!("himari-standard: --input given more than once");
                    return ExitCode::from(2);
                }
                input = Some(path.clone());
            }
            "--format" | "-f" => {
                i += 1;
                let Some(name) = args.get(i) else {
                    eprintln!("himari-standard: --format needs json or toml");
                    return ExitCode::from(2);
                };
                match name.to_ascii_lowercase().as_str() {
                    "json" => format = Some(Format::Json),
                    "toml" => format = Some(Format::Toml),
                    other => {
                        eprintln!("himari-standard: --format must be json or toml, got {other:?}");
                        return ExitCode::from(2);
                    }
                }
            }
            other if other.starts_with("--input=") => {
                if input.is_some() {
                    eprintln!("himari-standard: --input given more than once");
                    return ExitCode::from(2);
                }
                input = Some(other["--input=".len()..].to_string());
            }
            other if other.starts_with("--format=") => {
                let name = &other["--format=".len()..];
                match name.to_ascii_lowercase().as_str() {
                    "json" => format = Some(Format::Json),
                    "toml" => format = Some(Format::Toml),
                    _ => {
                        eprintln!("himari-standard: --format must be json or toml, got {name:?}");
                        return ExitCode::from(2);
                    }
                }
            }
            other => {
                eprintln!("himari-standard: unknown argument {other:?}\n{USAGE}");
                return ExitCode::from(2);
            }
        }
        i += 1;
    }

    if list {
        if input.is_some() {
            eprintln!("himari-standard: --list takes no --input");
            return ExitCode::from(2);
        }
        let entries = himari_standards::list();
        match serde_json::to_string_pretty(&entries) {
            Ok(text) => {
                println!("{text}");
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("himari-standard: cannot encode the registry: {e}");
                return ExitCode::from(2);
            }
        }
    }

    // Resolve the input text.
    let (text, origin) = match &input {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(text) => (text, path.clone()),
            Err(e) => {
                eprintln!("himari-standard: cannot read {path}: {e}");
                return ExitCode::from(2);
            }
        },
        None => {
            if std::io::stdin().is_terminal() {
                eprintln!("himari-standard: no request given (stdin is a terminal)\n{USAGE}");
                return ExitCode::from(2);
            }
            let mut buf = String::new();
            if let Err(e) = std::io::stdin().read_to_string(&mut buf) {
                eprintln!("himari-standard: cannot read stdin: {e}");
                return ExitCode::from(2);
            }
            (buf, "<stdin>".to_string())
        }
    };

    // Resolve the format: explicit flag > file extension > content sniffing.
    let resolved = format.or_else(|| input.as_deref().and_then(Format::from_path_extension));

    match himari_standards::run_json(&text, resolved) {
        Ok(outcome) => {
            print_json(&outcome.document);
            ExitCode::SUCCESS
        }
        Err(err) => {
            let document = serde_json::json!({
                "standard": err.standard.map(|s| s.as_str()),
                "edition": err.edition,
                "provenance": serde_json::Value::Null,
                "values": serde_json::Value::Null,
                "warnings": serde_json::json!([]),
                "errors": [err],
            });
            print_json(&document);
            eprintln!("himari-standard: {}: {} ({origin})", err.code, err.message);
            ExitCode::from(1)
        }
    }
}

fn print_json(value: &serde_json::Value) {
    // Pretty JSON is the contract; serialization of a `Value` cannot fail in practice, but
    // a failure must still not panic.
    match serde_json::to_string_pretty(value) {
        Ok(text) => println!("{text}"),
        Err(e) => {
            eprintln!("himari-standard: cannot encode the response: {e}");
            std::process::exit(2);
        }
    }
}
