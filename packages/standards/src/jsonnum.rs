//! Parameter deserialization shared by the numeric standards.
//!
//! Standards take parameters in SI base units; the number may arrive as
//!  - a native JSON/TOML number (integral or fractional — never as an `f64` literal that
//!    the format parser could round), or
//!  - a plain decimal string such as `"-60.75"` or `"2.94e4"`.
//!
//! Everything else (booleans, arrays, objects, NaN/infinity spellings, empty strings) is
//! rejected with an error naming the offending field. Rejecting is the point: a silently
//! coerced parameter would produce a plausible-looking number with the wrong magnitude.
//!
//! Note the division of labour with the decimal-exact standard
//! ([`crate::gb_t_8170_2008`]): that one uses its own string-only type because a value
//! must never pass through `f64` before a rounding decision is made. Here `f64` is fine —
//! every check below is a range/tolerance comparison.

use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use serde::Serialize;
use std::fmt;
use std::str::FromStr;

/// A finite numeric parameter.
///
/// Serializes as a JSON number / TOML float; deserializes from a number or a decimal
/// string.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Param(pub f64);

impl Param {
    /// The bare value.
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl From<f64> for Param {
    fn from(v: f64) -> Self {
        Self(v)
    }
}

impl Serialize for Param {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_f64(self.0)
    }
}

/// Parses a parameter from text: plain decimal or scientific notation, no NaN/infinity.
fn parse_text(text: &str) -> Result<f64, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("is empty".to_string());
    }
    let v = f64::from_str(t).map_err(|_| format!("{text:?} is not a decimal number"))?;
    if !v.is_finite() {
        return Err(format!("{text:?} is not a finite number"));
    }
    Ok(v)
}

impl<'de> Deserialize<'de> for Param {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(ParamVisitor)
    }
}

struct ParamVisitor;

