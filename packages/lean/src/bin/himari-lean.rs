//! `himari-lean` — command-line front end for the Lean co-process.
//!
//! ```text
//! himari-lean [--input <path>] [--print-source] [--emit <path>]
//! ```
//!
//! The request is read from `--input`, or from stdin when that flag is absent.
//! The result is a JSON document on stdout. The exit status is `0` **only** for a
//! verified run; see [`exit_code`] for the full table.
//!
//! `--print-source` writes the generated Lean source to stderr as well, so that
//! it can be reviewed or piped into a separate `lean` invocation without
//! disturbing the JSON on stdout.
//!
//! `--emit <path>` writes that same source to a file. This is the escape hatch
//! for hosts where the toolchain cannot be provisioned here: the obligation is a
//! self-contained `.lean` file that anyone with Lean can check with
//! `lean <path>` (see the crate README for the pinned version). The emitted file
//! is byte-identical to what the co-process would feed to stdin.

use std::io::{IsTerminal, Read, Write};
use std::process::ExitCode;

use himari_lean::report::Diagnostic;
use himari_lean::{verify_request, Report, RunOptions, Status};

const USAGE: &str = "\
himari-lean — generate and check a Lean 4 proof obligation

USAGE:
    himari-lean [--input <path>] [--print-source] [--emit <path>] [--no-verify]
    himari-lean --help | --version

INPUT:
    A JSON object on stdin, or the file named by --input:
        { \"theorem\": \"det3\", \"values\": [1,2,3,4,5,6,7,8,10] }
    Entries may also be decimal strings (\"1\", \"-2\"). Optional fields:
        literals       nine integers that override what the statement mentions
        expected       the determinant the caller believes is correct
        heartbeats     Lean -T budget (default 400000)
        memory_mb      Lean -M budget in MB (default 2048)
        timeout_secs   wall-clock cap this tool enforces (default 120)

EMIT:
    --emit <path> writes the generated Lean source to <path>. Check it with a
    real toolchain: `lean <path>` (pinned version in packages/lean/README.md).

OUTPUT:
    A JSON result on stdout whose \"status\" is one of:
        success, lean_missing, invalid_request, proof_failed,
        resource_exhausted, axiom_violation, oracle_mismatch

EXIT STATUS:
    0  success (the only verified outcome)
    2  usage error (bad arguments, unreadable --input)
    3  lean_missing       — no usable toolchain, nothing was checked
    4  invalid_request
    5  proof_failed
    6  resource_exhausted
    7  axiom_violation
    8  oracle_mismatch

ENVIRONMENT:
    HIMARI_LEAN_BIN   path to the `lean` executable to use instead of PATH
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut input: Option<String> = None;
    let mut print_source = false;
    let mut emit: Option<String> = None;
    let mut force_missing = false;

    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("himari-lean {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "--input" | "-i" => match it.next() {
                Some(path) => input = Some(path.clone()),
                None => return usage_error("--input requires a path"),
            },
            "--print-source" => print_source = true,
            "--emit" | "-o" => match it.next() {
                Some(path) => emit = Some(path.clone()),
                None => return usage_error("--emit requires a path"),
            },
            other if other.starts_with("--emit=") => {
                emit = Some(other["--emit=".len()..].to_string());
            }
            // Deliberately unadvertised: it exists so that downstream harnesses
            // can assert the missing-toolchain behaviour without uninstalling
            // anything. It can never turn a run into a success.
            "--no-verify" => force_missing = true,
            other if other.starts_with("--input=") => {
                input = Some(other["--input=".len()..].to_string());
            }
            other => return usage_error(&format!("unknown argument `{other}`")),
        }
    }

    let raw = match read_request(input.as_deref()) {
        Ok(raw) => raw,
        Err(msg) => return usage_error(&msg),
    };

    let mut opts = RunOptions {
        // The obligation must be emitted even when no toolchain is present: that
        // is exactly the case where a user needs the file to check it elsewhere.
        include_source: print_source || emit.is_some(),
        ..RunOptions::default()
    };
    if force_missing {
        opts = opts.without_lean();
    }

    let rep = verify_request(&raw, &opts);

    if print_source {
        if let Some(src) = &rep.source {
            let mut err = std::io::stderr().lock();
            let _ = writeln!(err, "# --- generated Lean source (stdout carries JSON) ---");
            let _ = err.write_all(src.as_bytes());
            let _ = writeln!(err, "# --- end of generated source ---");
        }
    }

    // The JSON goes to stdout even for refusals: a consumer that only reads
    // stdout still learns *why* the run did not verify. It is printed before the
    // file write so a failed write never leaves the caller with empty stdout.
    println!("{}", rep.to_json_pretty().trim_end());
    let _ = std::io::stdout().flush();

    let mut emit_failed = false;
    if let Some(path) = &emit {
        match &rep.source {
            Some(src) => {
                if let Err(e) = std::fs::write(path, src) {
                    eprintln!("himari-lean: cannot write `{path}`: {e}");
                    emit_failed = true;
                }
            }
            None => {
                // No source means the request never got far enough to generate
                // one; that is already visible in the JSON above.
                eprintln!("himari-lean: no source generated, `{path}` not written");
                emit_failed = true;
            }
        }
    }

    if emit_failed {
        return ExitCode::from(2);
    }
    ExitCode::from(exit_code(&rep))
}

/// Read the request text from a file or stdin.
fn read_request(path: Option<&str>) -> Result<String, String> {
    match path {
        Some(p) => std::fs::read_to_string(p).map_err(|e| format!("cannot read `{p}`: {e}")),
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| format!("cannot read stdin: {e}"))?;
            if buf.trim().is_empty() && std::io::stdin().is_terminal() {
                return Err("no request on stdin (pass --input <path> or pipe JSON)".into());
            }
            Ok(buf)
        }
    }
}

/// Exit code for a report, plus a human-readable line for non-successes.
fn exit_code(rep: &Report) -> u8 {
    let code = rep.status.exit_code();
    if rep.status == Status::Success {
        return code;
    }
    let first_error = rep
        .diagnostics
        .iter()
        .find(|d| d.level == "error")
        .map(|d: &Diagnostic| d.message.clone())
        .unwrap_or_else(|| "no diagnostic".to_string());
    eprintln!("himari-lean: {} ({code}) — {first_error}", rep.status);
    if rep.status == Status::LeanMissing {
        eprintln!("himari-lean: the theorem was NOT checked; this is not a verification.");
    }
    code
}

fn usage_error(msg: &str) -> ExitCode {
    eprintln!("himari-lean: {msg}");
    eprintln!("try `himari-lean --help`");
    ExitCode::from(2)
}
