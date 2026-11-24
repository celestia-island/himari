//! Brinell hardness (HBW) as defined by ISO 6506-1:2014.
//!
//! ISO 6506-1 ("Metallic materials — Brinell hardness test — Part 1: Test method") states
//! the hardness number as the test force divided by the *curved* surface area of the
//! spherical indentation, scaled to the standard's hardness unit.
//!
//! # Unit contract
//!
//! | field | unit | meaning |
//! |---|---|---|
//! | `force` | N | applied test force `F` |
//! | `ball_diameter` | m | ball diameter `D` |
//! | `indentation_diameter` | m | mean indentation diameter `d` |
//!
//! The module works in SI throughout and reports both the mean indentation pressure in
//! pascals and the hardness number:
//!
//! ```text
//! A   = pi * D * (D - sqrt(D^2 - d^2))     [m²]  curved indentation area
//! p   = 2 * F / A                          [Pa]  mean indentation pressure
//! HBW = 0.102 * p / 10^6                   [N/mm²]
//! ```
//!
//! The `0.102` factor is the literal printed in the standard. It is kept as that literal,
//! not recomputed as `1 / 9.80665`: the two differ in the last bits, and the standard's own
//! constant is the reference value (`HBW_SCALE`).
//!
//! # Range
//!
//! The standard's own validity range for the ratio `d / D` is 0.24 … 0.60. Inputs outside
//! it still yield a number, together with a warning (`indentation-ratio-outside-range`):
//! an out-of-range indentation diameter is a setup/measurement situation to be reported,
//! not a reason to refuse the arithmetic. Everything else that leaves the formula
//! undefined (non-positive force or diameters, `d >= D`) is an error.

use crate::jsonnum::Param;
use crate::provenance::{Issue, Provenance, StandardId};
use crate::StandardInfo;
use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::provenance::Severity;

/// Canonical standard id.
pub const ID: StandardId = StandardId::new("ISO-6506-1");

/// Edition year accepted on the wire.
pub const EDITION: u16 = 2014;

/// Lower end of the standard's validity range for `d / D`.
pub const RATIO_MIN: f64 = 0.24;

/// Upper end of the standard's validity range for `d / D`.
pub const RATIO_MAX: f64 = 0.60;

/// Factor from mean indentation pressure to the Brinell hardness number.
///
/// This is the literal printed in the standard (`0.102`), applied as such. It is not
/// recomputed as `1 / 9.80665`.
pub const HBW_SCALE: f64 = 0.102;

/// Where the formula comes from. See [`Provenance`] for what is *not* claimed.
pub const PROVENANCE: Provenance = Provenance {
    standard: ID,
    edition: "2014",
    formula_source: "ISO 6506-1:2014, clause 4 (Brinell hardness number and indentation \
                     surface area)",
    notes: "Evaluates the clause-level hardness relation from F [N], D [m] and d [m] only. \
            Hardness numbers are expressed as HBW with the standard's 0.102 N/mm^2 unit. \
            Ball type, force-level selection, dwell time, surface preparation, minimum \
            thickness and measurement uncertainty are NOT modelled or enforced. Not a \
            conformance claim.",
};

/// Registry entry.
pub const INFO: StandardInfo = StandardInfo {
    id: ID,
    edition: EDITION,
    provenance: PROVENANCE,
    summary: "Brinell hardness HBW from force and indentation geometry",
};

/// Typed input of the Brinell calculator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Input {
    /// Test force `F` in newtons.
    pub force: f64,
    /// Ball diameter `D` in metres.
    pub ball_diameter: f64,
    /// Mean indentation diameter `d` in metres.
    pub indentation_diameter: f64,
}

/// Typed result of the Brinell calculator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Output {
    /// Indentation diameter over ball diameter `d / D` (dimensionless).
    pub ratio: f64,
    /// Mean indentation pressure `2 F / A_indent` in pascals.
    pub pressure_pa: f64,
    /// Brinell hardness number, `0.102 * pressure_pa / 10^6` in N/mm².
    pub hbw: f64,
}

/// Wire form of this standard's request: envelope keys plus the typed fields.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Standard id; must be `"ISO-6506-1"`.
    pub standard: String,
    /// Edition year; must be `2014`. Accepts a year number or a year string.
    #[serde(deserialize_with = "crate::jsonnum::deserialize_edition")]
    pub edition: u16,
    /// Test force in N.
    pub force: Param,
    /// Ball diameter in m.
    pub ball_diameter: Param,
    /// Indentation diameter in m.
    pub indentation_diameter: Param,
}

impl Request {
    /// The typed input carried by this request.
    pub fn input(&self) -> Input {
        Input {
            force: self.force.get(),
            ball_diameter: self.ball_diameter.get(),
            indentation_diameter: self.indentation_diameter.get(),
        }
    }
}

