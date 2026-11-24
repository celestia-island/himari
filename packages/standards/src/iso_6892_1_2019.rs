//! Tensile-test properties as defined by ISO 6892-1:2019.
//!
//! ISO 6892-1 ("Metallic materials — Tensile testing — Part 1: Method of test at room
//! temperature") defines the three quantities this module computes from raw test data:
//!
//! ```text
//! Rm = Fm / S0                         tensile strength
//! A  = (Lu - L0) / L0 * 100            percentage elongation after fracture
//! E  = (F2 - F1) / (S0 * (e2 - e1))    modulus of elasticity (slope of the elastic part)
//! ```
//!
//! `A` is dimensionless in the formula above and is *reported* as a percentage: the
//! standard's symbol denotes a percentage, so [`Output::percent_elongation`] multiplies by
//! 100 while [`Output::elongation`] keeps the dimensionless ratio. Both are published so a
//! caller never has to guess which convention a number is in.
//!
//! # Unit contract
//!
//! | field | unit | meaning |
//! |---|---|---|
//! | `max_force` | N | `Fm`, the highest force during the test |
//! | `cross_section_area` | m² | `S0`, initial cross-sectional area |
//! | `initial_gauge_length` | m | `L0`, original gauge length |
//! | `final_gauge_length` | m | `Lu`, gauge length after fracture |
//! | `force_at_strain_1` / `force_at_strain_2` | N | `F1`, `F2` on the elastic line |
//! | `strain_1` / `strain_2` | 1 | `e1`, `e2`, engineering strain (extension / L0) |
//!
//! # What is checked
//!
//! Every division is guarded: area, gauge length and the strain interval must be positive
//! (an empty or inverted interval would silently flip the modulus sign). `Lu` below `L0`
//! is rejected — a broken specimen cannot be shorter than its original gauge length in
//! this convention, so it points at a unit mix-up or a data-entry error. Values are
//! returned only when finite.

use crate::jsonnum::Param;
use crate::provenance::{Issue, Provenance, StandardId};
use crate::StandardInfo;
use serde::{Deserialize, Serialize};

/// Message used when a request carries no measurement at all.
const NO_MEASUREMENTS: &str = "no measurements supplied: give (maximum_force, cross_section_area) \
     for Rm, (initial_gauge_length, final_gauge_length) for A, and/or the four \
     force/strain points for E";

/// Canonical standard id.
pub const ID: StandardId = StandardId::new("ISO-6892-1");

/// Edition year accepted on the wire.
pub const EDITION: u16 = 2019;

/// Where the formulas come from. See [`Provenance`] for what is *not* claimed.
pub const PROVENANCE: Provenance = Provenance {
    standard: ID,
    edition: "2019",
    formula_source: "ISO 6892-1:2019, clause 3 (definitions of Rm, A and E) and clause 10 \
                     (evaluation)",
    notes: "Evaluates Rm, A and E from supplied raw measurements only. Specimen geometry \
            selection, test speed regimes, extensometer class, yield phenomena (Rp0.2 etc.), \
            and the standard's own rounding/reporting rules are NOT modelled. Ratio and \
            crosshead-speed effects on A are not corrected. This is not a conformance claim.",
};

/// Registry entry.
pub const INFO: StandardInfo = StandardInfo {
    id: ID,
    edition: EDITION,
    provenance: PROVENANCE,
    summary: "Tensile properties Rm, A and E from force, area and extensometer data",
};

/// Typed input of the tensile-test calculator.
///
/// Every field is optional except at least one measurement: a request may ask for any
/// subset of `Rm`, `A` and `E`, and only the quantities whose inputs are all present are
/// computed. Mixing the three groups is allowed; within a group the inputs are all-or-none.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Input {
    /// `Fm` in N, with `S0`, for `Rm`.
    pub maximum_force: Option<f64>,
    /// `S0` in m², shared by `Rm` and `E`.
    pub cross_section_area: Option<f64>,
    /// `L0` in m, for `A` and (implicitly) the strain definition.
    pub initial_gauge_length: Option<f64>,
    /// `Lu` in m, for `A`.
    pub final_gauge_length: Option<f64>,
    /// `F1` in N, lower point of the elastic slope.
    pub force_at_strain_1: Option<f64>,
    /// `F2` in N, upper point of the elastic slope.
    pub force_at_strain_2: Option<f64>,
    /// `e1` (dimensionless engineering strain) at `F1`.
    pub strain_1: Option<f64>,
    /// `e2` (dimensionless engineering strain) at `F2`.
    pub strain_2: Option<f64>,
}

