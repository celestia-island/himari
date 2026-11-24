//! The one function that turns a request into an outcome.
//!
//! Order of operations matters and is deliberate:
//!
//! 1. **Request intake** — a malformed or forbidden request stops here, before
//!    anything is generated. (`invalid_request`)
//! 2. **Oracle** — Rust computes `det3` independently. If the request also
//!    states an expected value and the two disagree, the run stops before Lean
//!    is started. (`oracle_mismatch`)
//! 3. **Toolchain** — no usable `lean` stops here. (`lean_missing`) This step is
//!    *after* the oracle on purpose: a request can be judged wrong without a
//!    toolchain, and the two refusals must stay distinguishable.
//! 4. **Generate + run** — the generated source is hashed, then fed to `lean`
//!    over stdin. A deadline kill is `resource_exhausted`.
//! 5. **Proof** — a non-zero exit, or an `error` diagnostic from Lean, is
//!    `proof_failed`.
//! 6. **Axioms** — the `#print axioms` output must parse and name nothing
//!    outside the allow-list. (`axiom_violation`)
//! 7. **Success** — all of the above held.
//!
//! Every exit path fills in `status` *and* at least one diagnostic, so a caller
//! never receives a bare non-success it cannot explain.

use std::time::Duration;

use serde_json::{json, Value};

use crate::axioms;
use crate::oracle;
use crate::report::{Diagnostic, OracleReport, Report};
use crate::request::{self, Det3Request};
use crate::runner::{Discovery, LeanRun, LeanToolchain};
use crate::source;
use crate::status::Status;

/// What a run should be allowed to do.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunOptions {
    /// Include the generated source text in the report.
    pub include_source: bool,
    /// When set, skip toolchain discovery and use this executable. `None` means
    /// "let Lean be found"; tests use the override to pass a stub path, and the
    /// CLI never sets it (it honours `HIMARI_LEAN_BIN` through discovery).
    pub lean_override: Option<LeanToolchain>,
    /// When true, never look for a toolchain. Used by the unit-test suite so it
    /// cannot accidentally depend on what is installed on the machine.
    pub never_run_lean: bool,
}

impl RunOptions {
    /// Include the generated Lean source in the result.
    pub fn with_source(mut self) -> Self {
        self.include_source = true;
        self
    }

    /// Pretend no toolchain is installed. Tests only.
    pub fn without_lean(mut self) -> Self {
        self.never_run_lean = true;
        self
    }

    /// Use a specific `lean` executable. Tests only.
    pub fn with_lean(mut self, tc: LeanToolchain) -> Self {
        self.lean_override = Some(tc);
        self
    }
}

