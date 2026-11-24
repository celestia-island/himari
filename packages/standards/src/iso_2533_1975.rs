//! Standard-atmosphere troposphere, as specified in ISO 2533:1975.
//!
//! **Edition.** The relations below are the ones published in the **1975** edition of
//! ISO 2533, which is the only edition this crate was reviewed against. A replacement
//! edition reached the ISO/FDIS stage; its text was not consulted and no claim is made
//! about it, about the 1975 edition remaining current, or about newer editions. Check the
//! edition your work must follow before relying on a value.
//!
//! # Scope
//!
//! The troposphere only: geopotential altitudes from 0 m to 11 000 m, both ends inclusive.
//! Above 11 km the lapse rate changes (isothermal layer) and this module deliberately
//! refuses instead of extrapolating the tropospheric gradient.
//!
//! # Relations
//!
//! ```text
//! z   = R0 * H / (R0 + H)              geopotential height from geometric H
//! T   = T0 + beta * z                  T0 = 288.15 K, beta = -0.0065 K/m
//! p   = p0 * (T / T0) ^ (g0 / (R * |beta|))
//! rho = p / (R * T)                    ideal gas, dry air
//! ```
//!
//! with `p0 = 101 325 Pa`, `g0 = 9.80665 m/s²`, `R0 = 6 356 766 m` and
//! `R = 287.05307204706463 J/(kg·K)` — the dry-air gas constant implied by the standard's
//! molar mass. Humidity is not modelled.

use crate::jsonnum::Param;
use crate::provenance::{Issue, Provenance, StandardId};
use crate::StandardInfo;
use serde::{Deserialize, Serialize};

/// Canonical standard id.
pub const ID: StandardId = StandardId::new("ISO-2533");

/// Edition year accepted on the wire.
pub const EDITION: u16 = 1975;

/// Sea-level temperature `T0` (K).
pub const T0_K: f64 = 288.15;

/// Sea-level pressure `p0` (Pa).
pub const P0_PA: f64 = 101_325.0;

/// Tropospheric lapse rate `beta` (K/m); negative, temperature falls with height.
pub const LAPSE_RATE_K_PER_M: f64 = -0.0065;

/// Standard acceleration of free fall (m/s²).
pub const G0_M_PER_S2: f64 = 9.80665;

/// Earth radius used for the geopotential conversion (m).
pub const EARTH_RADIUS_M: f64 = 6_356_766.0;

/// Specific gas constant of dry air (J/(kg·K)).
pub const R_AIR_J_PER_KG_K: f64 = 287.05307204706463;

/// Lower bound of the modelled layer, as geopotential altitude (m).
pub const Z_MIN_M: f64 = 0.0;

/// Upper bound of the modelled layer, as geopotential altitude (m) — the tropopause.
pub const Z_MAX_M: f64 = 11_000.0;

/// Where the relations come from. See [`Provenance`] for what is *not* claimed.
pub const PROVENANCE: Provenance = Provenance {
    standard: ID,
    edition: "1975",
    formula_source: "ISO 2533:1975, clause 3 (geopotential height) and clause 4 \
                     (tropospheric temperature and pressure of the standard atmosphere)",
    notes: "Reviewed against the 1975 edition ONLY. A replacement edition was in ISO/FDIS \
            stage and was not consulted; no claim is made about it or any other edition. \
            Covers geopotential altitudes 0 m..=11000 m only (troposphere, dry air, no \
            humidity). Values are the evaluation of the clauses as written and were NOT \
            checked digit-by-digit against the edition's printed tables. Not a conformance \
            claim.",
};

/// Registry entry.
pub const INFO: StandardInfo = StandardInfo {
    id: ID,
    edition: EDITION,
    provenance: PROVENANCE,
    summary: "Troposphere temperature, pressure, density and geopotential altitude",
};

/// Typed input: one geometric altitude.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Input {
    /// Geometric altitude `H` in metres above mean sea level.
    pub geometric_altitude_m: f64,
}

/// Typed result.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Output {
    /// Geometric altitude as requested (m).
    pub geometric_altitude_m: f64,
    /// Geopotential altitude `z` (m).
    pub geopotential_altitude_m: f64,
    /// Temperature (K).
    pub temperature_k: f64,
    /// Pressure (Pa).
    pub pressure_pa: f64,
    /// Density (kg/m³).
    pub density_kg_m3: f64,
}

/// Wire form of this standard's request: envelope keys plus the typed field.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Standard id; must be `"ISO-2533"`.
    pub standard: String,
    /// Edition year; must be `1975`. Accepts a year number or a year string.
    #[serde(deserialize_with = "crate::jsonnum::deserialize_edition")]
    pub edition: u16,
    /// Geometric altitude `H` in m.
    pub geometric_altitude_m: Param,
}