/// Typed result; each quantity is `None` when the request did not supply its inputs.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Output {
    /// Tensile strength `Rm` in Pa (`None` if not requested).
    pub tensile_strength_pa: Option<f64>,
    /// Elongation ratio `(Lu - L0) / L0`, dimensionless (`None` if not requested).
    pub elongation: Option<f64>,
    /// Percentage elongation after fracture, `100 * (Lu - L0) / L0` (`None` if not requested).
    pub percent_elongation: Option<f64>,
    /// Modulus of elasticity `E` in Pa (`None` if not requested).
    pub modulus_pa: Option<f64>,
}

/// Wire form of this standard's request: envelope keys plus the typed fields.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Standard id; must be `"ISO-6892-1"`.
    pub standard: String,
    /// Edition year; must be `2019`. Accepts a year number or a year string.
    #[serde(deserialize_with = "crate::jsonnum::deserialize_edition")]
    pub edition: u16,
    /// `Fm` in N.
    #[serde(default)]
    pub maximum_force: Option<Param>,
    /// `S0` in m².
    #[serde(default)]
    pub cross_section_area: Option<Param>,
    /// `L0` in m.
    #[serde(default)]
    pub initial_gauge_length: Option<Param>,
    /// `Lu` in m.
    #[serde(default)]
    pub final_gauge_length: Option<Param>,
    /// `F1` in N.
    #[serde(default)]
    pub force_at_strain_1: Option<Param>,
    /// `F2` in N.
    #[serde(default)]
    pub force_at_strain_2: Option<Param>,
    /// `e1`, dimensionless.
    #[serde(default)]
    pub strain_1: Option<Param>,
    /// `e2`, dimensionless.
    #[serde(default)]
    pub strain_2: Option<Param>,
}

