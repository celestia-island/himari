//! `lean_missing` behaviour: the state this crate must never confuse with a proof.
//!
//! These tests force the missing state (they do not depend on whether Lean is
//! installed on the machine), then assert three things:
//!
//! 1. the status is `lean_missing`, not `success` and not a vague error;
//! 2. the process exit code is non-zero;
//! 3. the result still says what *was* established — here, only the Rust oracle's
//!    value — and explicitly fails `verified`.
//!
//! The toolchain is discovered through the environment only, so the missing state
//! is produced three ways here — an empty `PATH`, a `HIMARI_LEAN_BIN` that points
//! at something unusable, and the CLI's `--no-verify` switch — and all three must
//! land in the same non-success. None of them can be reached from a request.

mod common;

use common::{run_cli, run_cli_with_env};
use himari_lean::runner::LeanToolchain;
use serde_json::Value;

const MILESTONE: &str = r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10]}"#;

#[test]
fn missing_lean_reports_a_non_success_and_never_verified() {
    // `--no-verify` guarantees the missing state regardless of what is installed,
    // so this test asserts the same thing on every machine.
    let out = run_cli(None, &["--no-verify"], Some(MILESTONE));
    let v = out.json();
    assert_eq!(v["status"], "lean_missing", "{}", out.stderr);
    assert_eq!(v["verified"], false);
    assert_ne!(out.code, 0, "a non-success must not exit 0");
    assert_eq!(v["exit_code"], out.code);
    // Nothing was checked, so there is no Lean version and no axiom report.
    assert_eq!(v["lean"]["version"], Value::Null);
    assert_eq!(v["axioms"]["parsed"], false);
    // The oracle still ran: it does not need Lean.
    assert_eq!(v["oracle"]["det3"], "-3");
    // The failure is explained, not silent.
    let codes: Vec<&str> = v["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap_or(""))
        .collect();
    assert!(codes.contains(&"lean_missing"), "{codes:?}");
    assert!(out.stderr.contains("NOT checked"), "{}", out.stderr);
}

/// The same assertion for a machine with no Lean and no switches at all: with an
/// empty `PATH` and no override, discovery must end in `lean_missing`.
#[test]
fn without_any_toolchain_the_run_never_claims_success() {
    let out = run_cli_with_env(None, &[], Some(MILESTONE), &[("PATH", "")]);
    let v = out.json();
    assert_eq!(v["status"], "lean_missing", "{}", out.stderr);
    assert_eq!(v["verified"], false);
    assert_ne!(out.code, 0);
    let searched = v["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["code"] == "lean_missing")
        .and_then(|d| d["message"].as_str())
        .unwrap_or("");
    assert!(searched.contains("NOT checked"), "{searched}");
}

#[test]
fn no_verify_switch_forces_the_missing_state() {
    let out = run_cli(None, &["--no-verify"], Some(MILESTONE));
    let v = out.json();
    assert_eq!(v["status"], "lean_missing");
    assert_eq!(v["verified"], false);
    assert_eq!(out.code, 3);
    assert_eq!(v["exit_code"], 3);
}

#[test]
fn a_broken_override_is_missing_not_a_panic() {
    let dir = common::TempDir::new("broken-override");
    let candidate = dir.path().join("not-lean");
    std::fs::write(&candidate, b"#!/bin/sh\nexit 1\n").unwrap();
    // A file that exists but cannot answer `--version` must not be trusted.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = std::fs::metadata(&candidate).unwrap().permissions();
        p.set_mode(0o755);
        std::fs::set_permissions(&candidate, p).unwrap();
    }
    let out = run_cli(Some(&candidate), &[], Some(MILESTONE));
    let v = out.json();
    assert_eq!(v["status"], "lean_missing", "{}{}", out.stdout, out.stderr);
    assert_ne!(out.code, 0);
    let messages: String = v["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["message"].as_str().unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" | ");
    assert!(
        messages.contains("HIMARI_LEAN_BIN") || messages.contains("failed"),
        "{messages}"
    );
}

#[test]
fn generated_source_is_still_reviewable_when_lean_is_missing() {
    let out = run_cli(None, &["--no-verify", "--print-source"], Some(MILESTONE));
    let v = out.json();
    assert_eq!(v["status"], "lean_missing");
    // The obligation exists and is hashed even though nothing checked it.
    let statement = v["theorem"]["statement"].as_str().expect("statement");
    assert_eq!(
        statement,
        "theorem det3_example : det3 1 2 3 4 5 6 7 8 10 = -3 := by decide"
    );
    let sha = v["theorem"]["source_sha256"].as_str().expect("sha256");
    assert_eq!(sha.len(), 64, "sha256 hex digest has 64 chars");
    // The source goes to stderr so that stdout stays machine-readable JSON.
    assert!(out.stderr.contains("def det3"), "{}", out.stderr);
    assert!(out.stderr.contains("#print axioms det3_example"));
    assert!(out.stdout.contains("\"schema\""), "stdout must stay JSON");
}

#[test]
fn the_reported_hash_is_of_the_exact_source_lean_receives() {
    // The stub below behaves as "Lean said yes, axioms are fine". The point of
    // the test is the hash, not the stub's claim: whatever bytes are handed to
    // the subprocess must hash to the reported digest.
    let dir = common::TempDir::new("hash");
    let lean = common::stub_lean(dir.path(), "exit 0");
    let out = run_cli(Some(&lean), &[], Some(MILESTONE));
    let v = out.json();
    let reported = v["theorem"]["source_sha256"].as_str().expect("sha256");
    let sent = std::fs::read(dir.path().join("stdin.lean")).expect("stub stdin");
    let actual = himari_loader::hex_digest(&sent);
    assert_eq!(reported, actual);
    assert_eq!(reported.len(), 64);
}

#[test]
fn help_and_version_do_not_touch_lean() {
    let help = run_cli(None, &["--help"], None);
    assert_eq!(help.code, 0);
    assert!(help.stdout.contains("lean_missing"));
    assert!(help.stdout.contains("success, lean_missing"));

    let version = run_cli(None, &["--version"], None);
    assert_eq!(version.code, 0);
    assert!(version.stdout.contains("himari-lean 0.1.0"));
}

#[test]
fn unknown_arguments_are_usage_errors() {
    let out = run_cli(None, &["--frobnicate"], None);
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("unknown argument"));
    assert!(out.stdout.is_empty());
}

#[test]
fn unreadable_input_file_is_a_usage_error() {
    let out = run_cli(
        None,
        &["--input", "/nonexistent/himari-lean/req.json"],
        None,
    );
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("cannot read"));
}

/// Whatever discovery finds on this machine, the argv it would use is the
/// documented one. Asserted on the constructor so the contract is pinned even
/// where no toolchain exists.
#[test]
fn argv_contract_is_pinned() {
    assert_eq!(
        LeanToolchain::check_argv(400_000, 2048),
        vec!["--stdin", "--json", "-T", "400000", "-M", "2048"]
    );
}
