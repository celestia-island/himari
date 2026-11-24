//! Request dispatch: one entry point that turns an envelope into a typed evaluation.
//!
//! The dispatcher owns everything that is *not* standard-specific:
//!
//! 1. reading `standard` and `edition` out of the envelope (looking at the raw document, so
//!    that the error can name an unknown id instead of failing on the first missing field);
//! 2. resolving the pair against the registry — an unknown id or a different edition is an
//!    explicit error, never a fallback to "closest";
//! 3. handing the *whole* document to the standard's typed request, which rejects unknown
//!    fields and unparseable numbers;
//! 4. running validation and evaluation, and flattening the result into the stable response
//!    document.
//!
//! No input reaches a panic: every failure becomes a [`StandardError`] with a stable code.

use crate::envelope::Format;
use crate::provenance::{Issue, Provenance, StandardId};
use crate::{
    gb_t_228_1_2021, gb_t_231_1_2018, gb_t_8170_2008, iso_2533_1975, iso_6506_1_2014,
    iso_6892_1_2019, STANDARDS,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

/// A request that could not be evaluated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StandardError {
    /// Stable machine-readable code.
    ///
    /// - `invalid-document` — the text is not valid JSON/TOML;
    /// - `not-an-object` — the document is valid but not a single table/object;
    /// - `missing-field` — `standard` or `edition` (or a required input field) is absent;
    /// - `unknown-standard` — the id is not in the registry (see [`crate::list`]);
    /// - `unsupported-edition` — the id is known but not for the requested edition;
    /// - `unsupported-input` — wrong field type, unknown field, or malformed value;
    /// - `invalid-input` — numeric domain error (non-positive area, out of layer, …).
    pub code: &'static str,
    /// The standard the request named, when it named a known one.
    pub standard: Option<StandardId>,
    /// The edition on the wire (as text), when it was readable.
    pub edition: Option<String>,
    /// Human-readable explanation.
    pub message: String,
}

impl StandardError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            standard: None,
            edition: None,
            message: message.into(),
        }
    }

    fn with_match(mut self, id: Option<StandardId>, edition: Option<String>) -> Self {
        self.standard = id;
        self.edition = edition;
        self
    }
}

/// A successfully evaluated request: the stable document plus the typed values.
#[derive(Clone, Debug)]
pub struct StandardOutcome {
    /// The response document (the only thing the CLI prints).
    pub document: Value,
    /// The standard that answered.
    pub standard: StandardId,
    /// The edition that answered.
    pub edition: u16,
}

/// Evaluates one request document given as text.
///
/// `format` is optional; when absent it is inferred from the content (leading `{` = JSON).
pub fn run_json(text: &str, format: Option<Format>) -> Result<StandardOutcome, StandardError> {
    let format = format.unwrap_or_else(|| Format::sniff(text));
    let value = format
        .parse(text)
        .map_err(|e| StandardError::new("invalid-document", e))?;
    run(value)
}

