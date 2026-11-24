//! Decimal rounding as specified by GB/T 8170:2008 ("Rules of rounding off for numerical
//! values and expression and determination of limiting values"), clause 4.
//!
//! # Why strings, not floats
//!
//! A rounding decision must be made on the exact decimal value the user wrote. `60.25` is
//! not representable in binary floating point (it is stored as
//! `60.249999999999992894…`), so any `f64`-based "round half up/even" routine silently
//! rounds some ties the wrong way. This module therefore takes the value as a decimal
//! **string**, performs the rounding on the decimal digits (`i128` arithmetic on scaled
//! integers plus an explicit tie test), and returns the result as a decimal string. `f64`
//! never takes part in a rounding decision.
//!
//! # Inputs
//!
//! `value` — a plain decimal string:
//!
//! - optional `+`/`-` sign, at least one digit, optional `.` and fractional digits;
//! - no exponent (`1e3` / `6.025E1`), no `NaN`/`Inf`, no thousands separators, no
//!   whitespace inside the number;
//! - at most [`MAX_FRACTION_DIGITS`] fractional digits (so the scaled integer fits `i128`).
//!
//! Anything else is rejected with a message that names the offending part.
//!
//! `digits` — the retained precision, as a JSON number or a decimal string of digits:
//!
//! - `mode = "decimal"` (default): `digits >= 0` is a number of decimal places, so
//!   `digits = 1` rounds `60.25` to one decimal place;
//! - `mode = "unit"`: `digits` names the place of the retained digit as a multiple of the
//!   last retained digit, i.e. `0.5` means "multiples of 0.5" and `0.2` means "multiples of
//!   0.2" — the "unit of the last digit" convention. Both values must be positive; the
//!   spacing is resolved to a decimal scale and a decimal step.
//!
//! The tie rule is **round-half-even** in both modes (clause 4's "round five, taking the
//! even digit"): an exact tie goes to the retained digit whose last decimal is even.
//!
//! # Examples (also covered by the tests)
//!
//! | value | digits | mode | result | why |
//! |---|---|---|---|---|
//! | `60.25` | `1` | decimal | `60.2` | exact tie; 2 is even |
//! | `60.35` | `1` | decimal | `60.4` | exact tie; 4 is even |
//! | `60.38` | `1` | decimal | `60.4` | not a tie; 60.4 is nearer |
//! | `-60.75` | `1` | decimal | `-60.8` | exact tie; 8 is even |
//! | `60.25` | `0.5` | unit | `60.0` | tie between 60.0 and 60.5; index 120 is even |
//! | `60.3` | `0.2` | unit | `60.4` | tie between 60.2 and 60.4; index 302 is even |
//! | `60.2` | `0.2` | unit | `60.2` | already a multiple of 0.2 |
//!
//! The retained scale is preserved in the output (`60` at one decimal place prints as
//! `60.0`), because the number of retained digits is itself information.

use crate::provenance::{Issue, Provenance, StandardId};
use crate::StandardInfo;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Canonical standard id.
pub const ID: StandardId = StandardId::new("GB-T-8170");

/// Edition year accepted on the wire.
pub const EDITION: u16 = 2008;

/// Most fractional digits accepted for `value`.
///
/// Keeps the scaled integer inside `i128` (`10^30` still has ~38 extra decimal digits of
/// headroom for the integer part).
pub const MAX_FRACTION_DIGITS: u32 = 30;

/// Largest accepted value of `digits` in `mode = "decimal"`.
pub const MAX_DECIMAL_DIGITS: u32 = 30;

