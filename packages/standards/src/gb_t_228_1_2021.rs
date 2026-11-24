//! Tensile-property definitions as specified by GB/T 228.1-2021.
//!
//! GB/T 228.1-2021 ("Metallic materials — Tensile testing — Part 1: Method of test at
//! room temperature") is the current Chinese national standard for room-temperature
//! tensile testing; it was published 2021-12-31 and took effect 2022-07-01.
//!
//! # Scope of this module
//!
//! Only three definitions are implemented, exactly as the ISO sibling does:
//!
//! ```text
//! Rm = Fm / S0                       [Pa]  tensile strength
//! A  = (Lu - L0) / L0                [-]   percentage elongation after fracture (as a ratio)
//! E  = (F2 - F1) / (S0 * (e2 - e1))  [Pa]  modulus of elasticity from the elastic slope
//! ```
//!
//! They are the same arithmetic as [`crate::iso_6892_1_2019`], so this module re-uses that
//! tested evaluation rather than duplicating it. The identity (id, edition, provenance) is
//! its own, so a response can never be mislabelled as the ISO standard.
//!
//! # What is not claimed
//!
//! This module does **not** claim that GB/T 228.1-2021 is an identical adoption of any ISO
//! edition: the platform entry records that the standard adopts an ISO document, but the
//! adoption degree was not independently verified here. Only the definitions above are
//! implemented. Yield phenomena (`ReH`, `ReL`, `Rp0.2`), strain-rate method A/B tables,
//! specimen shapes, temperature tolerances, rounding rules and uncertainty are out of
//! scope. Not a conformance statement.

use crate::provenance::{Issue, Provenance, StandardId};
use crate::StandardInfo;

/// Canonical standard id.
pub const ID: StandardId = StandardId::new("GB-T-228.1");

/// Edition year accepted on the wire.
pub const EDITION: u16 = 2021;

/// Re-used request schema (identical fields to the ISO sibling).
pub use crate::iso_6892_1_2019::{Input, Output, Request, Values};

/// Where the formula comes from.
pub const PROVENANCE: Provenance = Provenance {
    standard: ID,
    edition: "2021",
    formula_source: "GB/T 228.1-2021, clause-level tensile-strength / elongation / modulus \
                     definitions",
    notes: "Current edition (published 2021-12-31, effective 2022-07-01). Only Rm = Fm/S0, \
            A = (Lu-L0)/L0 and E from the elastic slope are implemented. The degree to which \
            this edition adopts an ISO document was NOT independently verified, so no \
            equivalence is claimed. Yield phenomena, strain-rate method tables, specimen \
            shapes and uncertainty are NOT modelled. Not a conformance claim.",
};

/// Registry entry.
pub const INFO: StandardInfo = StandardInfo {
    id: ID,
    edition: EDITION,
    provenance: PROVENANCE,
    summary: "Tensile properties Rm / A / E per GB/T 228.1-2021",
};

/// Evaluates one tensile request with the shared, tested arithmetic.
pub fn evaluate(input: &Input) -> Result<(Output, Vec<Issue>), String> {
    input.evaluate()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gb_and_iso_ids_are_distinct_and_editions_pinned() {
        assert_eq!(ID.as_str(), "GB-T-228.1");
        assert_eq!(EDITION, 2021);
        assert_ne!(ID, crate::iso_6892_1_2019::ID);
    }

    #[test]
    fn gb_tensile_reproduces_the_documented_vectors() {
        let input = Input {
            maximum_force: Some(31_415.0),
            cross_section_area: Some(1.0e-4),
            initial_gauge_length: Some(0.05),
            final_gauge_length: Some(0.065),
            force_at_strain_1: None,
            force_at_strain_2: None,
            strain_1: None,
            strain_2: None,
        };
        let (out, _warnings) = evaluate(&input).unwrap();
        assert!(
            (out.tensile_strength_pa.unwrap() - 3.1415e8).abs() < 1.0,
            "{out:?}"
        );
        assert!((out.elongation.unwrap() - 0.3).abs() < 1e-12);
    }
}
