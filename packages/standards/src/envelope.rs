//! Envelope parsing and format selection for the request document.

use serde_json::Value;

/// Wire formats the request document may be written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// JSON object.
    Json,
    /// TOML document.
    Toml,
}

impl Format {
    /// Guesses the format from a file name's extension (`json` / `toml`).
    pub fn from_path_extension(path: &str) -> Option<Self> {
        let ext = path.rsplit_once('.')?.1.to_ascii_lowercase();
        match ext.as_str() {
            "json" => Some(Format::Json),
            "toml" => Some(Format::Toml),
            _ => None,
        }
    }

    /// Parses `text` into a JSON value, treating TOML input as JSON-compatible data.
    ///
    /// TOML dates/times have no JSON counterpart; they are rejected rather than silently
    /// stringified, because no standard here takes a date and a stringified one would only
    /// surface later as a confusing "expected a number" error.
    pub fn parse(self, text: &str) -> Result<Value, String> {
        match self {
            Format::Json => {
                serde_json::from_str::<Value>(text).map_err(|e| format!("invalid JSON: {e}"))
            }
            Format::Toml => {
                let t: toml::Value =
                    toml::from_str(text).map_err(|e| format!("invalid TOML: {e}"))?;
                toml_to_json(t)
            }
        }
    }

    /// Guesses the format from the content.
    ///
    /// A document starting with `{` or `[` is JSON (only JSON can start a document with a
    /// bracket, so the guess is unambiguous); everything else is TOML, whose documents
    /// start with a key or a table header.
    pub fn sniff(text: &str) -> Self {
        let head = text.trim_start();
        if head.starts_with('{') || head.starts_with('[') {
            Format::Json
        } else {
            Format::Toml
        }
    }
}

fn toml_to_json(v: toml::Value) -> Result<Value, String> {
    Ok(match v {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => Value::from(i),
        toml::Value::Float(f) => {
            let n = serde_json::Number::from_f64(f)
                .ok_or_else(|| "TOML float is not finite".to_string())?;
            Value::Number(n)
        }
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Datetime(d) => {
            return Err(format!(
                "TOML date/time value `{d}` is not supported: no standard here takes a date"
            ))
        }
        toml::Value::Array(a) => Value::Array(
            a.into_iter()
                .map(toml_to_json)
                .collect::<Result<Vec<_>, String>>()?,
        ),
        toml::Value::Table(t) => {
            let mut map = serde_json::Map::new();
            for (k, v) in t {
                map.insert(k, toml_to_json(v)?);
            }
            Value::Object(map)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_guessing() {
        assert_eq!(Format::from_path_extension("a.json"), Some(Format::Json));
        assert_eq!(Format::from_path_extension("a/b.TOML"), Some(Format::Toml));
        assert_eq!(Format::from_path_extension("a.txt"), None);
        assert_eq!(Format::from_path_extension("noext"), None);
    }

    #[test]
    fn content_guessing() {
        assert_eq!(Format::sniff("  {\"a\": 1}"), Format::Json);
        assert_eq!(Format::sniff("a = 1\n"), Format::Toml);
        // Bracket-led documents are JSON; TOML never starts a document with a bracket.
        assert_eq!(Format::sniff("[1, 2]"), Format::Json);
    }

    #[test]
    fn json_and_toml_parse_to_same_value() {
        let json = r#"{"standard":"X","n":1,"f":2.5}"#;
        let toml = "standard = \"X\"\nn = 1\nf = 2.5\n";
        assert_eq!(
            Format::Json.parse(json).unwrap(),
            Format::Toml.parse(toml).unwrap()
        );
    }

    #[test]
    fn toml_datetimes_are_rejected() {
        let err = Format::Toml.parse("when = 1979-05-27\n").unwrap_err();
        assert!(err.contains("date"), "{err}");
    }

    #[test]
    fn broken_documents_report_the_format() {
        let err = Format::Json.parse("{").unwrap_err();
        assert!(err.starts_with("invalid JSON"), "{err}");
        let err = Format::Toml.parse("= 1").unwrap_err();
        assert!(err.starts_with("invalid TOML"), "{err}");
    }
}
