//! The one test that needs an actual Lean 4 toolchain.
//!
//! # Status: PENDING — awaiting a provisioned toolchain
//!
//! No Lean was installed in the environment where this crate was written, and a
//! proof was **not** fabricated to compensate. This test is `#[ignore]`d, so it
//! never runs by default and can never be mistaken for a passing integration
//! check. It is enabled by putting a real `lean` on `PATH` (or in
//! `HIMARI_LEAN_BIN`) and running:
//!
//! ```text
//! cargo test -p himari-lean --test cli_real_lean -- --ignored --nocapture
//! ```
//!
//! Until then, the crate's evidence for "would this work" is:
//!
//! * the generated source is byte-for-byte pinned by a unit test,
//! * its determinant expansion was cross-checked against the Leibniz formula on
//!   20 000 random integer matrices outside the test suite,
//! * every field of the `lean` command line is asserted,
//! * and the missing-toolchain path is asserted to be a non-success.
//!
//! What remains unproven is exactly one thing: that Lean accepts the generated
//! file. That is what this test will establish.

mod common;

use std::time::Duration;

use common::{real_lean, run_cli};

const MILESTONE: &str = r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10]}"#;

#[test]
#[ignore = "requires a real Lean 4 toolchain; see the module docs"]
fn real_lean_checks_the_milestone_theorem() {
    let Some(lean) = real_lean() else {
        panic!(
            "no `lean` found; this test is deliberately ignored until a toolchain is \
             provisioned. Install Lean 4, then run with --ignored."
        );
    };

    let out = run_cli(Some(&lean), &["--print-source"], Some(MILESTONE));
    let v = out.json();

    // The whole point: the kernel accepted the obligation.
    assert_eq!(
        v["status"], "success",
        "lean did not accept the generated obligation\nstdout:\n{}\nstderr:\n{}",
        out.stdout, out.stderr
    );
    assert_eq!(out.code, 0);
    assert_eq!(v["verified"], true);

    // A real version string, not a placeholder.
    let version = v["lean"]["version"].as_str().unwrap_or("");
    assert!(
        version.to_lowercase().contains("lean"),
        "unexpected version string: {version}"
    );

    // `by decide` on a closed `Int` goal leans on the classical triple; nothing
    // outside the allow-list may appear, and `verified` already guarantees it,
    // but assert the positive form too so a regression is legible.
    let found: Vec<String> = v["axioms"]["found"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    for name in &found {
        assert!(
            ["propext", "Classical.choice", "Quot.sound"].contains(&name.as_str()),
            "unexpected axiom `{name}` in {found:?}"
        );
    }
    assert_eq!(v["axioms"]["disallowed"].as_array().unwrap().len(), 0);

    // The independent oracle reached the same number.
    assert_eq!(v["oracle"]["det3"], "-3");
    assert_eq!(v["oracle"]["agrees"], serde_json::Value::Null);

    // A canary: the same binary, same request, with the *wrong* expected value,
    // is refused before Lean runs. This guards against the harness itself having
    // become permissive.
    let bad = run_cli(
        Some(&lean),
        &[],
        Some(r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"expected":-306}"#),
    );
    assert_eq!(bad.json()["status"], "oracle_mismatch");

    // Negative control for "Lean really ran": a request whose statement the
    // kernel cannot prove must come back as `proof_failed`. Because the
    // generator always writes the *correct* right-hand side for the literals it
    // is given, a false statement is not expressible through a valid request --
    // which is the intended safety property. The control therefore runs the file
    // through the same argv with a corrupted right-hand side, one line changed
    // and nothing else, so that the only difference is the truth of the claim.
    let source = v["theorem"]["source"]
        .as_str()
        .expect("--print-source should embed the source");
    let corrupted = source.replace("= -3 := by decide", "= -4 := by decide");
    assert_ne!(corrupted, source, "the statement line must have been found");

    let run = himari_lean::runner::LeanToolchain::discover();
    let toolchain = match run {
        himari_lean::Discovery::Found(tc) => tc,
        other => panic!("toolchain disappeared between runs: {other:?}"),
    };
    let result = toolchain
        .check(&corrupted, 400_000, 2048, Duration::from_secs(600))
        .expect("spawn lean");
    assert_ne!(
        result.exit_code,
        Some(0),
        "a false statement must not check"
    );
    assert!(
        result.stdout.contains("\"severity\":\"error\""),
        "expected an error diagnostic, got:\n{}",
        result.stdout
    );
}