/// Where the rules come from. See [`Provenance`] for what is *not* claimed.
pub const PROVENANCE: Provenance = Provenance {
    standard: ID,
    edition: "2008",
    formula_source: "GB/T 8170-2008, clause 4 (rounding of decimal values: round-half-even \
                     and the unit-of-last-digit conventions)",
    notes: "Implements decimal-digit rounding only, on decimal STRINGS: no binary floating \
            point takes part in a rounding decision, and exponent notation is rejected. The \
            standard also covers determining limiting values, comparing and expressing \
            measured values with uncertainty, and the treatment of negative numbers; those \
            clauses are NOT implemented. Which retained precision (decimal places vs unit \
            of the last digit) applies to a given measurement is the caller's decision, \
            not the crate's. Not a conformance claim.",
};

/// Registry entry.
pub const INFO: StandardInfo = StandardInfo {
    id: ID,
    edition: EDITION,
    provenance: PROVENANCE,
    summary: "Exact decimal rounding (round-half-even), decimal-place or unit-of-last-digit",
};

/// How `digits` is interpreted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// `digits` is a number of decimal places (`1` = one decimal place).
    #[default]
    Decimal,
    /// `digits` is the unit of the last retained digit (`0.5` = multiples of 0.5).
    Unit,
}

/// A canonical decimal: sign, decimal digits with no leading/trailing zeros in `digits`,
/// and a scale (number of fractional digits).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decimal {
    /// `true` for negative values; a zero result always carries `false`.
    negative: bool,
    /// Magnitude, scaled by `10^scale`. Normalised: never has trailing zeros (except 0).
    magnitude: i128,
    /// Number of fractional digits implied by `magnitude`.
    scale: u32,
}

impl Decimal {
    /// Whether the value is negative.
    pub fn is_negative(&self) -> bool {
        self.negative
    }

    /// The scaled magnitude.
    pub fn magnitude(&self) -> i128 {
        self.magnitude
    }

    /// The number of fractional digits.
    pub fn scale(&self) -> u32 {
        self.scale
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.negative { "-" } else { "" };
        let digits = self.magnitude.to_string();
        let scale = self.scale as usize;
        if scale == 0 {
            return write!(f, "{sign}{digits}");
        }
        if digits.len() <= scale {
            let pad = "0".repeat(scale - digits.len());
            return write!(f, "{sign}0.{pad}{digits}");
        }
        let split = digits.len() - scale;
        write!(f, "{sign}{}.{}", &digits[..split], &digits[split..])
    }
}

fn pow10(n: u32) -> Option<i128> {
    10i128.checked_pow(n)
}

/// Parses a plain decimal string into a [`Decimal`].
///
/// Rejects exponent notation, `NaN`/`Inf`, digit separators, empty strings and values with
/// more than [`MAX_FRACTION_DIGITS`] fractional digits.
pub fn parse_decimal(text: &str) -> Result<Decimal, String> {
    let s = text.trim();
    if s.is_empty() {
        return Err("value is empty".to_string());
    }
    if s.len() != text.len() {
        return Err(format!(
            "value {text:?} has leading or trailing whitespace; pass the number only"
        ));
    }
    let (negative, body) = match s.as_bytes()[0] {
        b'-' => (true, &s[1..]),
        b'+' => (false, &s[1..]),
        _ => (false, s),
    };
    if body.is_empty() {
        return Err(format!("value {text:?} has a sign but no digits"));
    }
    let lower = body.to_ascii_lowercase();
    if lower.contains('e') {
        return Err(format!(
            "value {text:?} uses exponent notation; write the number out in full \
             (e.g. 6025 instead of 6.025e3) so the rounding decision is unambiguous"
        ));
    }
    if lower.contains("nan") || lower.contains("inf") {
        return Err(format!("value {text:?} is not a finite decimal number"));
    }
    if lower.contains('_') || lower.contains('\'') {
        return Err(format!(
            "value {text:?} contains digit separators; use a plain decimal number"
        ));
    }
    let (int_part, frac_part) = match body.split_once('.') {
        Some((i, f)) => (i, f),
        None => (body, ""),
    };
    if frac_part.contains('.') {
        return Err(format!("value {text:?} has more than one decimal point"));
    }
    let all_digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    if int_part.is_empty() && frac_part.is_empty() {
        return Err(format!("value {text:?} has no digits"));
    }
    if !all_digits(int_part) || !all_digits(frac_part) {
        return Err(format!(
            "value {text:?} is not a plain decimal number (digits, optional '.', digits)"
        ));
    }
    let fraction_digits = frac_part.len() as u32;
    if fraction_digits > MAX_FRACTION_DIGITS {
        return Err(format!(
            "value {text:?} has {fraction_digits} fractional digits; at most \
             {MAX_FRACTION_DIGITS} are supported"
        ));
    }
    let mut magnitude: i128 = 0;
    for b in int_part.bytes().chain(frac_part.bytes()) {
        magnitude = magnitude
            .checked_mul(10)
            .and_then(|m| m.checked_add(i128::from(b - b'0')))
            .ok_or_else(|| format!("value {text:?} is too long to represent exactly"))?;
    }
    // Normalise: strip trailing zeros from the scaled magnitude by reducing the scale.
    let mut scale = fraction_digits;
    while scale > 0 && magnitude % 10 == 0 {
        magnitude /= 10;
        scale -= 1;
    }
    Ok(Decimal {
        negative: negative && magnitude != 0,
        magnitude,
        scale,
    })
}

