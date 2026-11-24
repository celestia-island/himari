//! The structured JSON result of one co-process run.
//!
//! The shape is deliberately flat and boolean-free where correctness matters:
//! `status` is the single source of truth, `verified` is derived from it and can
//! never disagree with it, and an `oracle` block states independently what the
//! Rust side computed. A consumer should branch on `status`, keep `verified` only
//! as a redundant assertion, and treat `exit_code != 0` as "not verified".
//!
//! ```json
//! {
//!   "schema": "himari-lean/co-process-result",
//!   "status": "success",
//!   "verified": true,
//!   "exit_code": 0,
//!   "request": { "theorem": "det3", "values": [1,2,3,4,5,6,7,8,10] },
//!   "theorem": {
//!     "name": "det3_example",
//!     "statement": "theorem det3_example : ...",
//!     "source_sha256": "...",
//!     "rhs": "-3"
//!   },
//!   "lean": { "version": "Lean (version 4.x, ...)", "argv": ["--stdin", ...] },
//!   "axioms": { "parsed": true, "found": ["..."], "disallowed": [], "allowed": [...] },
//!   "oracle": { "engine": "rust-i128-permutation", "det3": "-3", "expected": "-3", "agrees": true },
//!   "diagnostics": [ { "level": "info", "code": "...", "message": "..." } ]
//! }
//! ```

use serde_json::{json, Map, Value};

use crate::axioms::ALLOWED_AXIOMS;
use crate::source;
use crate::status::Status;

/// Wire identifier for this result shape.
pub const SCHEMA: &str = "himari-lean/co-process-result";

/// One diagnostic line, matching the severity vocabulary used by Lean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// `error` / `warning` / `info`.
    pub level: String,
    /// Stable machine code, e.g. `lean_missing`, `forbidden_input`.
    pub code: String,
    pub message: String,
    /// `file:line:column` when Lean supplied a position.
    pub position: Option<String>,
}

impl Diagnostic {
    pub fn new(level: &str, code: &str, message: impl Into<String>) -> Self {
        Diagnostic {
            level: level.to_string(),
            code: code.to_string(),
            message: message.into(),
            position: None,
        }
    }

    pub fn error(code: &str, message: impl Into<String>) -> Self {
        Diagnostic::new("error", code, message)
    }

    pub fn info(code: &str, message: impl Into<String>) -> Self {
        Diagnostic::new("info", code, message)
    }

    pub fn warning(code: &str, message: impl Into<String>) -> Self {
        Diagnostic::new("warning", code, message)
    }

    pub fn at(mut self, position: impl Into<String>) -> Self {
        self.position = Some(position.into());
        self
    }

    fn to_json(&self) -> Value {
        let mut o = Map::new();
        o.insert("level".into(), json!(self.level));
        o.insert("code".into(), json!(self.code));
        o.insert("message".into(), json!(self.message));
        if let Some(p) = &self.position {
            o.insert("position".into(), json!(p));
        }
        Value::Object(o)
    }
}

/// Independent record of what the Rust oracle computed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OracleReport {
    /// The value the oracle produced, if it produced one.
    pub det3: Option<i128>,
    /// The right-hand side claimed by whoever built the request, if any.
    pub expected: Option<i128>,
    /// Whether the two agree. `None` when there was nothing to compare.
    pub agrees: Option<bool>,
    /// Set when the oracle itself could not answer.
    pub failure: Option<String>,
}

impl OracleReport {
    pub fn to_json(&self) -> Value {
        json!({
            "engine": "rust-i128-permutation",
            "det3": self.det3.map(|v| v.to_string()),
            "expected": self.expected.map(|v| v.to_string()),
            "agrees": self.agrees,
            "failure": self.failure,
        })
    }
}

/// One run's full result.
#[derive(Clone, Debug)]
pub struct Report {
    pub status: Status,
    /// The request echoed back (`theorem` + `values`), so a result is never
    /// detached from its input.
    pub request: Value,
    /// The theorem name, once known.
    pub theorem_name: Option<String>,
    /// The text of the example theorem statement, once generated.
    pub theorem_statement: Option<String>,
    /// SHA-256 (hex) of the complete generated source.
    pub source_sha256: Option<String>,
    /// The complete generated source. Only included when asked for, because it
    /// is not needed to interpret the result.
    pub source: Option<String>,
    pub lean_version: Option<String>,
    pub lean_argv: Option<Vec<String>>,
    pub oracle: OracleReport,
    pub axioms: Option<crate::axioms::AxiomReport>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    /// Start a report for a given request echo.
    pub fn new(request: Value) -> Self {
        Report {
            status: Status::InvalidRequest,
            request,
            theorem_name: None,
            theorem_statement: None,
            source_sha256: None,
            source: None,
            lean_version: None,
            lean_argv: None,
            oracle: OracleReport {
                det3: None,
                expected: None,
                agrees: None,
                failure: None,
            },
            axioms: None,
            diagnostics: Vec::new(),
        }
    }

    /// Attach generated source, recording its hash and the statement text.
    pub fn with_source(mut self, src: &str) -> Self {
        self.source_sha256 = Some(himari_loader::hex_digest(src.as_bytes()));
        self.theorem_name = Some(source::EXAMPLE_THEOREM.to_string());
        self.theorem_statement = statement_line(src);
        self.source = Some(src.to_string());
        self
    }

    pub fn diag(mut self, d: Diagnostic) -> Self {
        self.diagnostics.push(d);
        self
    }

