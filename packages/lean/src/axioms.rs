//! Parsing of `#print axioms` output and enforcement of the axiom allow-list.
//!
//! Lean's output (with `--json`) is one JSON object per line; the relevant one
//! for `#print axioms det3_example` has `"severity": "info"` and carries the
//! message text. The text came in two shapes across Lean versions, and we accept
//! both because the fourcore Lean shipping with a given toolchain is not chosen
//! by this crate:
//!
//! * modern: `'det3_example' depends on axioms: [propext, Classical.choice, Quot.sound]`
//! * older:  `'det3_example' does not depend on any axioms` / the same sentence
//!   with an explicit list.
//!
//! Anything we cannot classify is an error — never an empty allow-listed set.
//! An unparsed dependency list must not be mistaken for "no axioms".
//!
//! ## The allow-list
//!
//! Only the three axioms that Lean's own library uses for classical reasoning
//! are permitted:
//!
//! | axiom | why it is acceptable here |
//! | --- | --- |
//! | `propext` | propositional extensionality; already in Lean's kernel story for classical logic |
//! | `Classical.choice` | required by `decide` on decidable propositions in the classical fragment |
//! | `Quot.sound` | quotient soundness; needed by library lemmas the simplifier may reach |
//!
//! Everything else — `Lean.ofReduceBool`, `Lean.trustCompiler`, `sorryAx`, any
//! user-declared axiom — is a violation. `Lean.ofReduceBool` in particular is
//! what `native_decide` introduces; its presence means the kernel did not do the
//! work.

use std::collections::BTreeSet;

/// Axioms permitted in a `#print axioms` result.
pub const ALLOWED_AXIOMS: [&str; 3] = ["propext", "Classical.choice", "Quot.sound"];

/// Outcome of interpreting the `#print axioms` output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxiomReport {
    /// Every axiom name the output named, sorted and de-duplicated.
    pub found: Vec<String>,
    /// The subset that is not in [`ALLOWED_AXIOMS`].
    pub disallowed: Vec<String>,
    /// Whether the output explicitly stated "does not depend on any axioms".
    pub declared_none: bool,
    /// The raw line(s) the report was derived from, for the JSON payload.
    pub evidence: String,
}

impl AxiomReport {
    /// True when the report was obtained and contained nothing disallowed.
    pub fn is_clean(&self) -> bool {
        self.disallowed.is_empty()
    }
}

/// Why the axiom output could not be interpreted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxiomError {
    pub message: String,
}

impl std::fmt::Display for AxiomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Extract every string that could be a "message text" from `lean --json` lines.
///
/// We deliberately do not depend on the exact envelope: Lean emits
/// `{"severity":...,"pos":...,"endPos":...,"data":"<text>"}` for plain messages
/// and nests some payloads one level deeper. Both are handled by looking for the
/// `data`/`text`/`message` keys and, failing that, any string value that mentions
/// `depends on axioms` or `does not depend on any axioms`.
/// One Lean `--json` message: its text plus the severity Lean reported.
struct Message {
    text: String,
    severity: Option<String>,
}

fn message_texts(stdout: &str) -> Vec<Message> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(t) {
            Ok(v) => {
                let severity = v
                    .get("severity")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_ascii_lowercase());
                let mut texts = Vec::new();
                collect_texts(&v, &mut texts);
                for text in texts {
                    out.push(Message {
                        text,
                        severity: severity.clone(),
                    });
                }
            }
            // Not JSON (e.g. a plain-text run from a differently configured
            // toolchain): treat the line itself as a candidate.
            Err(_) => out.push(Message {
                text: t.to_string(),
                severity: None,
            }),
        }
    }
    out
}

fn collect_texts(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Array(a) => a.iter().for_each(|x| collect_texts(x, out)),
        serde_json::Value::Object(o) => {
            // Prefer the known text-bearing keys, then recurse into the rest so
            // that nested envelopes do not hide the message.
            for key in ["data", "text", "message"] {
                if let Some(x) = o.get(key) {
                    collect_texts(x, out);
                }
            }
            for (k, x) in o {
                if !matches!(k.as_str(), "data" | "text" | "message") {
                    collect_texts(x, out);
                }
            }
        }
        _ => {}
    }
}

