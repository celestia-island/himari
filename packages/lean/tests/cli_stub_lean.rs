//! Plumbing tests driven by a **stub** `lean`, not by Lean.
//!
//! The stub emulates the observable contract of `lean --stdin --json -T .. -M ..`:
//! it records argv and stdin, then prints whatever the test asks for. These tests
//! prove that this crate
//!
//! * passes the documented argv and the exact generated source,
//! * maps each kind of Lean output onto the right status,
//! * enforces the axiom allow-list on real `#print axioms` output shapes,
//! * honours the wall-clock cap.
//!
//! They prove **nothing about Lean**. A green test here means the pipe is wired
//! correctly; the kernel check lives in `cli_real_lean.rs`.
//!
//! Unix only (the stub is a shell script).

#![cfg(unix)]

mod common;

use common::{
    run_cli, stub_lean, TempDir, STUB_CLEAN_AXIOMS, STUB_DIRTY_AXIOMS, STUB_HANGS,
    STUB_NO_AXIOM_LINE, STUB_PROOF_FAILED,
};

const MILESTONE: &str = r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10]}"#;

fn argv(dir: &TempDir) -> Vec<String> {
    std::fs::read_to_string(dir.path().join("argv.txt"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn stub_success_requires_a_clean_axiom_report() {
    // A stub that exits 0 but never answers `#print axioms` must NOT be read as a
    // success: we do not know what the theorem rests on.
    let dir = TempDir::new("stub-silent");
    let lean = stub_lean(dir.path(), STUB_NO_AXIOM_LINE);
    let out = run_cli(Some(&lean), &[], Some(MILESTONE));
    let v = out.json();
    assert_eq!(v["status"], "axiom_violation", "{}", out.stdout);
    assert_eq!(out.code, 7);
    assert_eq!(v["verified"], false);
    assert_eq!(v["axioms"]["parsed"], false);
}

#[test]
fn stub_success_with_allowed_axioms_is_reported_as_success() {
    let dir = TempDir::new("stub-clean");
    let lean = stub_lean(dir.path(), STUB_CLEAN_AXIOMS);
    let out = run_cli(Some(&lean), &[], Some(MILESTONE));
    let v = out.json();
    assert_eq!(v["status"], "success", "stderr: {}", out.stderr);
    assert_eq!(out.code, 0);
    assert_eq!(v["verified"], true);
    assert_eq!(v["exit_code"], 0);
    // The axiom list is what the stub printed, parsed by the real parser.
    let found = v["axioms"]["found"].as_array().unwrap();
    assert_eq!(found.len(), 3);
    assert!(found.iter().any(|f| f == "propext"));
    assert!(found.iter().any(|f| f == "Classical.choice"));
    assert!(found.iter().any(|f| f == "Quot.sound"));
    assert_eq!(v["axioms"]["disallowed"].as_array().unwrap().len(), 0);
    // The oracle agrees, independently of the stub.
    assert_eq!(v["oracle"]["det3"], "-3");
    // A version line is reported because the stub answered `--version`.
    assert!(v["lean"]["version"].is_string());
}

#[test]
fn stub_disallowed_axioms_are_an_axiom_violation() {
    let dir = TempDir::new("stub-dirty");
    let lean = stub_lean(dir.path(), STUB_DIRTY_AXIOMS);
    let out = run_cli(Some(&lean), &[], Some(MILESTONE));
    let v = out.json();
    assert_eq!(v["status"], "axiom_violation");
    assert_eq!(out.code, 7);
    assert_eq!(v["verified"], false);
    let disallowed: Vec<&str> = v["axioms"]["disallowed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d.as_str().unwrap_or(""))
        .collect();
    assert!(disallowed.contains(&"Lean.ofReduceBool"), "{disallowed:?}");
    assert!(disallowed.contains(&"Lean.trustCompiler"), "{disallowed:?}");
}

#[test]
fn stub_proof_failure_is_proof_failed() {
    let dir = TempDir::new("stub-fail");
    let lean = stub_lean(dir.path(), STUB_PROOF_FAILED);
    let out = run_cli(Some(&lean), &[], Some(MILESTONE));
    let v = out.json();
    assert_eq!(v["status"], "proof_failed");
    assert_eq!(out.code, 5);
    assert_eq!(v["verified"], false);
    // Lean's own message reached the diagnostics, with its position.
    let msgs: Vec<&str> = v["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["level"] == "error")
        .map(|d| d["message"].as_str().unwrap_or(""))
        .collect();
    assert!(
        msgs.iter().any(|m| m.contains("decide")),
        "Lean's message should be surfaced: {msgs:?}"
    );
}

#[test]
fn stub_argv_and_stdin_are_exactly_the_contract() {
    let dir = TempDir::new("stub-argv");
    let lean = stub_lean(dir.path(), STUB_CLEAN_AXIOMS);
    let out = run_cli(Some(&lean), &["--print-source"], Some(MILESTONE));
    assert_eq!(out.json()["status"], "success", "{}", out.stderr);

    assert_eq!(
        argv(&dir),
        vec!["--stdin", "--json", "-T", "400000", "-M", "2048"]
    );

    let sent = std::fs::read_to_string(dir.path().join("stdin.lean")).unwrap();
    assert!(sent.contains("def det3 (a b c d e f g h i : Int) : Int :="));
    assert!(sent.contains("theorem det3_example : det3 1 2 3 4 5 6 7 8 10 = -3 := by decide"));
    assert!(sent.contains("#print axioms det3_example"));
    // The source that went over the wire is exactly what the report hashes.
    assert_eq!(
        out.json()["theorem"]["source_sha256"],
        himari_loader::hex_digest(sent.as_bytes())
    );
}

#[test]
fn stub_budgets_are_forwarded_from_the_request() {
    let dir = TempDir::new("stub-budgets");
    let lean = stub_lean(dir.path(), STUB_CLEAN_AXIOMS);
    let req =
        r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"heartbeats":1234,"memory_mb":512}"#;
    let out = run_cli(Some(&lean), &[], Some(req));
    assert_eq!(out.json()["status"], "success", "{}", out.stderr);
    assert_eq!(
        argv(&dir),
        vec!["--stdin", "--json", "-T", "1234", "-M", "512"]
    );
}

#[test]
fn stub_timeout_is_resource_exhausted() {
    let dir = TempDir::new("stub-hang");
    let lean = stub_lean(dir.path(), STUB_HANGS);
    let req = r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"timeout_secs":1}"#;
    let started = std::time::Instant::now();
    let out = run_cli(Some(&lean), &[], Some(req));
    let elapsed = started.elapsed();
    let v = out.json();
    assert_eq!(v["status"], "resource_exhausted", "{}", out.stdout);
    assert_eq!(out.code, 6);
    assert_eq!(v["verified"], false);
    assert!(
        elapsed < std::time::Duration::from_secs(20),
        "the cap must actually bound the run, took {elapsed:?}"
    );
}

#[test]
fn stub_warning_without_axiom_line_is_still_not_a_success() {
    // A `sorry` in the file produces a warning and a `sorryAx` axiom. Even when
    // the axiom line is missing entirely, absence must not read as clean.
    let dir = TempDir::new("stub-warn");
    let behaviour = r#"cat <<'EOF'
{"severity":"warning","data":"declaration uses 'sorry'"}
EOF
exit 0"#;
    let lean = stub_lean(dir.path(), behaviour);
    let out = run_cli(Some(&lean), &[], Some(MILESTONE));
    assert_eq!(out.json()["status"], "axiom_violation");
    assert_eq!(out.code, 7);
}

#[test]
fn stub_sorry_axiom_is_a_violation() {
    let dir = TempDir::new("stub-sorry");
    let behaviour = r#"cat <<'EOF'
{"severity":"warning","data":"declaration uses 'sorry'"}
{"severity":"info","data":"'det3_example' depends on axioms: [propext, sorryAx]"}
EOF
exit 0"#;
    let lean = stub_lean(dir.path(), behaviour);
    let out = run_cli(Some(&lean), &[], Some(MILESTONE));
    let v = out.json();
    assert_eq!(v["status"], "axiom_violation");
    let disallowed = v["axioms"]["disallowed"].as_array().unwrap();
    assert!(disallowed.iter().any(|d| d == "sorryAx"));
}

#[test]
fn stub_native_decide_signature_is_a_violation() {
    // `native_decide` is the exact attack the allow-list exists to catch.
    let dir = TempDir::new("stub-native");
    let behaviour = r#"cat <<'EOF'
{"severity":"info","data":"'det3_example' depends on axioms: [Lean.ofReduceBool, Lean.trustCompiler]"}
EOF
exit 0"#;
    let lean = stub_lean(dir.path(), behaviour);
    let out = run_cli(Some(&lean), &[], Some(MILESTONE));
    assert_eq!(out.json()["status"], "axiom_violation");
    assert_eq!(out.code, 7);
}

#[test]
fn stub_success_reports_lean_version_and_argv() {
    let dir = TempDir::new("stub-meta");
    let lean = stub_lean(dir.path(), STUB_CLEAN_AXIOMS);
    let out = run_cli(Some(&lean), &[], Some(MILESTONE));
    let v = out.json();
    assert_eq!(v["status"], "success");
    let argv: Vec<&str> = v["lean"]["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        argv,
        vec!["--stdin", "--json", "-T", "400000", "-M", "2048"]
    );
    let version = v["lean"]["version"].as_str().unwrap();
    assert!(!version.is_empty());
}

#[test]
fn stub_success_reports_the_statement_and_hash() {
    let dir = TempDir::new("stub-statement");
    let lean = stub_lean(dir.path(), STUB_CLEAN_AXIOMS);
    let out = run_cli(Some(&lean), &[], Some(MILESTONE));
    let v = out.json();
    assert_eq!(
        v["theorem"]["statement"],
        "theorem det3_example : det3 1 2 3 4 5 6 7 8 10 = -3 := by decide"
    );
    assert_eq!(v["theorem"]["name"], "det3_example");
    assert_eq!(v["theorem"]["source_sha256"].as_str().unwrap().len(), 64);
}

#[test]
fn stub_override_literals_change_the_source_but_not_the_oracle() {
    // Building a statement about a different matrix on purpose: the source must
    // follow the requested literals, while the oracle reports the requested
    // `values` — the two are separate questions and must not be conflated.
    let dir = TempDir::new("stub-literals");
    let lean = stub_lean(dir.path(), STUB_CLEAN_AXIOMS);
    let req = r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"literals":[9,9,9,9,9,9,9,9,9]}"#;
    let out = run_cli(Some(&lean), &[], Some(req));
    let v = out.json();
    assert_eq!(
        v["theorem"]["statement"],
        "theorem det3_example : det3 9 9 9 9 9 9 9 9 9 = 0 := by decide"
    );
    assert_eq!(v["oracle"]["det3"], "-3");
}

#[test]
fn stub_success_is_still_only_success_when_every_check_passed() {
    // Control: the same stub with a *mismatched* request must not be rescued by
    // the stub saying "axioms fine". The oracle check runs first.
    let dir = TempDir::new("stub-control");
    let lean = stub_lean(dir.path(), STUB_CLEAN_AXIOMS);
    let req = r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"expected":-306}"#;
    let out = run_cli(Some(&lean), &[], Some(req));
    assert_eq!(out.json()["status"], "oracle_mismatch");
    // Lean was never invoked: the stub left no recording.
    assert!(!dir.path().join("stdin.lean").exists());
}