/// Evaluates one request document that has already been parsed into a JSON value.
pub fn run(value: Value) -> Result<StandardOutcome, StandardError> {
    let object = match &value {
        Value::Object(map) => map,
        other => {
            return Err(StandardError::new(
                "not-an-object",
                format!(
                    "the request must be a single table/object, got {}",
                    crate::jsonnum::json_kind(other)
                ),
            ))
        }
    };

    let standard_text = match object.get("standard") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(StandardError::new(
                "unsupported-input",
                format!(
                    "`standard` must be a string, got {}",
                    crate::jsonnum::json_kind(other)
                ),
            ))
        }
        None => {
            return Err(StandardError::new(
                "missing-field",
                format!(
                    "`standard` is required; known ids are listed by `--list` (e.g. {})",
                    STANDARDS[0].id
                ),
            ))
        }
    };
    let edition_text = match object.get("edition") {
        Some(Value::Number(n)) if n.is_u64() => n.as_u64().unwrap_or_default().to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(StandardError::new(
                "unsupported-input",
                format!(
                    "`edition` must be a year number or a year string, got {}",
                    crate::jsonnum::json_kind(other)
                ),
            )
            .with_match(None, None))
        }
        None => {
            return Err(StandardError::new(
                "missing-field",
                "`edition` is required: every request states the edition it was written for",
            ))
        }
    };

    let info = STANDARDS
        .iter()
        .find(|i| i.id.as_str() == standard_text)
        .ok_or_else(|| {
            let known: Vec<&str> = STANDARDS.iter().map(|i| i.id.as_str()).collect();
            StandardError::new(
                "unknown-standard",
                format!(
                    "unknown standard `{standard_text}`; known ids: {}",
                    known.join(", ")
                ),
            )
        })?;

    if edition_text != info.edition.to_string() {
        return Err(StandardError::new(
            "unsupported-edition",
            format!(
                "{} is implemented for edition {} only; the request asked for edition \
                 {edition_text}. Editions are never substituted silently",
                info.id, info.edition
            ),
        )
        .with_match(Some(info.id), Some(edition_text)));
    }

    let edition = info.edition;
    let id = info.id;
    match id {
        x if x == iso_6506_1_2014::ID => {
            let req = typed::<iso_6506_1_2014::Request>(value)
                .map_err(|e| e.with_match(Some(id), Some(edition_text.clone())))?;
            let (out, warnings) = req
                .input()
                .evaluate()
                .map_err(|m| invalid_input(m, id, edition, &edition_text))?;
            finish(
                id,
                edition,
                iso_6506_1_2014::PROVENANCE,
                iso_6506_1_2014::Values {
                    ratio: out.ratio,
                    pressure_pa: out.pressure_pa,
                    hbw: out.hbw,
                },
                warnings,
            )
        }
        x if x == iso_6892_1_2019::ID => {
            let req = typed::<iso_6892_1_2019::Request>(value)
                .map_err(|e| e.with_match(Some(id), Some(edition_text.clone())))?;
            let (out, warnings) = req
                .input()
                .evaluate()
                .map_err(|m| invalid_input(m, id, edition, &edition_text))?;
            finish(
                id,
                edition,
                iso_6892_1_2019::PROVENANCE,
                iso_6892_1_2019::Values {
                    tensile_strength_pa: out.tensile_strength_pa,
                    elongation: out.elongation,
                    percent_elongation: out.percent_elongation,
                    modulus_pa: out.modulus_pa,
                },
                warnings,
            )
        }
        x if x == gb_t_231_1_2018::ID => {
            let req = typed::<gb_t_231_1_2018::Request>(value)
                .map_err(|e| e.with_match(Some(id), Some(edition_text.clone())))?;
            let (out, warnings) = req
                .input()
                .evaluate()
                .map_err(|m| invalid_input(m, id, edition, &edition_text))?;
            finish(
                id,
                edition,
                gb_t_231_1_2018::PROVENANCE,
                gb_t_231_1_2018::Values {
                    ratio: out.ratio,
                    pressure_pa: out.pressure_pa,
                    hbw: out.hbw,
                },
                warnings,
            )
        }
        x if x == gb_t_228_1_2021::ID => {
            let req = typed::<gb_t_228_1_2021::Request>(value)
                .map_err(|e| e.with_match(Some(id), Some(edition_text.clone())))?;
            let (out, warnings) = req
                .input()
                .evaluate()
                .map_err(|m| invalid_input(m, id, edition, &edition_text))?;
            finish(
                id,
                edition,
                gb_t_228_1_2021::PROVENANCE,
                gb_t_228_1_2021::Values {
                    tensile_strength_pa: out.tensile_strength_pa,
                    elongation: out.elongation,
                    percent_elongation: out.percent_elongation,
                    modulus_pa: out.modulus_pa,
                },
                warnings,
            )
        }
        x if x == iso_2533_1975::ID => {
            let req = typed::<iso_2533_1975::Request>(value)
                .map_err(|e| e.with_match(Some(id), Some(edition_text.clone())))?;
            let (out, warnings) = req
                .input()
                .evaluate()
                .map_err(|m| invalid_input(m, id, edition, &edition_text))?;
            finish(
                id,
                edition,
                iso_2533_1975::PROVENANCE,
                iso_2533_1975::Values {
                    geometric_altitude_m: out.geometric_altitude_m,
                    geopotential_altitude_m: out.geopotential_altitude_m,
                    temperature_k: out.temperature_k,
                    pressure_pa: out.pressure_pa,
                    density_kg_m3: out.density_kg_m3,
                },
                warnings,
            )
        }
        x if x == gb_t_8170_2008::ID => {
            let req = typed::<gb_t_8170_2008::Request>(value)
                .map_err(|e| e.with_match(Some(id), Some(edition_text.clone())))?;
            let input = req.input();
            let rounded = input
                .evaluate()
                .map_err(|m| invalid_input(m, id, edition, &edition_text))?;
            finish(
                id,
                edition,
                gb_t_8170_2008::PROVENANCE,
                gb_t_8170_2008::Values {
                    rounded: rounded.text.clone(),
                    input: input.value.clone(),
                    mode: input.mode,
                    digits: input.digits.clone(),
                    tie: rounded.tie,
                    changed: rounded.changed,
                },
                Vec::new(),
            )
        }
        _ => unreachable!("registry entry {id} has no dispatch arm"),
    }
}

