//! himari-standards: typed calculators for formulas taken from published engineering
//! standards, each with explicit provenance and its own input schema.
//!
//! This crate is deliberately narrow. It does **not** aim to be a general units library and
//! it does **not** claim conformance with any standard: every module computes one specific
//! clause-level relation as written in the source text cited in that module's `PROVENANCE`.
//! Nothing here quotes standard prose; formulas are restated as plain expressions.
//! Unit conversions are intentionally absent — callers pass SI base units and convert
//! on their own side (see each module's documented unit contract).
//!
//! # Layout
//!
//! | module | standard id | edition | computed quantities |
//! |---|---|---|---|
//! | [`iso_6506_1_2014`] | `ISO-6506-1` | `2014` | Brinell hardness `HBW` |
//! | [`iso_6892_1_2019`] | `ISO-6892-1` | `2019` | `Rm`, `A`, `E` |
//! | [`iso_2533_1975`] | `ISO-2533` | `1975` | `T`, `p`, `rho`, geopotential `z` |
//! | [`gb_t_231_1_2018`] | `GB-T-231.1` | `2018` | Brinell hardness `HBW` |
//! | [`gb_t_228_1_2021`] | `GB-T-228.1` | `2021` | `Rm`, `A`, `E` |
//! | [`gb_t_8170_2008`] | `GB-T-8170` | `2008` | rounded decimal value |
//!
//! Each module exposes:
//!
//! - `pub const ID: StandardId` — the only accepted spelling of the standard id;
//! - `pub const EDITION: u16` — the edition year;
//! - `pub const PROVENANCE: Provenance` — formula source and edition, static metadata;
//! - a typed `Input` struct with `validate()` + `evaluate()`;
//! - a `Request` struct whose serde form is the wire schema accepted by [`run`] / the
//!   `himari-standard` CLI;
//! - a `Response`/`Values` pair whose serde form is the stable output document.
//!
//! `cargo run -p himari-standards` (or the installed `himari-standard`) evaluates one
//! request: `--input <path>` or stdin, `--format json|toml`, `--list` for the exact ids
//! and editions.
//!
//! # Wire format
//!
//! One request is one JSON object (or the same shape in TOML). The envelope keys are the
//! same for every standard; the remaining keys are the standard's typed input fields.
//! Unknown keys are rejected, never ignored.
//!
//! ```json
//! {
//!   "standard": "ISO-6506-1",
//!   "edition": 2014,
//!   "force": 29420.0,
//!   "ball_diameter": 0.01,
//!   "indentation_diameter": 0.004
//! }
//! ```
//!
//! The identical request in TOML:
//!
//! ```toml
//! standard = "ISO-6506-1"
//! edition = 2014
//! force = 29420.0
//! ball_diameter = 0.01
//! indentation_diameter = 0.004
//! ```
//!
//! The response document has a fixed shape for every standard:
//!
//! ```json
//! {
//!   "standard": "ISO-6506-1",
//!   "edition": "2014",
//!   "provenance": {
//!     "standard": "ISO-6506-1",
//!     "edition": "2014",
//!     "formula_source": "ISO 6506-1:2014, clause 4 (...)",
//!     "notes": "...what is not modelled..."
//!   },
//!   "values": { "ratio": 0.4, "pressure_pa": 2243443119.714, "hbw": 228.83119821082636 },
//!   "warnings": [],
//!   "errors": []
//! }
//! ```
//!
//! `values` holds only the quantities that standard defines; a quantity whose inputs were
//! not requested (see [`iso_6892_1_2019`]) is `null`, so the keys themselves never move.
//! On failure the same document is returned with `values` and `provenance` set to `null`
//! and the cause in `errors`; the CLI prints it on stdout and exits 1.
//!
//! Input format: one JSON object, or the same keys as a TOML document. `--format` forces
//! the parser; otherwise the file extension (`.json`/`.toml`) decides, and failing that the
//! content does — only JSON can begin a document with `{` or `[`. `edition` accepts a year
//! number or a year string.
//!
//! Requests whose decimals take part in a *rounding decision* (see
//! [`gb_t_8170_2008`]) carry those numbers as decimal **strings**; JSON numbers are
//! rejected there because a binary float has already lost the trailing digit by the time
//! the crate sees it. Other standards accept JSON/TOML numbers or decimal strings for
//! parameters, because comparison tolerances there already absorb float parsing.
//!
//! [`run`] is the single dispatch entry point: it parses (or receives) an envelope, checks
//! the `standard` + `edition` pair against the registry, deserializes the typed input, runs
//! validation and evaluation, and converts every failure into a structured
//! [`StandardError`] — no input reaches a `panic!` or an `unwrap`.

mod dispatch;
mod envelope;
mod jsonnum;
mod provenance;

pub mod gb_t_228_1_2021;
pub mod gb_t_231_1_2018;
pub mod gb_t_8170_2008;
pub mod iso_2533_1975;
pub mod iso_6506_1_2014;
pub mod iso_6892_1_2019;

pub use dispatch::{list, run, run_json, StandardError, StandardOutcome};
pub use envelope::Format;
pub use jsonnum::Param;
pub use provenance::{Issue, Provenance, Severity, StandardId};

/// Registry entry: everything the CLI needs to describe one supported standard.
#[derive(Clone, Copy, Debug)]
pub struct StandardInfo {
    /// Canonical standard id (also the only accepted spelling on the wire).
    pub id: StandardId,
    /// Edition year accepted on the wire.
    pub edition: u16,
    /// Provenance of the formula(s) this crate evaluates.
    pub provenance: Provenance,
    /// One-line description of what the standard's calculator returns.
    pub summary: &'static str,
}

/// Every standard this crate implements, in a stable order.
pub const STANDARDS: &[StandardInfo] = &[
    iso_6506_1_2014::INFO,
    iso_6892_1_2019::INFO,
    iso_2533_1975::INFO,
    gb_t_231_1_2018::INFO,
    gb_t_228_1_2021::INFO,
    gb_t_8170_2008::INFO,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_ids_and_editions_are_unique() {
        let mut seen = Vec::new();
        for info in STANDARDS {
            let key = (info.id.as_str(), info.edition);
            assert!(!seen.contains(&key), "duplicate registry entry {key:?}");
            seen.push(key);
            assert!(
                info.provenance.formula_source.contains("clause"),
                "provenance of {} should point at a clause",
                info.id
            );
            assert!(
                !info.summary.is_empty(),
                "registry entry {} needs a summary",
                info.id
            );
        }
    }
}
