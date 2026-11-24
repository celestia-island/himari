//! Input plumbing and `invalid_request` refusal at the CLI boundary.
//!
//! Every case here must be refused *before* any toolchain is consulted, so these
//! tests never pass a usable `lean` and run identically on every machine.

mod common;

use common::{run_cli, TempDir};
use serde_json::Value;

const MILESTONE: &str = r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10]}"#;

fn assert_invalid(out: &common::CliOutcome, needle: Option<&str>) {
    let v = out.json();
    assert_eq!(v["status"], "invalid_request", "stdout: {}", out.stdout);
    assert_eq!(v["verified"], false);
    assert_eq!(out.code, 4, "exit code must be the invalid_request code");
    assert_eq!(v["exit_code"], 4);
    let messages: String = v["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["message"].as_str().unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" | ");
    if let Some(n) = needle {
        assert!(messages.contains(n), "expected `{n}` in: {messages}");
    }
}

#[test]
fn request_from_stdin_is_accepted_past_intake() {
    // No usable toolchain is supplied, so the run stops at `lean_missing` — but
    // it must get past intake, prove it did by reporting the oracle value, and
    // exit non-zero.
    let out = run_cli(None, &["--no-verify"], Some(MILESTONE));
    let v = out.json();
    assert_eq!(v["status"], "lean_missing");
    assert_eq!(v["oracle"]["det3"], "-3");
    assert_eq!(v["request"]["theorem"], "det3");
}

#[test]
fn request_from_a_file_matches_stdin() {
    let dir = TempDir::new("input-file");
    let path = dir.path().join("req.json");
    std::fs::write(&path, MILESTONE).unwrap();
    let from_file = run_cli(
        None,
        &["--input", path.to_str().unwrap(), "--no-verify"],
        None,
    );
    let from_stdin = run_cli(None, &["--no-verify"], Some(MILESTONE));
    assert_eq!(from_file.json()["status"], from_stdin.json()["status"]);
    assert_eq!(
        from_file.json()["theorem"]["source_sha256"],
        from_stdin.json()["theorem"]["source_sha256"]
    );
    // The `--input=<path>` spelling is equivalent.
    let inline = run_cli(
        None,
        &[&format!("--input={}", path.display()), "--no-verify"],
        None,
    );
    assert_eq!(
        inline.json()["theorem"]["source_sha256"],
        from_stdin.json()["theorem"]["source_sha256"]
    );
}

#[test]
fn integer_strings_are_accepted() {
    let out = run_cli(
        None,
        &["--no-verify"],
        Some(r#"{"theorem":"det3","values":["1","2","3","4","5","6","7","8","10"]}"#),
    );
    let v = out.json();
    assert_eq!(v["status"], "lean_missing");
    assert_eq!(v["oracle"]["det3"], "-3");
}

#[test]
fn empty_stdin_is_an_invalid_request() {
    let out = run_cli(None, &["--no-verify"], Some(""));
    assert_invalid(&out, Some("not valid JSON"));
}

#[test]
fn malformed_json_is_an_invalid_request() {
    for bad in ["{", "[]", "null", "\"det3\"", "{\"theorem\":\"det3\"}"] {
        let out = run_cli(None, &["--no-verify"], Some(bad));
        assert_invalid(&out, None);
    }
}

#[test]
fn wrong_matrix_arity_is_refused() {
    let out = run_cli(
        None,
        &["--no-verify"],
        Some(r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8]}"#),
    );
    assert_invalid(&out, Some("exactly 9"));
}

#[test]
fn unknown_theorem_is_refused() {
    let out = run_cli(
        None,
        &["--no-verify"],
        Some(r#"{"theorem":"det4","values":[1,2,3,4,5,6,7,8,10]}"#),
    );
    assert_invalid(&out, Some("unsupported theorem"));
}

#[test]
fn unknown_fields_are_refused_rather_than_ignored() {
    let out = run_cli(
        None,
        &["--no-verify"],
        Some(r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"values_typo":1}"#),
    );
    assert_invalid(&out, Some("unknown request field"));
}

#[test]
fn forbidden_constructs_in_the_request_are_refused() {
    let attacks = [
        r##"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"x":"by native_decide"}"##,
        r##"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"x":"#eval 1"}"##,
        r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,"10) sorry ("]}"#,
        r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,"10\naxiom cheat : False"]}"#,
        r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"x":"@[implemented_by evil] def f := 0"}"#,
        r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"x":"partial def loop : Nat := loop"}"#,
        r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"x":"extern foo : Nat"}"#,
        r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"x":"import Mathlib"}"#,
    ];
    for a in attacks {
        let out = run_cli(None, &["--no-verify"], Some(a));
        let v = out.json();
        assert_eq!(v["status"], "invalid_request", "should refuse: {a}");
        assert_eq!(out.code, 4);
        let msg = v["diagnostics"][0]["message"].as_str().unwrap_or("");
        assert!(msg.contains("forbidden token"), "{a}: {msg}");
    }
}

