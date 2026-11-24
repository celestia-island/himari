//! Brinell hardness as specified by GB/T 231.1-2018.
//!
//! GB/T 231.1-2018 ("Metallic materials — Brinell hardness test — Part 1: Test method")
//! is the current Chinese national standard for this test. According to the entry on the
//! National Standard Information Public Service Platform it **modifies and adopts**
//! ISO 6506-1:2014, and it fully replaces GB/T 231.1-2009.
//!
//! # Why this module is thin
//!
//! The relation between test force, ball diameter, indentation diameter and the hardness
//! number is the same arithmetic as the ISO parent standard, so this module re-uses the
//! already-tested evaluation in [`crate::iso_6506_1_2014`] instead of duplicating it.
//! What is *not* shared is the identity: this module carries its own standard id, edition
//! and provenance, so a response can never be mislabelled as ISO.
//!
//! The wire schema is therefore identical to the ISO module
//! (`force`, `ball_diameter`, `indentation_diameter`, all SI).
//!
//! # What is not claimed
//!
//! Only the clause-level arithmetic is implemented. Force-level selection for a given
//! material, ball type choice, dwell time, specimen thickness limits, uncertainty budget
//! and the standard's own tables are out of scope. This is not a conformance statement.

use crate::provenance::{Provenance, StandardId};
use crate::StandardInfo;

/// Canonical standard id.
pub const ID: StandardId = StandardId::new("GB-T-231.1");

/// Edition year accepted on the wire.
pub const EDITION: u16 = 2018;

/// Re-used request schema (identical fields to the ISO parent standard).
pub use crate::iso_6506_1_2014::{Input, Output, Request, Values};

/// Where the formula comes from.
pub const PROVENANCE: Provenance = Provenance {
    standard: ID,
    edition: "2018",
    formula_source: "GB/T 231.1-2018, clause-level hardness-number and indentation-area \
                     definitions (arithmetic shared with ISO 6506-1:2014, which the platform \
                     entry records this edition as modifying and adopting)",
    notes: "GB/T 231.1-2018 is the current edition and fully replaces GB/T 231.1-2009. Per \
            the National Standard Information Public Service Platform it modifies and adopts \
            ISO 6506-1:2014; only the hardness arithmetic is implemented here. Force-level \
            tables, ball-type selection, dwell time, thickness limits and uncertainty are NOT \
            modelled. Evaluation is shared with the ISO module, so the two ids always agree. \
            Not a conformance claim.",
};

/// Registry entry.
pub const INFO: StandardInfo = StandardInfo {
    id: ID,
    edition: EDITION,
    provenance: PROVENANCE,
    summary: "Brinell hardness HBW per GB/T 231.1-2018 (modifies and adopts ISO 6506-1:2014)",
};

/// Evaluates one Brinell request with the shared, tested arithmetic.
pub fn evaluate(input: &Input) -> Result<(Output, Vec<crate::provenance::Issue>), String> {
    input.evaluate()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gb_and_iso_ids_are_distinct_and_editions_pinned() {
        assert_eq!(ID.as_str(), "GB-T-231.1");
        assert_eq!(EDITION, 2018);
        assert_ne!(ID, crate::iso_6506_1_2014::ID);
    }

    #[test]
    fn gb_brinell_reproduces_the_documented_vector() {
        let input = Input {
            force: 29420.0,
            ball_diameter: 0.01,
            indentation_diameter: 0.004,
        };
        let (out, warnings) = evaluate(&input).unwrap();
        assert!(warnings.is_empty());
        assert!(
            (out.hbw - 228.831_198_210_826_27).abs() < 1e-9,
            "{}",
            out.hbw
        );
    }
}