/// A rounding place expressed as `(scale, step)`: round the value to an integer multiple
/// of `step * 10^-scale`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    /// Decimal places of the step.
    pub scale: u32,
    /// Numerator of the step at one decimal place (`0.5` gives (1, 5), `0.2` gives (1, 2)).
    pub step: i128,
}

impl Place {
    /// Resolves the request's `(mode, digits)` into a rounding place.
    fn resolve(mode: Mode, digits: &Decimal) -> Result<Self, String> {
        match mode {
            Mode::Decimal => {
                if digits.is_negative() {
                    return Err(format!(
                        "digits = {digits} is negative: the number of decimal places cannot \
                         be negative"
                    ));
                }
                if digits.scale() != 0 {
                    return Err(format!(
                        "digits = {digits} is not an integer; in mode \"decimal\" it is a \
                         number of decimal places. For a unit of the last digit use \
                         mode \"unit\""
                    ));
                }
                let places = u32::try_from(digits.magnitude()).map_err(|_| String::from(""))?;
                if places > MAX_DECIMAL_DIGITS {
                    return Err(format!(
                        "digits = {digits} exceeds the supported maximum of {MAX_DECIMAL_DIGITS} \
                         decimal places"
                    ));
                }
                Ok(Place {
                    scale: places,
                    step: 1,
                })
            }
            Mode::Unit => {
                if digits.is_negative() || digits.magnitude() == 0 {
                    return Err(format!(
                        "digits = {digits} must be positive in mode \"unit\" (it is the unit \
                         of the last retained digit, e.g. 0.5 or 0.2)"
                    ));
                }
                // A unit of `u` is only meaningful if rounding to it can be expressed as
                // rounding to a fixed decimal place. `0.5` -> (1, 5) and `0.2` -> (1, 2) are
                // the conventions this crate implements; `0.25`/`0.01`-style units are
                // accepted only when they are a single fractional digit scaled by 1..=9.
                if digits.scale() != 1 {
                    return Err(format!(
                        "unsupported unit {digits}: the unit of the last digit must be a \
                         single-decimal unit such as 0.5 or 0.2"
                    ));
                }
                let step = digits.magnitude();
                if step == 0 || step % 10 == 0 {
                    return Err(format!(
                        "unsupported unit {digits}: a trailing zero means the unit is not a \
                         single-digit multiple"
                    ));
                }
                Ok(Place { scale: 1, step })
            }
        }
    }
}

/// The outcome of one rounding.
#[derive(Clone, Debug, PartialEq)]
pub struct Rounded {
    /// Canonical text of the rounded value (`-60.8`, `60.0`, `0`).
    pub text: String,
    /// The rounded value as a decimal.
    pub decimal: Decimal,
    /// Whether the input was exactly halfway between two representable results.
    pub tie: bool,
    /// Whether rounding changed the value at all.
    pub changed: bool,
}

