//! Static provenance metadata for each standard, plus the diagnostic value types shared
//! by every module.

use serde::Serialize;
use std::fmt;

/// Canonical spelling of a standard's identifier.
///
/// Kept as a newtype — not a `&'static str` alias — so that a dispatching site cannot mix
/// two standards' strings up without the type system noticing, and so the accepted
/// spelling lives in exactly one place per standard (`ID::new()`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct StandardId(&'static str);

impl StandardId {
    /// Builds an id. Only called with the constants declared in each standard module.
    pub const fn new(spelling: &'static str) -> Self {
        Self(spelling)
    }

    /// The canonical wire spelling, e.g. `"ISO-6506-1"`.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for StandardId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Where a formula came from, and how far it may be trusted for a given edition.
///
/// This is metadata, not a compliance statement: `notes` spells out what has *not* been
/// verified (e.g. whether the cited edition is still current, which rounding rules are
/// applied at which step).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Provenance {
    /// Standard identifier (canonical spelling).
    pub standard: StandardId,
    /// Edition year of the source text the formula was read from.
    pub edition: &'static str,
    /// Clause / table / section inside that edition.
    pub formula_source: &'static str,
    /// Restriction and caveats that a caller must read before trusting a value.
    pub notes: &'static str,
}

/// How serious an [`Issue`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The result is still produced; the condition lies outside the standard's own
    /// validity range, or rests on an extra assumption.
    Warning,
}

/// One diagnostic emitted alongside a result.
///
/// Warnings never suppress a value: the standard documents which inputs are outside its
/// stated range without forbidding them (e.g. an unusual indentation ratio in a Brinell
/// test is still measurable). Inputs that cannot be interpreted at all are errors and have
/// no value at all — see [`crate::StandardError`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Issue {
    /// Stable machine-readable code, e.g. `"indentation-ratio-outside-range"`.
    pub code: &'static str,
    /// The input field the condition refers to, or `null` when it is not field-specific.
    pub field: Option<&'static str>,
    /// Human-readable explanation (English, no standard prose).
    pub message: String,
    /// How serious the condition is.
    pub severity: Severity,
}

impl Issue {
    /// Builds a warning attached to one input field.
    pub fn warning(
        code: &'static str,
        field: impl Into<Option<&'static str>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            field: field.into(),
            message: message.into(),
            severity: Severity::Warning,
        }
    }
}