/// Validate and (optionally) discharge one request.
///
/// This is the crate's whole contract in one function; see the module docs for
/// the ordering rationale.
pub fn verify_request(raw: &str, opts: &RunOptions) -> Report {
    let mut diag: Vec<Diagnostic> = Vec::new();

    // 1. Intake.
    let req = match request::parse_request(raw) {
        Ok(r) => r,
        Err(e) => {
            let echo = echo_request(raw);
            diag.push(
                Diagnostic::error("invalid_request", e.to_string())
                    .at(e.field.clone().unwrap_or_else(|| "$".into())),
            );
            let mut rep = Report::new(echo);
            rep.diagnostics = diag;
            rep.status = e.status;
            return rep;
        }
    };
    diag.push(Diagnostic::info(
        "request_accepted",
        format!(
            "theorem `{}`, budgets: {} heartbeats, {} MB, {} s wall clock",
            request::THEOREM_DET3,
            req.heartbeats,
            req.memory_mb,
            req.timeout_secs
        ),
    ));

    let echo = json!({
        "theorem": request::THEOREM_DET3,
        "values": req.values,
        "literals": req.literals,
        "expected": req.expected.map(|v| v.to_string()),
    });

    // 2. Oracle first: a wrong request can be refused without any toolchain.
    let oracle_value = match oracle::det3_or_fail(&req.values) {
        Ok(v) => v,
        Err(e) => {
            diag.push(Diagnostic::error("oracle_failure", e));
            let mut rep = Report::new(echo);
            rep.diagnostics = diag;
            rep.status = Status::OracleMismatch;
            rep.oracle = OracleReport {
                det3: None,
                expected: req.expected,
                agrees: None,
                failure: Some("the oracle could not answer".into()),
            };
            return rep;
        }
    };
    let mut oracle_report = OracleReport {
        det3: Some(oracle_value),
        expected: req.expected,
        agrees: None,
        failure: None,
    };
    diag.push(Diagnostic::info(
        "oracle_value",
        format!("rust oracle: det3({:?}) = {}", req.values, oracle_value),
    ));

    if let Some(expected) = req.expected {
        if expected != oracle_value {
            oracle_report.agrees = Some(false);
            diag.push(Diagnostic::error(
                "oracle_mismatch",
                format!("request states det3 = {expected}, rust oracle computes {oracle_value}"),
            ));
            let mut rep = Report::new(echo);
            rep.diagnostics = diag;
            rep.status = Status::OracleMismatch;
            rep.oracle = oracle_report;
            return rep;
        }
        oracle_report.agrees = Some(true);
    }

    // The statement uses the oracle value as its right-hand side *unless* the
    // request supplied override literals, in which case the caller is building a
    // statement on purpose and we must not "fix" it into a true one.
    let rhs = if req.literals == req.values {
        oracle_value
    } else {
        match oracle::det3_or_fail(&req.literals) {
            Ok(v) => v,
            Err(e) => {
                diag.push(Diagnostic::error("oracle_failure", e));
                let mut rep = Report::new(echo);
                rep.diagnostics = diag;
                rep.status = Status::OracleMismatch;
                rep.oracle = oracle_report;
                return rep;
            }
        }
    };

    // 3. Generate + hash before touching the toolchain, so every report that gets
    // past intake carries the exact source bytes it talks about.
    let lean_source = source::generate_det3_source(&req.values, &req.literals, rhs);
    let mut rep = Report::new(echo).with_source(&lean_source);
    rep.oracle = oracle_report;
    rep.diagnostics.extend(diag);

    // 4. Toolchain.
    let toolchain = match resolve_toolchain(opts, &mut rep) {
        Resolved::Ready(tc) => tc,
        Resolved::Stop(status) => {
            rep.status = status;
            return finish(rep, opts);
        }
    };

    // 5. Run.
    rep.lean_version = toolchain.version.clone();
    let run = match toolchain.check(
        &lean_source,
        req.heartbeats,
        req.memory_mb,
        Duration::from_secs(req.timeout_secs),
    ) {
        Ok(run) => run,
        Err(e) => {
            rep.diagnostics.push(Diagnostic::error(
                "lean_spawn_failed",
                format!("could not run `{}`: {e}", toolchain.path.display()),
            ));
            rep.status = Status::LeanMissing;
            return finish(rep, opts);
        }
    };
    rep.lean_argv = Some(run.argv.clone());
    rep.diagnostics.push(Diagnostic::info(
        "lean_argv",
        format!("lean {}", run.argv.join(" ")),
    ));
    rep.diagnostics.extend(lean_diagnostics(&run));

    if run.timed_out {
        rep.diagnostics.push(Diagnostic::error(
            "resource_exhausted",
            format!(
                "lean did not finish within {} s and was killed",
                req.timeout_secs
            ),
        ));
        rep.status = Status::ResourceExhausted;
        return finish(rep, opts);
    }

    // 6. Proof acceptance.
    if run.exit_code != Some(0) || has_lean_error(&run) {
        rep.diagnostics.push(Diagnostic::error(
            "proof_failed",
            match run.exit_code {
                Some(code) => {
                    format!("lean exited with status {code}; the theorem was not accepted")
                }
                None => "lean terminated without an exit status; the theorem was not accepted"
                    .to_string(),
            },
        ));
        rep.status = Status::ProofFailed;
        return finish(rep, opts);
    }

    // A proof that is not unambiguous must not pass. Even a "successful" exit can
    // leave a warning-level `sorry` in the file; the axiom check below catches
    // the recorded `sorryAx`, but the warning stream is reported either way.
    if let Some(w) = run
        .stdout
        .lines()
        .find(|l| l.contains("\"severity\":\"warning\""))
    {
        rep.diagnostics.push(Diagnostic::warning(
            "lean_warning",
            "lean emitted a warning while checking the obligation",
        ));
        let _ = w;
    }

    // 7. Axioms.
    match axioms::parse_axioms(&run.stdout) {
        Ok(report) => {
            let clean = report.is_clean();
            if clean {
                rep.diagnostics.push(Diagnostic::info(
                    "axioms_allowed",
                    if report.declared_none {
                        "the example theorem depends on no axioms".to_string()
                    } else {
                        format!(
                            "the example theorem depends only on: {}",
                            report.found.join(", ")
                        )
                    },
                ));
            } else {
                rep.diagnostics.push(Diagnostic::error(
                    "axiom_violation",
                    format!(
                        "the example theorem depends on axioms outside the allow-list: {}",
                        report.disallowed.join(", ")
                    ),
                ));
            }
            rep.axioms = Some(report);
            if !clean {
                rep.status = Status::AxiomViolation;
                return finish(rep, opts);
            }
        }
        Err(e) => {
            // An unreadable axiom report is not permission to proceed: we do not
            // know what the theorem rests on, so it is a violation, not a pass.
            rep.diagnostics.push(Diagnostic::error(
                "axiom_violation",
                format!("could not establish the theorem's axioms: {e}"),
            ));
            rep.status = Status::AxiomViolation;
            return finish(rep, opts);
        }
    }

    rep.status = Status::Success;
    finish(rep, opts)
}