/// Stable output document of this standard.
#[derive(Clone, Debug, Serialize)]
pub struct Response {
    /// Standard id as echoed in the request.
    pub standard: String,
    /// Edition as a string, so it cannot be mistaken for a computed value.
    pub edition: String,
    /// Formula provenance.
    pub provenance: Provenance,
    /// Computed quantities.
    pub values: Values,
    /// Non-fatal observations.
    pub warnings: Vec<Issue>,
    /// Always empty on success; present so every response has the same shape.
    pub errors: Vec<Issue>,
}

/// Computed quantities, with the units they carry.
#[derive(Clone, Debug, Serialize)]
pub struct Values {
    /// `d / D` (dimensionless).
    pub ratio: f64,
    /// `2 F / A_indent` in Pa.
    pub pressure_pa: f64,
    /// Brinell hardness number HBW (N/mm²).
    pub hbw: f64,
}

/// Curved indentation area, in m².
///
/// The area is `pi * D * (D - sqrt(D^2 - d^2))`. Evaluated here as
/// `pi * D^2 * r^2 / (1 + sqrt(1 - r^2))` with `r = d / D`: mathematically identical
/// (`1 - sqrt(1 - r^2) = r^2 / (1 + sqrt(1 - r^2))`) and free of the cancellation that
/// `1 - sqrt(1 - r^2)` loses for the deep indentations the standard covers.
fn indentation_area(big_d_m: f64, r: f64) -> f64 {
    let pi_const = std::f64::consts::PI;
    himari_macros::formula_matlab!(
        r#"decl { pi_const : dimensionless [1]; big_d_m : length [m]; r : dimensionless [1]; }
pi_const * big_d_m ^ 2 * r ^ 2 / (1 + sqrt((1 - r) * (1 + r)))"#
    )
}

impl Input {
    /// Checks that the inputs make the formula well-defined.
    pub fn validate(&self) -> Result<(), String> {
        let values = [
            ("force", self.force, "N"),
            ("ball_diameter", self.ball_diameter, "m"),
            ("indentation_diameter", self.indentation_diameter, "m"),
        ];
        for (name, v, unit) in values {
            if !v.is_finite() {
                return Err(format!("{name} must be a finite number, got {v}"));
            }
            if v <= 0.0 {
                return Err(format!("{name} must be positive, got {v} {unit}"));
            }
        }
        if self.indentation_diameter >= self.ball_diameter {
            return Err(format!(
                "indentation_diameter ({}) must be smaller than ball_diameter ({}); \
                 the indentation cannot reach the ball diameter or beyond",
                self.indentation_diameter, self.ball_diameter
            ));
        }
        Ok(())
    }