/// Typed input of the rounding calculator.
#[derive(Clone, Debug, PartialEq)]
pub struct Input {
    /// The value to round, as the decimal text the caller wrote.
    pub value: String,
    /// Retained precision: decimal places or unit of the last digit, per `mode`.
    pub digits: String,
    /// How `digits` is interpreted.
    pub mode: Mode,
}

impl Input {
    /// Rounds `value` to integer multiples of `step * 10^-scale` with round-half-even.
    pub fn evaluate(&self) -> Result<Rounded, String> {
        let value = parse_decimal(&self.value)?;
        let digits = parse_decimal(&self.digits)?;
        let place = Place::resolve(self.mode, &digits)?;
        let result = round_half_even(&value, place);
        // The rounded value must be representable at the output scale.
        let _ =
            pow10(place.scale).ok_or_else(|| format!("scale {} is out of range", place.scale))?;
        Ok(result)
    }
}

/// Rounds `value` to integer multiples of `place.step * 10^-place.scale`, half-to-even.
fn round_half_even(value: &Decimal, place: Place) -> Rounded {
    // Work at scale = max(value.scale, place.scale); the step is made integral there.
    let scale = value.scale.max(place.scale);
    let lift = 10i128.pow(scale - value.scale);
    let scaled = value.magnitude * lift;
    let step = place.step * 10i128.pow(scale - place.scale);
    debug_assert!(step > 0);

    let q = scaled / step;
    let r = scaled % step; // 0 <= r < step

    let rounded_units = match r.cmp(&(step - r)) {
        std::cmp::Ordering::Less => q,
        std::cmp::Ordering::Greater => q + 1,
        // Exact tie: round to the even neighbour.
        std::cmp::Ordering::Equal => {
            if q % 2 == 0 {
                q
            } else {
                q + 1
            }
        }
    };
    let tie = r == step - r;
    // Reporting scale is the place's own scale, so the retained precision is visible
    // (`60` at one decimal place prints as `60.0`). Multiplying by `place.step` undoes the
    // lift applied above: 602 at scale 1 is 60.2.
    let magnitude = rounded_units * place.step;
    let rounded = Decimal {
        negative: value.negative && magnitude != 0,
        magnitude,
        scale: place.scale,
    };
    // A different reporting scale alone is not a value change; compare both sides at the
    // place's scale (where `magnitude` already lives).
    let original_at_place = value.magnitude.checked_div(lift).unwrap_or(0);
    let changed = value.magnitude % lift != 0 || rounded.magnitude != original_at_place;
    Rounded {
        text: rounded.to_string(),
        changed,
        tie,
        decimal: rounded,
    }
}

/// Wire form of this standard's request: envelope keys plus the typed fields.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Standard id; must be `"GB-T-8170"`.
    pub standard: String,
    /// Edition year; must be `2008`. Accepts a year number or a year string.
    #[serde(deserialize_with = "crate::jsonnum::deserialize_edition")]
    pub edition: u16,
    /// The value to round, as a decimal string.
    pub value: String,
    /// The retained precision, as a decimal string (or a JSON integer number).
    pub digits: DigitsField,
    /// `"decimal"` (default) or `"unit"`.
    #[serde(default)]
    pub mode: Mode,
}

/// `digits` accepts a decimal string (the precise form) or a JSON/TOML integer, which is
/// what a hand-written request naturally looks like for decimal places.
#[derive(Clone, Debug, PartialEq)]
pub struct DigitsField(pub String);