/// Parse the axiom list out of the full stdout of a `lean --json` run.
///
/// Returns `Err` when no line mentions the axiom question at all, which is the
/// case when the file did not get far enough to run `#print axioms`.
pub fn parse_axioms(stdout: &str) -> Result<AxiomReport, AxiomError> {
    let texts = message_texts(stdout);
    let mut evidence = String::new();
    let mut found: BTreeSet<String> = BTreeSet::new();
    let mut declared_none = false;

    // Any error-severity message aborts the run, even when that particular line
    // carries no axiom text: an error somewhere else means the proof never ran.
    if let Some(message) = texts
        .iter()
        .find(|message| matches!(message.severity.as_deref(), Some("error")))
    {
        return Err(AxiomError {
            message: format!(
                "Lean reported an error while printing axioms: {}",
                message.text
            ),
        });
    }
    for message in &texts {
        let text = &message.text;
        let lower = text.to_ascii_lowercase();
        if lower.contains("does not depend on any axioms") {
            declared_none = true;
            evidence.push_str(text);
            evidence.push('\n');
            continue;
        }
        if let Some(idx) = lower.find("depends on axioms") {
            // The list follows the colon after the phrase; the phrase itself
            // contains "axioms" so we index past it, not at the first `[`.
            let rest = &text[idx..];
            let Some(open) = rest.find('[') else {
                return Err(AxiomError {
                    message: format!("axiom line has no bracketed list: {text}"),
                });
            };
            if rest[open..].find(']').is_none() {
                return Err(AxiomError {
                    message: format!("axiom list is not closed: {text}"),
                });
            }
            // Consume *every* bracketed group, not just the first: a message that
            // reads "[propext] and also [sorryAx]" must not be reduced to the
            // benign prefix.
            for inner in rest.split('[').skip(1) {
                let Some(end) = inner.find(']') else {
                    return Err(AxiomError {
                        message: format!("axiom list is not closed: {text}"),
                    });
                };
                for item in inner[..end].split(',') {
                    let name = item.trim();
                    if name.is_empty() {
                        continue;
                    }
                    found.insert(name.to_string());
                }
            }
            if rest.matches('[').count() != rest.matches(']').count() {
                return Err(AxiomError {
                    message: format!("unbalanced brackets in axiom report: {text}"),
                });
            }
            evidence.push_str(text);
            evidence.push('\n');
        }
    }

    if evidence.is_empty() {
        return Err(AxiomError {
            message: "lean output never reported the axioms of the example theorem".into(),
        });
    }
    if declared_none && !found.is_empty() {
        return Err(AxiomError {
            message: "conflicting axiom reports (both 'none' and an explicit list)".into(),
        });
    }

    let found: Vec<String> = found.into_iter().collect();
    let disallowed: Vec<String> = found
        .iter()
        .filter(|n| !ALLOWED_AXIOMS.contains(&n.as_str()))
        .cloned()
        .collect();
    Ok(AxiomReport {
        found,
        disallowed,
        declared_none,
        evidence: evidence.trim_end().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK_LINE: &str = r#"{"severity":"info","pos":{"line":12,"column":0},"endPos":{"line":12,"column":0},"data":"'det3_example' depends on axioms: [propext, Classical.choice, Quot.sound]"}"#;

    #[test]
    fn parses_the_classical_triple() {
        let r = parse_axioms(OK_LINE).unwrap();
        assert_eq!(r.found, vec!["Classical.choice", "Quot.sound", "propext"]);
        assert!(r.is_clean());
        assert!(!r.declared_none);
        assert!(r.evidence.contains("depends on axioms"));
    }

    #[test]
    fn parses_none_declared() {
        let line = r#"{"severity":"info","data":"'det3_example' does not depend on any axioms"}"#;
        let r = parse_axioms(line).unwrap();
        assert!(r.found.is_empty());
        assert!(r.is_clean());
        assert!(r.declared_none);
    }

    #[test]
    fn tolerates_other_severities_on_the_same_stream() {
        let stdout = format!(
            "{}\n{}\n{}\n",
            r#"{"severity":"warning","data":"declaration uses 'sorry'"}"#,
            OK_LINE,
            r#"{"severity":"info","data":"other chatter"}"#
        );
        let r = parse_axioms(&stdout).unwrap();
        assert!(r.is_clean());
        assert_eq!(r.found.len(), 3);
    }

    #[test]
    fn native_decide_axiom_is_a_violation() {
        let line = r#"{"severity":"info","data":"'det3_example' depends on axioms: [propext, Lean.ofReduceBool, Lean.trustCompiler]"}"#;
        let r = parse_axioms(line).unwrap();
        assert!(!r.is_clean());
        assert_eq!(
            r.disallowed,
            vec!["Lean.ofReduceBool", "Lean.trustCompiler"]
        );
    }

    #[test]
    fn sorry_axiom_is_a_violation() {
        let line = r#"{"severity":"warning","data":"'det3_example' depends on axioms: [propext, sorryAx]"}"#;
        let r = parse_axioms(line).unwrap();
        assert_eq!(r.disallowed, vec!["sorryAx"]);
    }

    #[test]
    fn any_custom_axiom_is_a_violation() {
        for name in [
            "myCheat",
            "Classical.em",
            "funext",
            "Quot.lift",
            "propext.bogus",
        ] {
            let line =
                format!(r#"{{"data":"'det3_example' depends on axioms: [propext, {name}]"}}"#);
            let r = parse_axioms(&line).unwrap();
            assert_eq!(r.disallowed, vec![name.to_string()], "{name}");
        }
    }

    #[test]
    fn allow_list_matching_is_exact_not_prefix() {
        // `Classical.choiceFoo` must not pass by sharing a prefix.
        let line =
            r#"{"data":"'det3_example' depends on axioms: [Classical.choiceFoo, propext2]"}"#;
        let r = parse_axioms(line).unwrap();
        assert_eq!(
            r.disallowed,
            vec!["Classical.choiceFoo".to_string(), "propext2".to_string()]
        );
    }

    #[test]
    fn missing_report_is_an_error_not_an_empty_allow_list() {
        let err = parse_axioms("").unwrap_err();
        assert!(err.message.contains("never reported"));
        // An error-severity message is reported for what it is instead of being
        // folded into "the report never arrived" — both are failures, but the
        // diagnostic must not hide Lean's own complaint.
        let err =
            parse_axioms(r#"{"severity":"error","data":"unknown identifier 'det3'"}"#).unwrap_err();
        assert!(err.message.contains("reported an error"), "{}", err.message);
    }

    #[test]
    fn malformed_axiom_line_is_an_error() {
        let err =
            parse_axioms(r#"{"data":"'det3_example' depends on axioms: propext"}"#).unwrap_err();
        assert!(err.message.contains("no bracketed list"));
        let err =
            parse_axioms(r#"{"data":"'det3_example' depends on axioms: [propext"}"#).unwrap_err();
        assert!(err.message.contains("not closed"));
    }

    #[test]
    fn contradictory_reports_are_an_error() {
        let stdout = concat!(
            r#"{"data":"'det3_example' does not depend on any axioms"}"#,
            "\n",
            r#"{"data":"'det3_example' depends on axioms: [propext]"}"#,
            "\n"
        );
        assert!(parse_axioms(stdout)
            .unwrap_err()
            .message
            .contains("conflicting"));
    }

    #[test]
    fn plain_text_output_is_also_parsed() {
        // A toolchain without --json support (or a hand-run file) still yields a
        // usable answer.
        let r = parse_axioms("'det3_example' depends on axioms: [propext]\n").unwrap();
        assert_eq!(r.found, vec!["propext"]);
        assert!(r.is_clean());
    }

    #[test]
    fn nested_envelope_is_unwrapped() {
        let line = r#"{"severity":"info","data":{"text":"'det3_example' depends on axioms: [propext, Quot.sound]"}}"#;
        let r = parse_axioms(line).unwrap();
        assert_eq!(r.found, vec!["Quot.sound", "propext"]);
    }
}

#[cfg(test)]
mod severity_tests {
    use super::parse_axioms;

    fn joined(first: &str, second: &str) -> String {
        let mut out = String::from(first);
        out.push('\n');
        out.push_str(second);
        out
    }

    #[test]
    fn an_error_on_any_line_aborts_even_without_axiom_text() {
        // Review R2/S3: severity used to be consulted only on the axiom-bearing
        // line, so an error elsewhere plus a clean report parsed as clean.
        let stdout = joined(
            r#"{"severity":"error","data":"unknown identifier 'X'"}"#,
            r#"{"severity":"info","data":"'x' depends on axioms: [propext]"}"#,
        );
        let err = parse_axioms(&stdout).unwrap_err();
        assert!(err.message.contains("reported an error"), "{}", err.message);
    }

    #[test]
    fn a_clean_report_after_warnings_still_passes() {
        let stdout = joined(
            r#"{"severity":"warning","data":"declaration uses 'sorry'"}"#,
            r#"{"severity":"info","data":"'x' depends on axioms: [propext]"}"#,
        );
        let report = parse_axioms(&stdout).unwrap();
        assert!(report.disallowed.is_empty());
    }
}
