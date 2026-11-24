//! End-to-end checks of the `himari-standard` binary: request in (JSON and TOML, file and
//! stdin), stable response document out, exit codes as documented.
//!
//! These tests spawn the built binary through `CARGO_BIN_EXE_himari-standard`, so they
//! exercise exactly what a user runs.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_himari-standard"))
}

/// A unique scratch path under the target directory.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("himari-standard-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir.join(name)
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run(args: &[&str], stdin: Option<&str>) -> Run {
    let mut cmd = bin();
    cmd.args(args);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd.stdin(if stdin.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    let mut child = cmd.spawn().expect("spawn himari-standard");
    if let Some(text) = stdin {
        child
            .stdin
            .as_mut()
            .expect("stdin")
            .write_all(text.as_bytes())
            .expect("write stdin");
    }
    let out = child.wait_with_output().expect("wait");
    Run {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn json(text: &str) -> serde_json::Value {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {text}"))
}

const BRINELL_JSON: &str = r#"{"standard":"ISO-6506-1","edition":2014,"force":29420.0,
"ball_diameter":0.01,"indentation_diameter":0.004}"#;

const BRINELL_TOML: &str = "standard = \"ISO-6506-1\"\nedition = 2014\nforce = 29420.0\n\
ball_diameter = 0.01\nindentation_diameter = 0.004\n";

#[test]
fn list_prints_exact_ids_and_editions() {
    let r = run(&["--list"], None);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    let list = json(&r.stdout);
    let entries: Vec<(String, String)> = list
        .as_array()
        .expect("array")
        .iter()
        .map(|e| {
            (
                e["standard"].as_str().unwrap().to_string(),
                e["edition"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(
        entries,
        vec![
            ("ISO-6506-1".to_string(), "2014".to_string()),
            ("ISO-6892-1".to_string(), "2019".to_string()),
            ("ISO-2533".to_string(), "1975".to_string()),
            ("GB-T-231.1".to_string(), "2018".to_string()),
            ("GB-T-228.1".to_string(), "2021".to_string()),
            ("GB-T-8170".to_string(), "2008".to_string()),
        ]
    );
}

#[test]
fn json_file_request_produces_the_stable_document() {
    let path = scratch("brinell.json");
    std::fs::write(&path, BRINELL_JSON).unwrap();
    let r = run(&["--input", path.to_str().unwrap()], None);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    let doc = json(&r.stdout);
    for key in [
        "standard",
        "edition",
        "provenance",
        "values",
        "warnings",
        "errors",
    ] {
        assert!(doc.get(key).is_some(), "missing {key}: {}", r.stdout);
    }
    assert_eq!(doc["standard"], "ISO-6506-1");
    assert_eq!(doc["edition"], "2014");
    assert_eq!(doc["errors"], serde_json::json!([]));
    let hbw = doc["values"]["hbw"].as_f64().unwrap();
    assert!((hbw - 228.831_198_210_826_27).abs() < 1e-9, "hbw = {hbw}");
}

#[test]
fn toml_and_json_requests_agree() {
    let json_path = scratch("same.json");
    let toml_path = scratch("same.toml");
    std::fs::write(&json_path, BRINELL_JSON).unwrap();
    std::fs::write(&toml_path, BRINELL_TOML).unwrap();
    let a = run(&["--input", json_path.to_str().unwrap()], None);
    let b = run(&["--input", toml_path.to_str().unwrap()], None);
    assert_eq!((a.code, b.code), (0, 0), "{} {}", a.stderr, b.stderr);
    assert_eq!(json(&a.stdout), json(&b.stdout));
}

#[test]
fn stdin_is_used_when_no_input_is_given() {
    let j = run(&[], Some(BRINELL_JSON));
    assert_eq!(j.code, 0, "stderr: {}", j.stderr);
    assert_eq!(json(&j.stdout)["standard"], "ISO-6506-1");

    // TOML is recognised by content (only JSON starts with a bracket), so --format is
    // optional for either dialect.
    let t = run(&[], Some(BRINELL_TOML));
    assert_eq!(t.code, 0, "stderr: {}", t.stderr);
    assert_eq!(json(&t.stdout)["edition"], "2014");

    let forced = run(&["--format", "toml"], Some(BRINELL_TOML));
    assert_eq!(forced.code, 0, "stderr: {}", forced.stderr);

    // Text that is neither dialect is a parse error.
    let bad = run(&[], Some("standard = "));
    assert_eq!(bad.code, 1);
    assert_eq!(json(&bad.stdout)["errors"][0]["code"], "invalid-document");
}

#[test]
fn format_can_be_forced_regardless_of_extension() {
    let path = scratch("mislabelled.json");
    std::fs::write(&path, BRINELL_TOML).unwrap();
    let auto = run(&["--input", path.to_str().unwrap()], None);
    assert_eq!(auto.code, 1, "the .json extension must be honoured");
    let forced = run(
        &["--input", path.to_str().unwrap(), "--format", "toml"],
        None,
    );
    assert_eq!(forced.code, 0, "stderr: {}", forced.stderr);
}

#[test]
fn every_standard_runs_through_the_cli() {
    let cases = [
        (
            "tensile.json",
            r#"{"standard":"ISO-6892-1","edition":2019,"maximum_force":12566.0,
               "cross_section_area":4e-5,"initial_gauge_length":0.05,"final_gauge_length":0.065}"#,
        ),
        (
            "atmosphere.toml",
            "standard = \"ISO-2533\"\nedition = 1975\ngeometric_altitude_m = 0\n",
        ),
        (
            "round.json",
            r#"{"standard":"GB-T-8170","edition":2008,"value":"60.25","digits":"1"}"#,
        ),
    ];
    for (name, body) in cases {
        let path = scratch(name);
        std::fs::write(&path, body).unwrap();
        let r = run(&["--input", path.to_str().unwrap()], None);
        assert_eq!(r.code, 0, "{name}: {}", r.stderr);
        assert_eq!(json(&r.stdout)["errors"], serde_json::json!([]), "{name}");
    }
}

#[test]
fn rounding_keeps_decimal_exactness_through_the_cli() {
    let path = scratch("round-unit.json");
    std::fs::write(
        &path,
        r#"{"standard":"GB-T-8170","edition":2008,"value":"60.25","digits":"0.5","mode":"unit"}"#,
    )
    .unwrap();
    let r = run(&["--input", path.to_str().unwrap()], None);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    let doc = json(&r.stdout);
    assert_eq!(doc["values"]["rounded"], "60.0");
    assert_eq!(doc["values"]["tie"], true);
}

#[test]
fn failures_are_documents_with_errors_and_exit_one() {
    // Unknown field.
    let path = scratch("typo.json");
    std::fs::write(
        &path,
        r#"{"standard":"ISO-6506-1","edition":2014,"force":1,"ball_diameter":0.01,
            "indentation_diameter":0.004,"extra":1}"#,
    )
    .unwrap();
    let r = run(&["--input", path.to_str().unwrap()], None);
    assert_eq!(r.code, 1);
    let doc = json(&r.stdout);
    assert_eq!(doc["errors"][0]["code"], "unsupported-input");
    assert_eq!(doc["values"], serde_json::Value::Null);
    assert!(!r.stderr.is_empty());

    // Unknown standard.
    let r = run(&[], Some(r#"{"standard":"ISO-0000","edition":2014}"#));
    assert_eq!(r.code, 1);
    assert_eq!(json(&r.stdout)["errors"][0]["code"], "unknown-standard");

    // Edition mismatch.
    let r = run(&[], Some(r#"{"standard":"ISO-6506-1","edition":2024}"#));
    assert_eq!(r.code, 1);
    assert_eq!(json(&r.stdout)["errors"][0]["code"], "unsupported-edition");

    // Numeric domain error.
    let r = run(
        &[],
        Some(
            r#"{"standard":"ISO-6506-1","edition":2014,"force":29420,"ball_diameter":0.01,
                "indentation_diameter":0.02}"#,
        ),
    );
    assert_eq!(r.code, 1);
    assert_eq!(json(&r.stdout)["errors"][0]["code"], "invalid-input");
}

#[test]
fn command_line_misuse_is_exit_two_without_a_document() {
    for args in [
        vec!["--nope"],
        vec!["--format", "yaml"],
        vec!["--input", "/nonexistent/request.json"],
        vec!["--list", "--input", "x.json"],
    ] {
        let r = run(&args, None);
        assert_eq!(r.code, 2, "args {args:?} gave {}", r.code);
        assert!(r.stdout.is_empty(), "args {args:?} wrote a document");
    }
}

#[test]
fn gb_ids_answer_with_their_own_identity() {
    // A GB request must never be labelled as the ISO parent standard.
    let body = r#"{"standard":"GB-T-231.1","edition":2018,"force":29420.0,"ball_diameter":0.01,"indentation_diameter":0.004}"#;
    let r = run(&[], Some(body));
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    let doc = json(&r.stdout);
    assert_eq!(doc["standard"], "GB-T-231.1");
    assert_eq!(doc["edition"], "2018");
    let hbw = doc["values"]["hbw"].as_f64().unwrap();
    assert!((hbw - 228.831_198_210_826_27).abs() < 1e-9, "{hbw}");

    let body = r#"{"standard":"GB-T-228.1","edition":2021,"maximum_force":31415.0,"cross_section_area":1e-4}"#;
    let r = run(&[], Some(body));
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    let doc = json(&r.stdout);
    assert_eq!(doc["standard"], "GB-T-228.1");
    assert!((doc["values"]["tensile_strength_pa"].as_f64().unwrap() - 3.1415e8).abs() < 1.0);
}

#[test]
fn shipped_examples_are_valid_requests() {
    // Drift guard for the shipped example requests under `examples/`. Not every one
    // of them is cited by the README or the justfile recipe; this test pins the set
    // that ships, so renaming or breaking a file is caught either way.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    for (name, expect_standard) in [
        ("brinell.json", "GB-T-231.1"),
        ("tensile.toml", "GB-T-228.1"),
        ("atmosphere.json", "ISO-2533"),
    ] {
        let path = root.join(name);
        let r = run(&["--input", path.to_str().unwrap()], None);
        assert_eq!(r.code, 0, "{name} stderr: {}", r.stderr);
        let doc = json(&r.stdout);
        assert_eq!(doc["standard"], expect_standard, "{name}");
        assert_eq!(
            doc["errors"].as_array().unwrap().len(),
            0,
            "{name}: {}",
            r.stdout
        );
    }
}