#[test]
fn oracle_disagreement_is_reported_before_lean_runs() {
    let out = run_cli(
        None,
        &["--no-verify"],
        Some(r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"expected":"-306"}"#),
    );
    let v = out.json();
    assert_eq!(v["status"], "oracle_mismatch");
    assert_eq!(out.code, 8);
    assert_eq!(v["verified"], false);
    assert_eq!(v["oracle"]["det3"], "-3");
    assert_eq!(v["oracle"]["expected"], "-306");
    assert_eq!(v["oracle"]["agrees"], false);
}

#[test]
fn oracle_agreement_still_needs_lean_to_reach_success() {
    let out = run_cli(
        None,
        &["--no-verify"],
        Some(r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"expected":-3}"#),
    );
    let v = out.json();
    assert_eq!(v["status"], "lean_missing");
    assert_eq!(v["oracle"]["agrees"], true);
}

#[test]
fn the_echo_carries_the_request_back() {
    let out = run_cli(None, &["--no-verify"], Some(MILESTONE));
    let v = out.json();
    let values = v["request"]["values"].as_array().unwrap();
    assert_eq!(values.len(), 9);
    assert_eq!(values[8], 10);
    assert_eq!(v["request"]["theorem"], "det3");
}

#[test]
fn a_refused_request_never_produces_a_source_hash() {
    // Nothing was generated, so there is nothing to hash — and therefore no way
    // to mistake a refusal for an unverified-but-generated obligation.
    let dir = TempDir::new("no-hash");
    let out = run_cli(Some(&dir.path().join("does-not-exist")), &[], Some("{"));
    let v = out.json();
    assert_eq!(v["status"], "invalid_request");
    let _ = Value::Null;
    assert!(v["theorem"]["source_sha256"].is_null());
    assert!(v["theorem"]["statement"].is_null());
}

#[test]
fn emit_writes_the_obligation_even_without_a_toolchain() {
    // The escape hatch for hosts where Lean cannot be provisioned: the file must
    // land on disk and be a self-contained, kernel-checkable obligation.
    let dir = TempDir::new("emit");
    let out_path = dir.path().join("det3.lean");
    // `--no-verify` forces the missing-toolchain state, so this test asserts the
    // same thing whether or not a real Lean happens to be installed on the host.
    let outcome = run_cli(
        None,
        &["--emit", out_path.to_str().unwrap(), "--no-verify"],
        Some(MILESTONE),
    );
    // No toolchain here, so the run is a non-success...
    assert_ne!(outcome.code, 0, "stdout: {}", outcome.stdout);
    // ...but the obligation is still written, and is not a "verified" claim.
    let src = std::fs::read_to_string(&out_path).expect("emitted file");
    assert!(src.contains("theorem det3_example"), "{src}");
    assert!(src.contains(":= by decide"), "{src}");
    assert!(src.contains("#print axioms det3_example"), "{src}");
    // Scan the code region only: the header comment deliberately names the
    // constructs we refuse, so scanning it would flag the documentation.
    let code: String = src.split("-/").nth(1).unwrap_or(&src).to_string();
    for forbidden in [
        "native_decide",
        "sorry",
        "axiom ",
        "implemented_by",
        "extern",
        "#eval",
    ] {
        assert!(
            !code.contains(forbidden),
            "`{forbidden}` leaked into:\n{code}"
        );
    }
    let v: Value = outcome.json();
    assert_eq!(v["status"], "lean_missing");
    assert_eq!(v["verified"], false);
}