    /// The wire form.
    pub fn to_json(&self) -> Value {
        let mut o = Map::new();
        o.insert("schema".into(), json!(SCHEMA));
        o.insert("status".into(), json!(self.status.as_str()));
        // `verified` is recomputed from `status`, never stored, so the two can
        // not drift apart.
        o.insert("verified".into(), json!(self.status.is_verified()));
        o.insert("exit_code".into(), json!(self.status.exit_code()));
        o.insert("request".into(), self.request.clone());

        let mut th = Map::new();
        th.insert(
            "name".into(),
            match &self.theorem_name {
                Some(n) => json!(n),
                None => Value::Null,
            },
        );
        th.insert(
            "statement".into(),
            match &self.theorem_statement {
                Some(s) => json!(s),
                None => Value::Null,
            },
        );
        th.insert(
            "source_sha256".into(),
            match &self.source_sha256 {
                Some(h) => json!(h),
                None => Value::Null,
            },
        );
        if let Some(src) = &self.source {
            th.insert("source".into(), json!(src));
        }
        o.insert("theorem".into(), Value::Object(th));

        let mut lean = Map::new();
        lean.insert(
            "version".into(),
            match &self.lean_version {
                Some(v) => json!(v),
                None => Value::Null,
            },
        );
        lean.insert(
            "argv".into(),
            match &self.lean_argv {
                Some(a) => json!(a),
                None => Value::Null,
            },
        );
        o.insert("lean".into(), Value::Object(lean));

        let ax = match &self.axioms {
            Some(a) => json!({
                "parsed": true,
                "found": a.found,
                "disallowed": a.disallowed,
                "allowed": ALLOWED_AXIOMS,
                "declared_none": a.declared_none,
                "evidence": a.evidence,
            }),
            None => json!({
                "parsed": false,
                "found": Value::Null,
                "disallowed": Value::Null,
                "allowed": ALLOWED_AXIOMS,
            }),
        };
        o.insert("axioms".into(), ax);
        o.insert("oracle".into(), self.oracle.to_json());
        o.insert(
            "diagnostics".into(),
            Value::Array(self.diagnostics.iter().map(Diagnostic::to_json).collect()),
        );
        Value::Object(o)
    }

    /// Pretty-printed wire form, newline-terminated.
    pub fn to_json_pretty(&self) -> String {
        let mut s = serde_json::to_string_pretty(&self.to_json()).unwrap_or_else(|e| {
            format!(
                "{{\"status\":\"{}\",\"error\":\"serialisation failed: {e}\"}}",
                self.status
            )
        });
        s.push('\n');
        s
    }
}

/// The `theorem ... := by decide` line of a generated source, if present.
pub fn statement_line(src: &str) -> Option<String> {
    src.lines()
        .find(|l| l.starts_with("theorem "))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_report_round_trips_through_json() {
        let src = source::generate_det3_source(
            &source::MILESTONE_VALUES,
            &source::MILESTONE_VALUES,
            source::MILESTONE_DET,
        );
        let r =
            Report::new(json!({"theorem":"det3","values":[1,2,3,4,5,6,7,8,10]})).with_source(&src);
        let mut r = r;
        r.status = Status::Success;
        r.lean_version = Some("Lean (version 4.11.0, x86_64-unknown-linux-gnu, commit abc)".into());
        r.lean_argv = Some(lean_argv());
        r.axioms = Some(crate::axioms::AxiomReport {
            found: vec![
                "Classical.choice".into(),
                "Quot.sound".into(),
                "propext".into(),
            ],
            disallowed: vec![],
            declared_none: false,
            evidence: "'det3_example' depends on axioms: [propext, Classical.choice, Quot.sound]"
                .into(),
        });
        r.oracle = OracleReport {
            det3: Some(source::MILESTONE_DET),
            expected: Some(source::MILESTONE_DET),
            agrees: Some(true),
            failure: None,
        };

        let text = r.to_json_pretty();
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["status"], "success");
        assert_eq!(v["verified"], true);
        assert_eq!(v["exit_code"], 0);
        assert_eq!(
            v["theorem"]["statement"],
            "theorem det3_example : det3 1 2 3 4 5 6 7 8 10 = -3 := by decide"
        );
        assert_eq!(
            v["theorem"]["source_sha256"],
            himari_loader::hex_digest(src.as_bytes())
        );
        assert_eq!(v["oracle"]["det3"], "-3");
        assert_eq!(v["axioms"]["found"].as_array().unwrap().len(), 3);
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn lean_missing_report_is_loud_and_never_verified() {
        let mut r = Report::new(json!({"theorem":"det3","values":[1,2,3,4,5,6,7,8,10]})).diag(
            Diagnostic::error("lean_missing", "no `lean` executable found on PATH"),
        );
        r.status = Status::LeanMissing;
        let v = serde_json::from_str::<Value>(&r.to_json_pretty()).unwrap();
        assert_eq!(v["status"], "lean_missing");
        assert_eq!(v["verified"], false);
        assert_ne!(v["exit_code"].as_u64().unwrap(), 0);
        assert_eq!(v["lean"]["version"], Value::Null);
        assert_eq!(v["axioms"]["parsed"], false);
        assert_eq!(v["diagnostics"][0]["code"], "lean_missing");
        assert_eq!(v["diagnostics"][0]["level"], "error");
    }

    fn lean_argv() -> Vec<String> {
        crate::runner::LeanToolchain::check_argv(400_000, 2048)
    }

    #[test]
    fn statement_line_finds_the_theorem_of_a_generated_source() {
        let src = source::generate_det3_source(
            &source::MILESTONE_VALUES,
            &source::MILESTONE_VALUES,
            source::MILESTONE_DET,
        );
        assert_eq!(
            statement_line(&src).as_deref(),
            Some("theorem det3_example : det3 1 2 3 4 5 6 7 8 10 = -3 := by decide")
        );
        assert_eq!(statement_line("def x := 1\n"), None);
    }
}