fn typed<T: DeserializeOwned>(value: Value) -> Result<T, StandardError> {
    serde_json::from_value(value).map_err(|e| {
        StandardError::new("unsupported-input", format!("invalid request: {e}"))
            .with_match(None, None)
    })
}

fn invalid_input(
    message: String,
    id: StandardId,
    edition: u16,
    edition_text: &str,
) -> StandardError {
    debug_assert!(!message.is_empty());
    let _ = edition;
    StandardError::new("invalid-input", message)
        .with_match(Some(id), Some(edition_text.to_string()))
}

/// Assembles the stable document: envelope fields, provenance, typed values, diagnostics.
fn finish<V: Serialize>(
    id: StandardId,
    edition: u16,
    provenance: Provenance,
    values: V,
    warnings: Vec<Issue>,
) -> Result<StandardOutcome, StandardError> {
    let values = serde_json::to_value(&values).map_err(|e| {
        StandardError::new("invalid-input", format!("could not encode values: {e}"))
            .with_match(Some(id), Some(edition.to_string()))
    })?;
    let document = serde_json::json!({
        "standard": id.as_str(),
        "edition": edition.to_string(),
        "provenance": provenance,
        "values": values,
        "warnings": warnings,
        "errors": Vec::<Issue>::new(),
    });
    Ok(StandardOutcome {
        document,
        standard: id,
        edition,
    })
}