impl Request {
    /// The typed input carried by this request.
    pub fn input(&self) -> Input {
        Input {
            maximum_force: self.maximum_force.map(Param::get),
            cross_section_area: self.cross_section_area.map(Param::get),
            initial_gauge_length: self.initial_gauge_length.map(Param::get),
            final_gauge_length: self.final_gauge_length.map(Param::get),
            force_at_strain_1: self.force_at_strain_1.map(Param::get),
            force_at_strain_2: self.force_at_strain_2.map(Param::get),
            strain_1: self.strain_1.map(Param::get),
            strain_2: self.strain_2.map(Param::get),
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
    /// Computed quantities; absent ones are `null`.
    pub values: Values,
    /// Non-fatal observations.
    pub warnings: Vec<Issue>,
    /// Always empty on success; present so every response has the same shape.
    pub errors: Vec<Issue>,
}

/// Computed quantities. Every key is always present; `null` means "not requested".
#[derive(Clone, Debug, Serialize)]
pub struct Values {
    /// `Rm` in Pa.
    pub tensile_strength_pa: Option<f64>,
    /// `(Lu - L0) / L0`, dimensionless.
    pub elongation: Option<f64>,
    /// `100 * (Lu - L0) / L0`.
    pub percent_elongation: Option<f64>,
    /// `E` in Pa.
    pub modulus_pa: Option<f64>,
}

/// Which measurement groups a request asked for.
struct Wanted {
    strength: bool,
    elongation: bool,
    modulus: bool,
}

fn all_or_none(fields: &[(&'static str, Option<f64>)]) -> Result<Option<()>, String> {
    let present = fields.iter().filter(|(_, v)| v.is_some()).count();
    match present {
        0 => Ok(None),
        n if n == fields.len() => Ok(Some(())),
        _ => {
            let missing: Vec<&str> = fields
                .iter()
                .filter(|(_, v)| v.is_none())
                .map(|(k, _)| *k)
                .collect();
            let required: Vec<&str> = fields.iter().map(|(k, _)| *k).collect();
            Err(format!(
                "incomplete group {}: needs all of {:?}, but {:?} is missing",
                required[0], required, missing
            ))
        }
    }
}

fn positive(name: &str, v: Option<f64>, unit: &str) -> Result<Option<f64>, String> {
    match v {
        None => Ok(None),
        Some(x) if !x.is_finite() => Err(format!("{name} must be finite, got {x}")),
        Some(x) if x <= 0.0 => Err(format!("{name} must be positive, got {x} {unit}")),
        Some(x) => Ok(Some(x)),
    }
}

impl Input {
    /// Confirms that at least one quantity was requested and that every requested group
    /// has all of its inputs (all-or-none), without looking at the values themselves.
    ///
    /// Kept separate from the value checks: the dispatcher reports a *schema* problem
    /// (`unsupported-input` / missing field) before it reports a numeric domain problem.
    pub fn check_complete(&self) -> Result<(), String> {
        let strength = self.maximum_force.is_some();
        let elongation = self.initial_gauge_length.is_some() || self.final_gauge_length.is_some();
        let modulus = self.force_at_strain_1.is_some()
            || self.force_at_strain_2.is_some()
            || self.strain_1.is_some()
            || self.strain_2.is_some();

        if !(strength || elongation || modulus) {
            return Err(NO_MEASUREMENTS.to_string());
        }

        if strength {
            all_or_none(&[
                ("maximum_force", self.maximum_force),
                ("cross_section_area", self.cross_section_area),
            ])?;
        }
        if elongation {
            all_or_none(&[
                ("initial_gauge_length", self.initial_gauge_length),
                ("final_gauge_length", self.final_gauge_length),
            ])?;
        }
        if modulus {
            all_or_none(&[
                ("force_at_strain_1", self.force_at_strain_1),
                ("force_at_strain_2", self.force_at_strain_2),
                ("strain_1", self.strain_1),
                ("strain_2", self.strain_2),
            ])?;
        }

        if modulus && self.cross_section_area.is_none() {
            return Err(
                "E needs cross_section_area: the elastic slope is d(stress)/d(strain), \
                 so the force difference must be divided by S0"
                    .to_string(),
            );
        }
        Ok(())
    }

    /// Validates that the values of the requested groups are well-defined.
    pub fn validate(&self) -> Result<(), String> {
        self.check_complete()?;
        let modulus = self.force_at_strain_1.is_some();

        positive("cross_section_area", self.cross_section_area, "m^2")?;
        positive("initial_gauge_length", self.initial_gauge_length, "m")?;
        positive("final_gauge_length", self.final_gauge_length, "m")?;
        positive("maximum_force", self.maximum_force, "N")?;

        if let (Some(l0), Some(lu)) = (self.initial_gauge_length, self.final_gauge_length) {
            if lu < l0 {
                return Err(format!(
                    "final_gauge_length ({lu} m) is smaller than initial_gauge_length \
                     ({l0} m): A would be negative, check the units and the measurement"
                ));
            }
        }

        if modulus {
            let f1 = self.force_at_strain_1.unwrap_or_default();
            let f2 = self.force_at_strain_2.unwrap_or_default();
            let e1 = self.strain_1.unwrap_or_default();
            let e2 = self.strain_2.unwrap_or_default();
            for (name, v) in [("force_at_strain_1", f1), ("force_at_strain_2", f2)] {
                if !v.is_finite() {
                    return Err(format!("{name} must be finite, got {v}"));
                }
            }
            for (name, v) in [("strain_1", e1), ("strain_2", e2)] {
                if !v.is_finite() {
                    return Err(format!("{name} must be finite, got {v}"));
                }
            }
            // Written as an explicit test so a NaN strain (filtered above) cannot slip
            // through a negated comparison.
            if e2 <= e1 {
                return Err(format!(
                    "strain_2 ({e2}) must be greater than strain_1 ({e1}): the elastic \
                     slope needs a positive strain interval"
                ));
            }
        }
        Ok(())
    }

    /// Evaluates every requested quantity. Returns the values and any warnings.
    pub fn evaluate(&self) -> Result<(Output, Vec<Issue>), String> {
        self.validate()?;
        let wanted = Wanted {
            strength: self.maximum_force.is_some(),
            elongation: self.initial_gauge_length.is_some(),
            modulus: self.force_at_strain_1.is_some(),
        };
        let mut out = Output::default();
        let mut warnings = Vec::new();

        if wanted.strength {
            let (fm, s0) = (
                self.maximum_force.unwrap(),
                self.cross_section_area.unwrap(),
            );
            out.tensile_strength_pa = Some(tensile_strength(fm, s0));
        }
        if wanted.elongation {
            let (l0, lu) = (
                self.initial_gauge_length.unwrap(),
                self.final_gauge_length.unwrap(),
            );
            let ratio = elongation(lu, l0);
            out.elongation = Some(ratio);
            out.percent_elongation = Some(ratio * 100.0);
            if ratio == 0.0 {
                warnings.push(Issue::warning(
                    "no-plastic-extension",
                    "final_gauge_length",
                    "final and initial gauge lengths are equal, so A = 0; \
                     check that the measurement is not a transcription error",
                ));
            }
        }
        if wanted.modulus {
            // check_complete() guarantees S0 is present here.
            let s0 = self.cross_section_area.unwrap_or_default();
            out.modulus_pa = Some(elastic_slope(
                self.force_at_strain_1.unwrap_or_default(),
                self.force_at_strain_2.unwrap_or_default(),
                self.strain_1.unwrap_or_default(),
                self.strain_2.unwrap_or_default(),
                s0,
            ));
        }

        for (name, value) in [
            ("tensile_strength_pa", out.tensile_strength_pa),
            ("elongation", out.elongation),
            ("modulus_pa", out.modulus_pa),
        ] {
            if let Some(v) = value {
                if !v.is_finite() {
                    return Err(format!("{name} overflowed to {v}; inputs are out of range"));
                }
            }
        }
        Ok((out, warnings))
    }
}

/// `Rm = Fm / S0`, in Pa.
fn tensile_strength(fm_n: f64, s0_m2: f64) -> f64 {
    himari_macros::formula_matlab!(
        r#"decl { fm_n : force [N]; s0_m2 : area [m^2]; }
fm_n / s0_m2"#
    )
}

/// `A = (Lu - L0) / L0`, dimensionless.
fn elongation(lu_m: f64, l0_m: f64) -> f64 {
    himari_macros::formula_matlab!(
        r#"decl { lu_m : length [m]; l0_m : length [m]; }
(lu_m - l0_m) / l0_m"#
    )
}

/// `E = (F2 - F1) / (S0 * (e2 - e1))`, in Pa.
/// `E = (F2 - F1) / (S0 * (e2 - e1))`, in Pa.
///
/// The parameter order is the caller's (forces first, then the strain interval, then the
/// area); the macro symbols are named after the parameters they bind.
fn elastic_slope(f1_n: f64, f2_n: f64, strain_1: f64, strain_2: f64, area_m2: f64) -> f64 {
    let (e1, e2) = (strain_1, strain_2);
    let s0_m2 = area_m2;
    himari_macros::formula_matlab!(
        r#"decl { f1_n : force [N]; f2_n : force [N]; s0_m2 : area [m^2]; e1 : dimensionless [1]; e2 : dimensionless [1]; }
(f2_n - f1_n) / s0_m2 / (e2 - e1)"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point_stress() -> Input {
        Input {
            maximum_force: Some(12566.0),
            cross_section_area: Some(40.0e-6),
            ..Input::default()
        }
    }

    #[test]
    fn tensile_strength_matches_reference() {
        let (out, warnings) = point_stress().evaluate().unwrap();
        assert_eq!(warnings, vec![]);
        let rm = out.tensile_strength_pa.unwrap();
        assert!((rm - 314.15e6).abs() < 1e-3, "Rm = {rm} Pa");
        assert!((rm / 1e6 - 314.15).abs() < 1e-9);
        assert_eq!(out.elongation, None);
        assert_eq!(out.modulus_pa, None);
    }

    #[test]
    fn elongation_matches_reference() {
        let inp = Input {
            initial_gauge_length: Some(0.05),
            final_gauge_length: Some(0.065),
            ..Input::default()
        };
        let (out, warnings) = inp.evaluate().unwrap();
        assert_eq!(warnings, vec![]);
        assert_eq!(out.elongation, Some(0.3));
        assert_eq!(out.percent_elongation, Some(30.0));
    }

    #[test]
    fn modulus_matches_reference() {
        // A linear elastic line: 20 kN over 20 MPa stress for 0.002 strain gives 10 GPa.
        let s0 = 1.0e-3;
        let inp = Input {
            cross_section_area: Some(s0),
            force_at_strain_1: Some(1000.0),
            force_at_strain_2: Some(21000.0),
            strain_1: Some(0.0),
            strain_2: Some(0.002),
            ..Input::default()
        };
        let (out, _) = inp.evaluate().unwrap();
        let e = out.modulus_pa.unwrap();
        assert_eq!(e, 1.0e10, "E = {e} Pa");
        assert_eq!(out.tensile_strength_pa, None);
    }

    #[test]
    fn modulus_uses_the_force_difference_not_the_origin() {
        // F1 is not zero and e1 is not zero: only the interval matters.
        let inp = Input {
            cross_section_area: Some(2.0e-6),
            force_at_strain_1: Some(201.0),
            force_at_strain_2: Some(401.0),
            strain_1: Some(0.000981),
            strain_2: Some(0.001981),
            ..Input::default()
        };
        let (out, _) = inp.evaluate().unwrap();
        let want = (401.0 - 201.0) / (2.0e-6 * (0.001981 - 0.000981));
        let got = out.modulus_pa.unwrap();
        assert!((got - want).abs() <= 1e-9 * want, "{got} vs {want}");
    }

    #[test]
    fn all_three_groups_can_be_requested_together() {
        let inp = Input {
            maximum_force: Some(12566.0),
            cross_section_area: Some(40.0e-6),
            initial_gauge_length: Some(0.05),
            final_gauge_length: Some(0.065),
            force_at_strain_1: Some(100.0),
            force_at_strain_2: Some(900.0),
            strain_1: Some(0.0001),
            strain_2: Some(0.0009),
        };
        let (out, warnings) = inp.evaluate().unwrap();
        assert_eq!(warnings, vec![]);
        assert_eq!(out.elongation, Some(0.3));
        assert_eq!(
            out.tensile_strength_pa.unwrap(),
            12566.0 / 40.0e-6,
            "Rm is exact for these binary-exact inputs"
        );
        assert!(out.modulus_pa.unwrap() > 0.0);
    }

    #[test]
    fn incomplete_groups_are_rejected_with_the_missing_name() {
        let cases = [
            Input {
                maximum_force: Some(1.0),
                ..Input::default()
            },
            Input {
                initial_gauge_length: Some(0.05),
                ..Input::default()
            },
            Input {
                force_at_strain_1: Some(1.0),
                force_at_strain_2: Some(2.0),
                strain_1: Some(0.0),
                ..Input::default()
            },
        ];
        for inp in cases {
            let err = inp.evaluate().expect_err("incomplete group");
            assert!(err.contains("missing"), "unhelpful: {err}");
        }
    }

    #[test]
    fn empty_request_is_rejected() {
        let err = Input::default().evaluate().expect_err("nothing requested");
        assert!(err.contains("no measurements"), "{err}");
    }

    #[test]
    fn modulus_without_area_is_rejected() {
        let inp = Input {
            force_at_strain_1: Some(1.0),
            force_at_strain_2: Some(2.0),
            strain_1: Some(0.0),
            strain_2: Some(0.001),
            ..Input::default()
        };
        let err = inp.evaluate().expect_err("needs S0");
        assert!(err.contains("cross_section_area"), "{err}");
    }

    #[test]
    fn domain_errors_are_reported_not_panicked() {
        for bad in [
            Input {
                cross_section_area: Some(0.0),
                ..point_stress()
            },
            Input {
                cross_section_area: Some(-1.0),
                ..point_stress()
            },
            Input {
                maximum_force: Some(f64::NAN),
                cross_section_area: Some(1.0),
                ..Input::default()
            },
            Input {
                initial_gauge_length: Some(0.05),
                final_gauge_length: Some(0.04),
                ..Input::default()
            },
            Input {
                initial_gauge_length: Some(0.0),
                final_gauge_length: Some(0.04),
                ..Input::default()
            },
            Input {
                cross_section_area: Some(1.0),
                force_at_strain_1: Some(0.0),
                force_at_strain_2: Some(1.0),
                strain_1: Some(0.002),
                strain_2: Some(0.002),
                ..Input::default()
            },
            Input {
                cross_section_area: Some(1.0),
                force_at_strain_1: Some(0.0),
                force_at_strain_2: Some(1.0),
                strain_1: Some(0.003),
                strain_2: Some(0.002),
                ..Input::default()
            },
            Input {
                cross_section_area: Some(1.0),
                force_at_strain_1: Some(0.0),
                force_at_strain_2: Some(1.0),
                strain_1: Some(0.0),
                strain_2: Some(f64::INFINITY),
                ..Input::default()
            },
        ] {
            assert!(bad.evaluate().is_err(), "should be rejected: {bad:?}");
        }
    }

    #[test]
    fn equal_lengths_warn() {
        let inp = Input {
            initial_gauge_length: Some(0.05),
            final_gauge_length: Some(0.05),
            ..Input::default()
        };
        let (out, warnings) = inp.evaluate().unwrap();
        assert_eq!(out.elongation, Some(0.0));
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "no-plastic-extension");
    }
}