impl Request {
    /// The typed input carried by this request.
    pub fn input(&self) -> Input {
        Input {
            geometric_altitude_m: self.geometric_altitude_m.get(),
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
    /// Geometric altitude as requested (m).
    pub geometric_altitude_m: f64,
    /// Geopotential altitude (m).
    pub geopotential_altitude_m: f64,
    /// Temperature (K).
    pub temperature_k: f64,
    /// Pressure (Pa).
    pub pressure_pa: f64,
    /// Density (kg/m³).
    pub density_kg_m3: f64,
}

impl Input {
    /// Rejects inputs outside the modelled layer or not finite.
    ///
    /// The layer is defined on geopotential altitude, and the two heights differ by about
    /// 19 m at 11 km (`z = R0*H/(R0+H)`), so the bound is applied to the geopotential
    /// altitude the relations actually use.
    pub fn validate(&self) -> Result<(), String> {
        let h = self.geometric_altitude_m;
        if !h.is_finite() {
            return Err(format!("geometric_altitude_m must be finite, got {h}"));
        }
        let z = geopotential(h);
        if !(Z_MIN_M..=Z_MAX_M).contains(&z) {
            return Err(format!(
                "geometric_altitude_m = {h} m gives geopotential altitude {z:.3} m, outside \
                 the troposphere modelled here ({Z_MIN_M}..={Z_MAX_M} m)"
            ));
        }
        Ok(())
    }

    /// Evaluates the atmosphere at the requested altitude.
    pub fn evaluate(&self) -> Result<(Output, Vec<Issue>), String> {
        self.validate()?;
        let h = self.geometric_altitude_m;
        let z = geopotential(h);
        let temperature_k = temperature(LAPSE_RATE_K_PER_M, z);
        let pressure_pa = pressure(temperature_k);
        let density_kg_m3 = density(pressure_pa, temperature_k);
        Ok((
            Output {
                geometric_altitude_m: h,
                geopotential_altitude_m: z,
                temperature_k,
                pressure_pa,
                density_kg_m3,
            },
            Vec::new(),
        ))
    }
}

/// `z = R0 * H / (R0 + H)`, in metres.
fn geopotential(h_m: f64) -> f64 {
    let r0_m = EARTH_RADIUS_M;
    himari_macros::formula_matlab!(
        r#"decl { r0_m : length [m]; h_m : length [m]; }
r0_m * h_m / (r0_m + h_m)"#
    )
}

/// `T = T0 + beta * z`, in kelvin.
fn temperature(beta_k_per_m: f64, z_m: f64) -> f64 {
    let t0_k = T0_K;
    himari_macros::formula_matlab!(
        r#"decl { t0_k : temp [K]; beta_k_per_m : temp_gradient [K/m]; z_m : length [m]; }
t0_k + beta_k_per_m * z_m"#
    )
}

/// `p = p0 * (T / T0) ^ (g0 / (R * |beta|))`, in pascals.
fn pressure(t_k: f64) -> f64 {
    let (p0_pa, t0_k, g0_m_per_s2, r_j_per_kg_k, beta_abs_k_per_m) = (
        P0_PA,
        T0_K,
        G0_M_PER_S2,
        R_AIR_J_PER_KG_K,
        LAPSE_RATE_K_PER_M.abs(),
    );
    himari_macros::formula_matlab!(
        r#"decl { p0_pa : pressure [Pa]; t_k : temp [K]; t0_k : temp [K]; g0_m_per_s2 : accel [m/s^2]; r_j_per_kg_k : specific_gas [J/kg/K]; beta_abs_k_per_m : grad [K/m]; }
p0_pa * (t_k / t0_k) ^ (g0_m_per_s2 / (r_j_per_kg_k * beta_abs_k_per_m))"#
    )
}

/// `rho = p / (R * T)`, in kg/m³.
fn density(p_pa: f64, t_k: f64) -> f64 {
    let r_j_per_kg_k = R_AIR_J_PER_KG_K;
    himari_macros::formula_matlab!(
        r#"decl { p_pa : pressure [Pa]; r_j_per_kg_k : specific_gas [J/kg/K]; t_k : temp [K]; }
p_pa / (r_j_per_kg_k * t_k)"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(h: f64) -> Output {
        Input {
            geometric_altitude_m: h,
        }
        .evaluate()
        .expect("inside the modelled layer")
        .0
    }

    /// Relative comparison that also accepts an exact zero.
    fn close(got: f64, want: f64, rel: f64) -> bool {
        (got - want).abs() <= rel * want.abs()
    }

    #[test]
    fn sea_level_is_the_reference_state() {
        let o = at(0.0);
        assert_eq!(o.geopotential_altitude_m, 0.0);
        assert_eq!(o.temperature_k, T0_K);
        assert_eq!(o.pressure_pa, P0_PA);
        // rho0 = p0 / (R * T0) = 1.22499916 kg/m^3 with this R; it is NOT exactly 1.225.
        assert!(close(o.density_kg_m3, 1.224_999_155_887_712_5, 1e-15));
        assert!((o.density_kg_m3 - 1.225).abs() < 1e-5);
    }

    #[test]
    fn one_kilometre_matches_independent_reference() {
        let o = at(1000.0);
        // References recomputed independently in exact double precision from the clauses.
        assert!(
            close(o.geopotential_altitude_m, 999.842_712_046_967_4, 1e-14),
            "z = {}",
            o.geopotential_altitude_m
        );
        assert!(
            close(o.temperature_k, 281.651_022_371_694_7, 1e-14),
            "T = {}",
            o.temperature_k
        );
        assert!(
            close(o.pressure_pa, 89_876.285_187_271_26, 1e-12),
            "p = {}",
            o.pressure_pa
        );
        assert!(
            close(o.density_kg_m3, 1.111_658_985_055_827_6, 1e-12),
            "rho = {}",
            o.density_kg_m3
        );
        // Sanity against the widely quoted value 89875 Pa for the ICAO table.
        assert!((o.pressure_pa - 89_875.0).abs() < 5.0);
    }

    #[test]
    fn eleven_km_boundary_is_inclusive() {
        let o = at(11_000.0);
        assert!(close(
            o.geopotential_altitude_m,
            10_980.998_045_468_38,
            1e-14
        ));
        assert!(close(o.temperature_k, 216.773_512_704_455_5, 1e-14));
        assert!(close(o.pressure_pa, 22_699.960_739_233_35, 1e-12));
        assert!(close(o.density_kg_m3, 0.364_801_564_186_560_1, 1e-12));
        // Temperature has dropped by beta * z, i.e. by ~71.4 K.
        assert!((T0_K - o.temperature_k - 0.0065 * 10_980.998_045_468_38).abs() < 1e-12);
    }

    #[test]
    fn pressure_follows_the_analytic_exponent() {
        // Independent restatement of the barometric relation with the exponent computed
        // outside the crate's constant.
        let exponent = G0_M_PER_S2 / (R_AIR_J_PER_KG_K * LAPSE_RATE_K_PER_M.abs());
        assert!(close(exponent, 5.255_876_113_278_519, 1e-15));
        for h in [0.0, 100.0, 2500.0, 7000.0, 11_000.0] {
            let o = at(h);
            let want = P0_PA * (o.temperature_k / T0_K).powf(exponent);
            assert!(close(o.pressure_pa, want, 1e-15), "p at {h}");
            let want_rho = o.pressure_pa / (R_AIR_J_PER_KG_K * o.temperature_k);
            assert!(close(o.density_kg_m3, want_rho, 1e-15), "rho at {h}");
        }
    }

    #[test]
    fn geopotential_relation_round_trips() {
        for h in [0.0, 1.0, 500.0, 5000.0, 11_000.0] {
            let o = at(h);
            let z = o.geopotential_altitude_m;
            assert!(z <= h, "geopotential must not exceed geometric at {h}");
            // The two heights differ by about H^2 / R0 (1.7e-3 relative at 11 km).
            let expected_gap = h * h / (EARTH_RADIUS_M + h);
            assert!(
                ((h - z) - expected_gap).abs() <= 1e-9 * (expected_gap + 1.0),
                "z={z} vs h={h}"
            );
            // Invert: H = R0 * z / (R0 - z).
            let back = EARTH_RADIUS_M * z / (EARTH_RADIUS_M - z);
            assert!((back - h).abs() <= 1e-6 * h.abs() + 1e-9);
        }
    }

    #[test]
    fn state_is_monotone_through_the_layer() {
        let mut prev = at(0.0);
        for h in [
            1.0, 500.0, 1000.0, 3000.0, 5000.0, 8000.0, 10_000.0, 11_000.0,
        ] {
            let cur = at(h);
            assert!(cur.temperature_k < prev.temperature_k, "T at {h}");
            assert!(cur.pressure_pa < prev.pressure_pa, "p at {h}");
            assert!(cur.density_kg_m3 < prev.density_kg_m3, "rho at {h}");
            prev = cur;
        }
    }

    #[test]
    fn out_of_range_is_an_error_not_an_extrapolation() {
        for h in [
            -1.0,
            11_020.0,
            20_000.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            let err = Input {
                geometric_altitude_m: h,
            }
            .evaluate()
            .expect_err("outside the layer");
            assert!(err.contains("geometric_altitude_m"), "{err}");
        }
    }
}