enum Resolved {
    Ready(LeanToolchain),
    /// Discovery failed or was disabled; the report already carries why.
    Stop(Status),
}

fn resolve_toolchain(opts: &RunOptions, rep: &mut Report) -> Resolved {
    if opts.never_run_lean {
        rep.diagnostics.push(Diagnostic::error(
            "lean_missing",
            "no `lean` executable available (discovery disabled by the caller)",
        ));
        return Resolved::Stop(Status::LeanMissing);
    }
    if let Some(tc) = &opts.lean_override {
        rep.lean_version = tc.version.clone();
        return Resolved::Ready(tc.clone());
    }
    match LeanToolchain::discover() {
        Discovery::Found(tc) => {
            rep.diagnostics.push(Diagnostic::info(
                "lean_found",
                format!(
                    "{} (from {}): {}",
                    tc.path.display(),
                    tc.origin,
                    tc.version.as_deref().unwrap_or("<version unknown>")
                ),
            ));
            Resolved::Ready(tc)
        }
        Discovery::Missing { searched } => {
            rep.diagnostics.push(Diagnostic::error(
                "lean_missing",
                format!(
                    "no `lean` executable found (searched {}); the theorem was NOT checked",
                    if searched.is_empty() {
                        "no PATH entries".to_string()
                    } else {
                        searched.join(", ")
                    }
                ),
            ));
            rep.diagnostics.push(Diagnostic::info(
                "next_step",
                format!(
                    "install Lean 4, put `lean` on PATH, or set {} to its path",
                    crate::runner::OVERRIDE_ENV
                ),
            ));
            Resolved::Stop(Status::LeanMissing)
        }
        Discovery::Broken { detail } => {
            // A configured-but-unusable toolchain is still "we could not check
            // anything", which is the missing state; the detail says why.
            rep.diagnostics.push(Diagnostic::error(
                "lean_missing",
                format!("no usable `lean` (a candidate was found but failed): {detail}"),
            ));
            Resolved::Stop(Status::LeanMissing)
        }
    }
}

/// Turn the `--json` lines Lean wrote into report diagnostics.
fn lean_diagnostics(run: &LeanRun) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for line in run.stdout.lines().chain(run.stderr.lines()) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            out.push(Diagnostic::info("lean_output", line.to_string()));
            continue;
        };
        let severity = v
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("info")
            .to_string();
        let message = v
            .get("data")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| line.to_string());
        let pos = v.get("pos").and_then(|p| {
            let line = p.get("line")?.as_u64()?;
            let col = p.get("column")?.as_u64()?;
            Some(format!("stdin:{line}:{col}"))
        });
        let mut d = Diagnostic::new(&severity, "lean_message", message);
        d.position = pos;
        out.push(d);
    }
    out
}