/// The registry as data, for `--list` and for envelope checks.
pub fn list() -> Vec<Value> {
    STANDARDS
        .iter()
        .map(|i| {
            serde_json::json!({
                "standard": i.id.as_str(),
                "edition": i.edition.to_string(),
                "summary": i.summary,
                "provenance": i.provenance,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(text: &str) -> StandardError {
        run_json(text, None).expect_err(text)
    }

    #[test]
    fn unknown_standard_names_the_known_ids() {
        let e = err(r#"{"standard":"ISO-9999","edition":2014}"#);
        assert_eq!(e.code, "unknown-standard");
        assert!(e.message.contains("ISO-6506-1"), "{}", e.message);
    }

    #[test]
    fn wrong_edition_is_not_substituted() {
        let e = err(r#"{"standard":"ISO-6506-1","edition":2024,"force":1}"#);
        assert_eq!(e.code, "unsupported-edition");
        assert_eq!(e.edition.as_deref(), Some("2024"));
        assert_eq!(e.standard, Some(iso_6506_1_2014::ID));
    }

    #[test]
    fn missing_envelope_fields_are_reported() {
        assert_eq!(err(r#"{"edition":2014}"#).code, "missing-field");
        assert_eq!(err(r#"{"standard":"ISO-6506-1"}"#).code, "missing-field");
    }

    #[test]
    fn unknown_input_fields_are_rejected() {
        let e = err(
            r#"{"standard":"ISO-6506-1","edition":2014,"force":29420,"ball_diameter":0.01,
                "indentation_diameter":0.004,"typography":true}"#,
        );
        assert_eq!(e.code, "unsupported-input");
        assert!(e.message.contains("typography"), "{}", e.message);
    }

    #[test]
    fn numeric_parse_errors_are_structured() {
        let e = err(
            r#"{"standard":"ISO-6506-1","edition":2014,"force":"lots","ball_diameter":0.01,
                "indentation_diameter":0.004}"#,
        );
        assert_eq!(e.code, "unsupported-input");
        let e = err(r#"{"standard":"ISO-2533","edition":1975,"geometric_altitude_m":true}"#);
        assert_eq!(e.code, "unsupported-input");
    }

    #[test]
    fn domain_errors_are_structured() {
        let e = err(
            r#"{"standard":"ISO-6506-1","edition":2014,"force":29420,"ball_diameter":0.01,
                "indentation_diameter":0.01}"#,
        );
        assert_eq!(e.code, "invalid-input");
        let e = err(r#"{"standard":"ISO-2533","edition":1975,"geometric_altitude_m":12000}"#);
        assert_eq!(e.code, "invalid-input");
    }

    #[test]
    fn document_shape_errors() {
        // Valid documents of the wrong shape are reported as such; broken syntax is a
        // parse error; a table without the envelope keys is a missing-field error.
        // A bracket-led document is sniffed as JSON and rejected for its shape.
        assert_eq!(err("[1, 2]").code, "not-an-object");
        assert_eq!(err("[]").code, "not-an-object");
        assert_eq!(err("{").code, "invalid-document");
        assert_eq!(err("{\"a\": }").code, "invalid-document");
        assert_eq!(
            err(r#"{"standard":1,"edition":2014}"#).code,
            "unsupported-input"
        );
    }

    #[test]
    fn successful_response_has_the_stable_shape() {
        let out = run_json(
            r#"{"standard":"ISO-6506-1","edition":2014,"force":29420,
                "ball_diameter":0.01,"indentation_diameter":0.004}"#,
            None,
        )
        .unwrap();
        let doc = &out.document;
        for key in [
            "standard",
            "edition",
            "provenance",
            "values",
            "warnings",
            "errors",
        ] {
            assert!(doc.get(key).is_some(), "missing {key}");
        }
        assert_eq!(doc["edition"], "2014");
        assert_eq!(doc["errors"], serde_json::json!([]));
        assert_eq!(doc["warnings"], serde_json::json!([]));
        let hbw = doc["values"]["hbw"].as_f64().unwrap();
        assert!((hbw - 228.831_198_210_826_27).abs() < 1e-9);
        assert_eq!(out.edition, 2014);
    }

    #[test]
    fn edition_may_be_given_as_a_string() {
        let out = run_json(
            r#"{"standard":"ISO-2533","edition":"1975","geometric_altitude_m":0}"#,
            None,
        )
        .unwrap();
        assert_eq!(out.document["edition"], "1975");
    }

    #[test]
    fn list_covers_every_registry_entry() {
        let list = list();
        assert_eq!(list.len(), STANDARDS.len());
        for (entry, info) in list.iter().zip(STANDARDS) {
            assert_eq!(entry["standard"], info.id.as_str());
            assert_eq!(entry["edition"], info.edition.to_string());
        }
    }
}