impl<'de> Deserialize<'de> for DigitsField {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::{self, Visitor};
        struct V;
        impl Visitor<'_> for V {
            type Value = DigitsField;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a decimal string such as \"1\" or \"0.5\", or an integer number")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<DigitsField, E> {
                self.visit_string(v.to_string())
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<DigitsField, E> {
                // Reject early what can never be a place; the mode check happens later.
                parse_decimal(&v).map_err(E::custom)?;
                Ok(DigitsField(v))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<DigitsField, E> {
                Ok(DigitsField(v.to_string()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<DigitsField, E> {
                Ok(DigitsField(v.to_string()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<DigitsField, E> {
                // A float `digits` would already have lost the trailing decimal: `0.5` is
                // exact in binary, but `0.1`-style units are not, so the string form is the
                // only accepted spelling for non-integers.
                if v.is_finite() && v.fract() == 0.0 && v.abs() < 1e15 {
                    Ok(DigitsField(format!("{v:.0}")))
                } else {
                    Err(E::custom(format!(
                        "digits = {v} must be an integer number or a decimal string such as \
                         \"0.5\"; pass non-integer units as strings so the digits stay exact"
                    )))
                }
            }
            fn visit_bool<E: de::Error>(self, _: bool) -> Result<DigitsField, E> {
                Err(E::custom("digits must be a number, not a boolean"))
            }
        }
        d.deserialize_any(V)
    }
}

impl Request {
    /// The typed input carried by this request.
    pub fn input(&self) -> Input {
        Input {
            value: self.value.clone(),
            digits: self.digits.0.clone(),
            mode: self.mode,
        }
    }
}

/// Stable output document of this standard.
#[derive(Clone, Debug, Serialize)]
pub struct Response {
    /// Standard id as echoed in the request.
    pub standard: String,
    /// Edition as a string.
    pub edition: String,
    /// Rule provenance.
    pub provenance: Provenance,
    /// The rounded value and its flags.
    pub values: Values,
    /// Non-fatal observations.
    pub warnings: Vec<Issue>,
    /// Always empty on success; present so every response has the same shape.
    pub errors: Vec<Issue>,
}

/// Computed quantities of a rounding.
#[derive(Clone, Debug, Serialize)]
pub struct Values {
    /// Canonical decimal text of the result.
    pub rounded: String,
    /// The input string, echoed for correlation.
    pub input: String,
    /// `"decimal"` or `"unit"`.
    pub mode: Mode,
    /// The value the rounding was resolved to (decimal places or unit).
    pub digits: String,
    /// Whether the input was exactly halfway between two candidates.
    pub tie: bool,
    /// Whether rounding changed the value.
    pub changed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(v: &str) -> Input {
        Input {
            value: v.to_string(),
            digits: "1".to_string(),
            mode: Mode::Decimal,
        }
    }

    fn round(v: &str, digits: &str, mode: Mode) -> String {
        Input {
            value: v.to_string(),
            digits: digits.to_string(),
            mode,
        }
        .evaluate()
        .unwrap_or_else(|e| panic!("{v} / {digits}: {e}"))
        .text
    }

    #[test]
    fn half_even_decimal_places() {
        // Ties go to the even neighbour.
        assert_eq!(round("60.25", "1", Mode::Decimal), "60.2");
        assert_eq!(round("60.35", "1", Mode::Decimal), "60.4");
        assert_eq!(round("-60.75", "1", Mode::Decimal), "-60.8");
        assert_eq!(round("-60.65", "1", Mode::Decimal), "-60.6");
        // Non-ties round to the nearer neighbour.
        assert_eq!(round("60.38", "1", Mode::Decimal), "60.4");
        assert_eq!(round("60.31", "1", Mode::Decimal), "60.3");
        assert_eq!(round("-60.38", "1", Mode::Decimal), "-60.4");
        // Integers are untouched, but still printed to the retained precision.
        assert_eq!(round("60", "1", Mode::Decimal), "60.0");
        assert_eq!(round("60", "0", Mode::Decimal), "60");
        // Zero decimal places: .5 ties go to even integers.
        assert_eq!(round("60.5", "0", Mode::Decimal), "60");
        assert_eq!(round("61.5", "0", Mode::Decimal), "62");
        assert_eq!(round("0.5", "0", Mode::Decimal), "0");
        assert_eq!(round("-0.5", "0", Mode::Decimal), "0");
        assert_eq!(round("-2.5", "0", Mode::Decimal), "-2");
        assert_eq!(round("2.5", "0", Mode::Decimal), "2");
    }

    #[test]
    fn unit_of_last_digit() {
        // Multiples of 0.5: 60.25 is an exact tie between 60.0 and 60.5 -> even index wins.
        assert_eq!(round("60.25", "0.5", Mode::Unit), "60.0");
        assert_eq!(round("60.75", "0.5", Mode::Unit), "61.0");
        assert_eq!(round("60.3", "0.5", Mode::Unit), "60.5");
        // 60.2 is not a multiple of 0.5: it is nearer 60.0 than 60.5.
        assert_eq!(round("60.2", "0.5", Mode::Unit), "60.0");
        // Multiples of 0.2: 60.1 is an exact tie between 60.0 (index 300, even) and 60.2.
        assert_eq!(round("60.1", "0.2", Mode::Unit), "60.0");
        assert_eq!(round("60.3", "0.2", Mode::Unit), "60.4");
        assert_eq!(round("60.2", "0.2", Mode::Unit), "60.2");
        assert_eq!(round("60.0", "0.2", Mode::Unit), "60.0");
        assert_eq!(round("-60.1", "0.2", Mode::Unit), "-60.0");
        assert_eq!(round("-60.3", "0.2", Mode::Unit), "-60.4");
        // Every result is an integer multiple of 0.2: the tenths digit is even or the
        // value is integral.
        for i in 0..=20u32 {
            let v = format!("{}.{}", 60 + i / 10, i % 10);
            let got = round(&v, "0.2", Mode::Unit);
            let d = parse_decimal(&got).unwrap();
            // The result keeps the unit's scale (one decimal, or none when the multiple is
            // an integer such as 60), so the lattice check uses the actual scale.
            let tenths = match d.scale() {
                0 => 0,
                1 => d.magnitude() % 10,
                other => panic!("{v} -> {got} has scale {other}"),
            };
            assert!(
                tenths % 2 == 0,
                "{v} -> {got} is not a multiple of 0.2 (tenths digit {tenths})"
            );
        }
    }

    #[test]
    fn tie_and_changed_flags() {
        let r = dec("60.25").evaluate().unwrap();
        assert!(r.tie);
        assert!(r.changed);
        let r = dec("60.38").evaluate().unwrap();
        assert!(!r.tie);
        assert!(r.changed);
        let r = dec("60.2").evaluate().unwrap();
        assert!(!r.tie);
        assert!(!r.changed);
        assert_eq!(r.text, "60.2");
    }

    #[test]
    fn output_keeps_the_requested_precision() {
        // Trailing zeros are meaningful: they show how many digits were retained.
        assert_eq!(round("60.0", "1", Mode::Decimal), "60.0");
        assert_eq!(round("60", "1", Mode::Decimal), "60.0");
        assert_eq!(round("60", "2", Mode::Decimal), "60.00");
        assert_eq!(round("60.123", "3", Mode::Decimal), "60.123");
        assert_eq!(round("1234.5678", "2", Mode::Decimal), "1234.57");
    }

    #[test]
    fn results_do_not_depend_on_binary_float_parsing() {
        // Values that are famously not representable in f64 still round exactly.
        for (v, want) in [
            ("1.005", "1.00"),
            ("2.675", "2.68"),
            ("1.0051", "1.01"),
            ("0.125", "0.12"),
            ("0.375", "0.38"),
            ("0.115", "0.12"),
        ] {
            assert_eq!(round(v, "2", Mode::Decimal), want, "value {v}");
        }
        // The f64 trap this lane avoids: 1.005 and 2.675 both store strictly BELOW their
        // decimal values (`1.005f64` compares against another f64 literal, so the check
        // must widen to a type with exact decimal digits — `format!`).
        assert_eq!(format!("{:.20}", 1.005f64), "1.00499999999999989342");
        assert_eq!(format!("{:.20}", 2.675f64), "2.67499999999999982236");
        // A float half-up routine therefore sees no tie at all for 2.675 and may answer
        // 2.67; the string lane sees the exact decimal tie and the even neighbour wins.
        assert_eq!(round("2.675", "2", Mode::Decimal), "2.68");
        assert_eq!(round("1.005", "2", Mode::Decimal), "1.00");
        // Genuine decimal ties at two places: 1.015 -> 1.02 (2 even), 1.025 -> 1.02.
        assert_eq!(round("1.015", "2", Mode::Decimal), "1.02");
        assert_eq!(round("1.025", "2", Mode::Decimal), "1.02");
    }

    #[test]
    fn malformed_values_are_rejected_with_reasons() {
        for bad in [
            "", " ", "abc", "1e3", "6.025E1", "NaN", "nan", "inf", "-inf", "1.2.3", "60,", "6 0.2",
            " 60.2", "60.2 ", "1_000.5", "--1", "+", "-", ".", "1,000.5",
        ] {
            let err = dec(bad).evaluate().expect_err(bad);
            assert!(!err.is_empty(), "empty message for {bad:?}");
        }
        let err = dec("1e3").evaluate().unwrap_err();
        assert!(err.contains("exponent"), "{err}");
        let err = dec("NaN").evaluate().unwrap_err();
        assert!(err.contains("finite"), "{err}");
        let err = dec(" 60.2").evaluate().unwrap_err();
        assert!(err.contains("whitespace"), "{err}");
    }

    #[test]
    fn malformed_digits_are_rejected_with_reasons() {
        let cases = [
            ("60.25", "-1", Mode::Decimal),
            ("60.25", "1.5", Mode::Decimal),
            ("60.25", "0", Mode::Unit),
            ("60.25", "-0.5", Mode::Unit),
            ("60.25", "0.25", Mode::Unit),
            ("60.25", "0.05", Mode::Unit),
            ("60.25", "x", Mode::Decimal),
        ];
        for (v, d, m) in cases {
            let err = Input {
                value: v.to_string(),
                digits: d.to_string(),
                mode: m,
            }
            .evaluate()
            .expect_err("bad digits");
            assert!(!err.is_empty());
        }
        let err = Input {
            value: "60.25".to_string(),
            digits: "1.5".to_string(),
            mode: Mode::Decimal,
        }
        .evaluate()
        .unwrap_err();
        assert!(err.contains("unit"), "should point at mode \"unit\": {err}");
    }

    #[test]
    fn leading_zeros_and_signs_are_canonicalised() {
        assert_eq!(round("+060.25", "1", Mode::Decimal), "60.2");
        assert_eq!(round("000.5", "0", Mode::Decimal), "0");
        assert_eq!(round("-0.0", "1", Mode::Decimal), "0.0");
        assert_eq!(round("-0.4", "0", Mode::Decimal), "0");
        assert_eq!(round(".5", "0", Mode::Decimal), "0");
        assert_eq!(round("60.", "1", Mode::Decimal), "60.0");
    }

    #[test]
    fn large_values_are_exact() {
        assert_eq!(
            round("123456789012345678901234567890.5", "0", Mode::Decimal),
            "123456789012345678901234567890"
        );
        assert_eq!(
            round("123456789012345678901234567891.5", "0", Mode::Decimal),
            "123456789012345678901234567892"
        );
    }

    #[test]
    fn decimal_display_round_trips() {
        // Parsing normalises trailing zeros; every canonical form round-trips exactly.
        for text in ["0", "60", "-60.8", "0.001", "-0.001", "123456789.5"] {
            let d = parse_decimal(text).unwrap();
            assert_eq!(d.to_string(), text.trim_start_matches('+'));
        }
    }
}