fn has_lean_error(run: &LeanRun) -> bool {
    run.stdout
        .lines()
        .chain(run.stderr.lines())
        .any(|l| l.contains("\"severity\":\"error\""))
}

/// Echo a request back even when it could not be parsed, bounded and truncated.
fn echo_request(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.len() > 512 {
        return json!({ "raw_truncated": &trimmed[..512] });
    }
    match serde_json::from_str::<Value>(trimmed) {
        Ok(v) => json!({ "raw": v }),
        Err(_) => json!({ "raw_text": trimmed }),
    }
}

/// Drop the source from the report unless it was requested.
fn finish(mut rep: Report, opts: &RunOptions) -> Report {
    if !opts.include_source {
        rep.source = None;
    }
    debug_assert!(
        rep.status == Status::Success || !rep.diagnostics.iter().all(|d| d.level != "error"),
        "a non-success report must carry an error diagnostic"
    );
    rep
}

/// Convenience for callers that hold a typed request.
pub fn verify_typed(req: &Det3Request, opts: &RunOptions) -> Report {
    let mut raw = json!({
        "theorem": request::THEOREM_DET3,
        "values": req.values,
        "literals": req.literals,
        "heartbeats": req.heartbeats,
        "memory_mb": req.memory_mb,
        "timeout_secs": req.timeout_secs,
    });
    // `expected` is optional: emitting `null` would be rejected by intake, which
    // requires an integer when the key is present.
    if let Some(e) = req.expected {
        raw["expected"] = json!(e.to_string());
    }
    verify_request(&raw.to_string(), opts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MILESTONE: &str = r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10]}"#;

    #[test]
    fn invalid_request_is_reported_before_anything_else() {
        let rep = verify_request(
            r#"{"theorem":"det3","values":[1,2,3]}"#,
            &RunOptions::default().without_lean(),
        );
        assert_eq!(rep.status, Status::InvalidRequest);
        assert!(!rep.status.is_verified());
        assert_eq!(rep.diagnostics[0].code, "invalid_request");
        // Nothing was generated, so there is no source to hash.
        assert!(rep.source_sha256.is_none());
    }

    #[test]
    fn forbidden_input_is_an_invalid_request_with_the_token_named() {
        let rep = verify_request(
            r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"x":"by native_decide"}"#,
            &RunOptions::default().without_lean(),
        );
        assert_eq!(rep.status, Status::InvalidRequest);
        assert!(rep.diagnostics[0].message.contains("native_decide"));
    }

    #[test]
    fn oracle_mismatch_stops_before_the_toolchain() {
        let rep = verify_request(
            r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"expected":-307}"#,
            &RunOptions::default().without_lean(),
        );
        assert_eq!(rep.status, Status::OracleMismatch);
        assert_eq!(rep.oracle.det3, Some(-3));
        assert_eq!(rep.oracle.expected, Some(-307));
        assert_eq!(rep.oracle.agrees, Some(false));
        assert!(rep.diagnostics.iter().any(|d| d.code == "oracle_mismatch"));
    }

    #[test]
    fn matching_expected_value_proceeds_on_the_oracle_dimension() {
        // No Lean here: the run must stop at the toolchain, but only after the
        // oracle has agreed, so the oracle block must be filled in.
        let rep = verify_request(
            r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"expected":"-3"}"#,
            &RunOptions::default().without_lean(),
        );
        assert_eq!(rep.status, Status::LeanMissing);
        assert_eq!(rep.oracle.det3, Some(-3));
        assert_eq!(rep.oracle.agrees, Some(true));
    }

    #[test]
    fn missing_lean_is_a_first_class_non_success() {
        let rep = verify_request(MILESTONE, &RunOptions::default().without_lean());
        assert_eq!(rep.status, Status::LeanMissing);
        assert!(!rep.status.is_verified());
        assert_ne!(rep.status.exit_code(), 0);
        assert_eq!(
            rep.diagnostics.last().map(|d| d.code.as_str()),
            Some("lean_missing")
        );
        // The source was still generated and hashed, so the obligation is
        // reviewable even though nothing checked it.
        assert!(rep.source_sha256.is_some());
        let v = rep.to_json();
        assert_eq!(v["status"], "lean_missing");
        assert_eq!(v["verified"], false);
        assert_eq!(v["oracle"]["det3"], "-3");
    }

    #[test]
    fn source_is_only_included_when_asked_for() {
        let plain = verify_request(MILESTONE, &RunOptions::default().without_lean()).to_json();
        assert!(plain["theorem"].get("source").is_none());
        assert!(plain["theorem"]["source_sha256"].is_string());

        let full = verify_request(
            MILESTONE,
            &RunOptions::default().with_source().without_lean(),
        )
        .to_json();
        assert!(full["theorem"]["source"]
            .as_str()
            .unwrap()
            .contains("def det3"));
    }

    #[test]
    fn statement_is_generated_from_literals_and_the_oracle_rhs() {
        let rep = verify_request(MILESTONE, &RunOptions::default().without_lean());
        assert_eq!(
            rep.theorem_statement.as_deref(),
            Some("theorem det3_example : det3 1 2 3 4 5 6 7 8 10 = -3 := by decide")
        );
    }

    #[test]
    fn override_literals_build_the_statement_the_caller_asked_for() {
        // A statement the kernel will reject: literals say one matrix, the rhs is
        // the determinant of another. The co-process must not silently "fix" this.
        let rep = verify_request(
            r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"literals":[9,9,9,9,9,9,9,9,9]}"#,
            &RunOptions::default().without_lean(),
        );
        // det3 of the all-nines matrix is 0, so the rhs the generator used is 0,
        // while the oracle for `values` is -3: those are separate questions.
        assert_eq!(rep.oracle.det3, Some(-3));
        assert_eq!(
            rep.theorem_statement.as_deref(),
            Some("theorem det3_example : det3 9 9 9 9 9 9 9 9 9 = 0 := by decide")
        );
    }

    #[test]
    fn cleanup_rule_holds_for_every_refusal_path() {
        // Every non-success produced without a toolchain must carry an error
        // diagnostic whose code matches the status.
        for (raw, want) in [
            ("{}", Status::InvalidRequest),
            (MILESTONE, Status::LeanMissing),
            (
                r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10],"expected":1}"#,
                Status::OracleMismatch,
            ),
        ] {
            let rep = verify_request(raw, &RunOptions::default().without_lean());
            assert_eq!(rep.status, want, "{raw}");
            assert!(
                rep.diagnostics
                    .iter()
                    .any(|d| d.level == "error" && d.code == want.as_str()),
                "{}: diagnostics = {:?}",
                want,
                rep.diagnostics
            );
        }
    }

    #[test]
    fn typed_entry_point_matches_the_textual_one() {
        let req = request::parse_request(MILESTONE).unwrap();
        let a = verify_typed(&req, &RunOptions::default().without_lean());
        let b = verify_request(MILESTONE, &RunOptions::default().without_lean());
        assert_eq!(a.status, b.status, "typed diagnostics: {:?}", a.diagnostics);
        assert_eq!(a.source_sha256, b.source_sha256);
        assert_eq!(a.theorem_statement, b.theorem_statement);
    }

    #[test]
    fn lean_diagnostics_are_parsed_from_json_lines() {
        let run = LeanRun {
            argv: vec![],
            exit_code: Some(1),
            stdout: "{\"severity\":\"error\",\"pos\":{\"line\":12,\"column\":3},\"data\":\"unexpected identifier\"}\nplain line\n".into(),
            stderr: String::new(),
            timed_out: false,
        };
        let d = lean_diagnostics(&run);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].level, "error");
        assert_eq!(d[0].message, "unexpected identifier");
        assert_eq!(d[0].position.as_deref(), Some("stdin:12:3"));
        assert_eq!(d[1].code, "lean_output");
        assert!(has_lean_error(&run));
    }
}