    /// Evaluates the hardness. Returns the value and any warnings.
    pub fn evaluate(&self) -> Result<(Output, Vec<Issue>), String> {
        self.validate()?;
        let (force, big_d, indentation) =
            (self.force, self.ball_diameter, self.indentation_diameter);
        let relative = indentation / big_d;
        let area = indentation_area(big_d, relative);
        // Mean indentation pressure over the curved surface, Pa.
        let pressure_pa = 2.0 * force / area;
        // The standard's hardness scale: 1 HBW = 0.102 N/mm^2 of mean pressure.
        let hbw = HBW_SCALE * pressure_pa / 1.0e6;
        debug_assert!(pressure_pa.is_finite() && hbw.is_finite());

        let mut warnings = Vec::new();
        // The endpoint comparison uses a tolerance so that an input which *is* the bound
        // in decimal (0.24 as typed) is not flagged by binary rounding noise.
        let eps = 1e-12;
        if relative < RATIO_MIN - eps || relative > RATIO_MAX + eps {
            warnings.push(Issue::warning(
                "indentation-ratio-outside-range",
                "indentation_diameter",
                format!(
                    "d/D = {relative:.6} is outside the standard's valid range \
                     {RATIO_MIN:.2}..={RATIO_MAX:.2}; the value is computed but should not be \
                     reported as a standardized hardness"
                ),
            ));
        }
        Ok((
            Output {
                ratio: relative,
                pressure_pa,
                hbw,
            },
            warnings,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 10 mm ball / 29 420 N / d = 4 mm case.
    fn canonical(indentation_diameter: f64) -> Input {
        Input {
            force: 29420.0,
            ball_diameter: 0.01,
            indentation_diameter,
        }
    }

    #[test]
    fn worked_example_matches_reference_values() {
        let (out, warnings) = canonical(0.004).evaluate().unwrap();
        assert_eq!(warnings, vec![]);
        // 228.83119821082627 HBW is the reference value for this case.
        assert!(
            (out.hbw - 228.831_198_210_826_27).abs() < 1e-9,
            "HBW = {}",
            out.hbw
        );
        assert!((out.ratio - 0.4).abs() < 1e-15);
        // The same case in pascals: 1 HBW corresponds to 10^6 / 0.102 Pa.
        assert!(
            (out.pressure_pa - out.hbw * 1.0e6 / HBW_SCALE).abs() <= 1e-6 * out.pressure_pa,
            "pressure = {} Pa",
            out.pressure_pa
        );
    }

    #[test]
    fn second_worked_example_matches_reference_value() {
        let (out, warnings) = canonical(0.0035).evaluate().unwrap();
        assert_eq!(warnings, vec![]);
        assert!(
            (out.hbw - 302.037_155_626_990_17).abs() < 1e-9,
            "HBW = {}",
            out.hbw
        );
        assert!((out.ratio - 0.35).abs() < 1e-15);
    }

    #[test]
    fn matches_the_literal_standard_form_within_float_noise() {
        // Independent arithmetic straight from the standard's text, in the same unit the
        // module reports (HBW as a hardness number, i.e. N/mm^2 of the 0.102-scaled
        // pressure): HBW = 0.102 * 2F / A / 10^6 with A = pi*D*(D - sqrt(D^2 - d^2)).
        for d in [0.0024, 0.003, 0.0035, 0.004, 0.005, 0.006] {
            let inp = canonical(d);
            let (out, _) = inp.evaluate().unwrap();
            let (f, big_d) = (inp.force, inp.ball_diameter);
            let literal = HBW_SCALE * 2.0 * f
                / (std::f64::consts::PI * big_d * (big_d - (big_d * big_d - d * d).sqrt()))
                / 1.0e6;
            assert!(
                (out.hbw - literal).abs() <= 1e-9 * literal.abs(),
                "d={d}: {} vs {literal}",
                out.hbw
            );
        }
    }

    #[test]
    fn area_identity_holds_at_every_ratio() {
        // The cancellation-free restatement agrees with the literal area expression.
        for r in [0.05, 0.1, 0.24, 0.4, 0.6, 0.9] {
            let big_d = 0.01;
            let d = r * big_d;
            let literal = std::f64::consts::PI * big_d * (big_d - (big_d * big_d - d * d).sqrt());
            let got = indentation_area(big_d, r);
            assert!(
                (got - literal).abs() <= 1e-13 * literal,
                "r={r}: {got} vs {literal}"
            );
        }
    }

    #[test]
    fn scale_factor_is_the_printed_literal() {
        // 0.102 is not 1/9.80665; the module documents the literal as the reference.
        assert_eq!(HBW_SCALE.to_bits(), 0.102f64.to_bits());
        assert_ne!(HBW_SCALE.to_bits(), (1.0f64 / 9.80665f64).to_bits());
    }

    #[test]
    fn ratio_bounds_warn_but_still_compute() {
        let (out, warnings) = canonical(0.0023).evaluate().unwrap();
        assert!(out.hbw > 0.0);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "indentation-ratio-outside-range");
        assert_eq!(warnings[0].field, Some("indentation_diameter"));
        assert_eq!(warnings[0].severity, Severity::Warning);

        let (_, warnings) = canonical(0.0061).evaluate().unwrap();
        assert_eq!(warnings.len(), 1);

        // Exactly on the boundary is inside the range.
        assert_eq!(canonical(0.0024).evaluate().unwrap().1, vec![]);
        assert_eq!(canonical(0.0060).evaluate().unwrap().1, vec![]);
    }

    #[test]
    fn domain_errors_are_reported_not_panicked() {
        for bad in [
            Input {
                force: 0.0,
                ..canonical(0.004)
            },
            Input {
                force: -29420.0,
                ..canonical(0.004)
            },
            Input {
                ball_diameter: 0.0,
                ..canonical(0.004)
            },
            Input {
                ball_diameter: -0.01,
                ..canonical(0.004)
            },
            Input {
                indentation_diameter: 0.0,
                ..canonical(0.004)
            },
            Input {
                indentation_diameter: 0.01,
                ..canonical(0.004)
            },
            Input {
                indentation_diameter: 0.02,
                ..canonical(0.004)
            },
            Input {
                force: f64::NAN,
                ..canonical(0.004)
            },
            Input {
                force: f64::INFINITY,
                ..canonical(0.004)
            },
            Input {
                indentation_diameter: f64::NAN,
                ..canonical(0.004)
            },
        ] {
            let err = bad.evaluate().expect_err("must be rejected");
            assert!(!err.is_empty());
        }
    }

    #[test]
    fn extreme_but_valid_inputs_stay_finite() {
        let inp = Input {
            force: 1e7,
            ball_diameter: 1e-3,
            indentation_diameter: 5.9e-4,
        };
        let (out, _) = inp.evaluate().unwrap();
        assert!(out.hbw.is_finite() && out.hbw > 0.0);
    }
}