impl<'de> Visitor<'de> for ParamVisitor {
    type Value = Param;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a finite number or a decimal string, e.g. 0.01 or \"0.01\"")
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Param, E> {
        if v.is_finite() {
            Ok(Param(v))
        } else {
            Err(E::custom("value is not finite"))
        }
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Param, E> {
        Ok(Param(v as f64))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Param, E> {
        Ok(Param(v as f64))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Param, E> {
        parse_text(v).map(Param).map_err(E::custom)
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Param, E> {
        self.visit_str(&v)
    }

    // serde_json's bundled `Number` type: route through its textual form so the exact
    // digits are preserved for integer and fractional literals alike.
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Param, A::Error> {
        use serde::de::value::MapAccessDeserializer;
        let unknown = serde_json::Value::deserialize(MapAccessDeserializer::new(&mut map))?;
        match unknown {
            serde_json::Value::Number(n) => parse_text(&n.to_string())
                .map(Param)
                .map_err(de::Error::custom),
            other => Err(de::Error::custom(format!(
                "expected a number, got {}",
                json_kind(&other)
            ))),
        }
    }

    fn visit_unit<E: de::Error>(self) -> Result<Param, E> {
        Err(E::custom("expected a number, got null"))
    }

    fn visit_none<E: de::Error>(self) -> Result<Param, E> {
        Err(E::custom("expected a number, got none"))
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<Param, E> {
        Err(E::custom("expected a number, got a boolean"))
    }
}

/// Short JSON kind name for error messages.
pub(crate) fn json_kind(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "a boolean",
        serde_json::Value::Number(_) => "a number",
        serde_json::Value::String(_) => "a string",
        serde_json::Value::Array(_) => "an array",
        serde_json::Value::Object(_) => "an object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, serde::Deserialize)]
    struct Holder {
        x: Param,
    }

    fn from_json(s: &str) -> Result<Holder, serde_json::Error> {
        serde_json::from_str(s)
    }

    #[test]
    fn json_numbers_of_every_shape_are_accepted() {
        assert_eq!(from_json(r#"{"x": 29420}"#).unwrap().x, Param(29420.0));
        assert_eq!(from_json(r#"{"x": 0.0035}"#).unwrap().x, Param(0.0035));
        assert_eq!(from_json(r#"{"x": -60.75}"#).unwrap().x, Param(-60.75));
        assert_eq!(from_json(r#"{"x": 2.94e4}"#).unwrap().x, Param(29400.0));
    }

    #[test]
    fn decimal_strings_are_accepted() {
        assert_eq!(from_json(r#"{"x": "0.01"}"#).unwrap().x, Param(0.01));
        assert_eq!(from_json(r#"{"x": " 1e-3 "}"#).unwrap().x, Param(1e-3));
    }

    #[test]
    fn non_numbers_are_rejected_with_a_reason() {
        for bad in [
            r#"{"x": null}"#,
            r#"{"x": true}"#,
            r#"{"x": [1]}"#,
            r#"{"x": {"v": 1}}"#,
            r#"{"x": ""}"#,
            r#"{"x": "abc"}"#,
            r#"{"x": "NaN"}"#,
            r#"{"x": "1e999"}"#,
        ] {
            let err = from_json(bad).expect_err(bad);
            let msg = err.to_string();
            assert!(
                msg.contains("expected") || msg.contains("is not") || msg.contains("is empty"),
                "unhelpful error for {bad}: {msg}"
            );
        }
    }

    #[test]
    fn toml_accepts_integer_and_float_forms() {
        let a: Holder = toml::from_str("x = 29420").unwrap();
        let b: Holder = toml::from_str("x = 0.0035").unwrap();
        let c: Holder = toml::from_str("x = \"0.0035\"").unwrap();
        assert_eq!(
            (a.x, b.x, c.x),
            (Param(29420.0), Param(0.0035), Param(0.0035))
        );
        assert!(toml::from_str::<Holder>("x = true").is_err());
        assert!(toml::from_str::<Holder>("x = 1979-05-27").is_err());
    }
}
/// An edition year read from the wire: number or decimal-digit string, no floats.
///
/// Kept apart from [`Param`] because the envelope compares it exactly against the
/// registry: `1975.0` or `1975.5` must not be silently accepted as `1975`.
pub(crate) fn deserialize_edition<'de, D: Deserializer<'de>>(d: D) -> Result<u16, D::Error> {
    d.deserialize_any(EditionVisitor)
}

struct EditionVisitor;

impl<'de> Visitor<'de> for EditionVisitor {
    type Value = u16;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an edition year such as 2014 or \"2014\"")
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<u16, E> {
        u16::try_from(v).map_err(|_| E::custom(format!("edition {v} is not a valid year")))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<u16, E> {
        u16::try_from(v).map_err(|_| E::custom(format!("edition {v} is not a valid year")))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<u16, E> {
        Err(E::custom(format!(
            "edition {v} must be written as a whole year (2014, not 2014.0) or a string"
        )))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<u16, E> {
        let t = v.trim();
        if t.is_empty() || !t.bytes().all(|b| b.is_ascii_digit()) {
            return Err(E::custom(format!("edition {v:?} is not a year")));
        }
        t.parse::<u16>()
            .map_err(|_| E::custom(format!("edition {v:?} is not a valid year")))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<u16, E> {
        self.visit_str(&v)
    }
}

#[cfg(test)]
mod edition_tests {
    use super::*;

    #[derive(Debug, serde::Deserialize)]
    struct Holder {
        #[serde(deserialize_with = "deserialize_edition")]
        edition: u16,
    }

    #[test]
    fn edition_accepts_numbers_and_digit_strings() {
        for src in [r#"{"edition": 2014}"#, r#"{"edition": "2014"}"#] {
            assert_eq!(serde_json::from_str::<Holder>(src).unwrap().edition, 2014);
        }
        let h: Holder = toml::from_str("edition = 1975").unwrap();
        assert_eq!(h.edition, 1975);
    }

    #[test]
    fn edition_rejects_non_years() {
        for src in [
            r#"{"edition": 2014.0}"#,
            r#"{"edition": "20x4"}"#,
            r#"{"edition": ""}"#,
            r#"{"edition": true}"#,
            r#"{"edition": -1}"#,
        ] {
            assert!(serde_json::from_str::<Holder>(src).is_err(), "{src}");
        }
    }
}
