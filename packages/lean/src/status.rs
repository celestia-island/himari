//! The closed set of outcomes a Lean co-process run may end in.
//!
//! The list is ordered from "we could not start" to "we have a proof", and the
//! only member that means verification happened is [`Status::Success`]. Every
//! other member is a **non-success**: it must never be rendered as "verified",
//! and it must never exit with status 0 at the CLI boundary.
//!
//! Two of them are easy to conflate and must not be:
//!
//! * [`Status::LeanMissing`] — the toolchain was not there, so *no theorem was
//!   ever checked*. This says nothing about whether the statement is true.
//! * [`Status::ProofFailed`] — `lean` ran and refused the theorem.
//!
//! Treating the first as the second (or worse, as the third "fine, assume ok")
//! is how an unverified pipeline reports green.

use std::fmt;

/// Terminal state of one co-process run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Status {
    /// The `lean` binary could not be located or could not be executed.
    LeanMissing,
    /// The request was malformed, out of scope, or contained a forbidden token.
    InvalidRequest,
    /// Lean ran but the theorem did not check (compile error, or `sorry`).
    ProofFailed,
    /// A guard (heartbeats / memory / wall clock) bounded the run out.
    ResourceExhausted,
    /// The theorem checked, but `#print axioms` named something outside the
    /// allow-list.
    AxiomViolation,
    /// The Rust oracle and the literal expected value in the request disagree.
    OracleMismatch,
    /// Lean accepted the theorem, the axioms are clean, and the oracle agrees.
    Success,
}

impl Status {
    /// Canonical wire name (the value of the `"status"` field).
    pub const fn as_str(self) -> &'static str {
        match self {
            Status::LeanMissing => "lean_missing",
            Status::InvalidRequest => "invalid_request",
            Status::ProofFailed => "proof_failed",
            Status::ResourceExhausted => "resource_exhausted",
            Status::AxiomViolation => "axiom_violation",
            Status::OracleMismatch => "oracle_mismatch",
            Status::Success => "success",
        }
    }

    /// Whether this outcome means "the theorem is machine-checked".
    ///
    /// Only [`Status::Success`] does. This is deliberately a method rather than
    /// a caller-side `status == Success` convention so that downstream code has
    /// one place to read the rule from.
    pub const fn is_verified(self) -> bool {
        matches!(self, Status::Success)
    }

    /// Process exit code for this outcome: `0` only for [`Status::Success`].
    pub const fn exit_code(self) -> u8 {
        match self {
            Status::Success => 0,
            Status::LeanMissing => 3,
            Status::InvalidRequest => 4,
            Status::ProofFailed => 5,
            Status::ResourceExhausted => 6,
            Status::AxiomViolation => 7,
            Status::OracleMismatch => 8,
        }
    }

    /// Parse a wire name back into a status. Unknown names are rejected rather
    /// than defaulted, so a newer producer cannot silently downgrade into an
    /// older consumer's assumption.
    pub fn parse(name: &str) -> Option<Status> {
        Some(match name {
            "lean_missing" => Status::LeanMissing,
            "invalid_request" => Status::InvalidRequest,
            "proof_failed" => Status::ProofFailed,
            "resource_exhausted" => Status::ResourceExhausted,
            "axiom_violation" => Status::AxiomViolation,
            "oracle_mismatch" => Status::OracleMismatch,
            "success" => Status::Success,
            _ => return None,
        })
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_success_counts_as_verified() {
        let all = [
            Status::LeanMissing,
            Status::InvalidRequest,
            Status::ProofFailed,
            Status::ResourceExhausted,
            Status::AxiomViolation,
            Status::OracleMismatch,
            Status::Success,
        ];
        for s in all {
            assert_eq!(s.is_verified(), s == Status::Success, "{s}");
        }
    }

    #[test]
    fn lean_missing_is_never_verified_and_never_exits_zero() {
        assert!(!Status::LeanMissing.is_verified());
        assert_ne!(Status::LeanMissing.exit_code(), 0);
        assert_eq!(Status::LeanMissing.as_str(), "lean_missing");
    }

    #[test]
    fn every_non_success_has_a_distinct_nonzero_exit_code() {
        let names = [
            Status::LeanMissing,
            Status::InvalidRequest,
            Status::ProofFailed,
            Status::ResourceExhausted,
            Status::AxiomViolation,
            Status::OracleMismatch,
        ];
        let mut codes: Vec<u8> = names.iter().map(|s| s.exit_code()).collect();
        assert!(codes.iter().all(|&c| c != 0), "{codes:?}");
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), names.len(), "exit codes must be distinct");
        assert_eq!(Status::Success.exit_code(), 0);
    }

    #[test]
    fn wire_names_round_trip_and_unknown_names_are_rejected() {
        for s in [
            Status::LeanMissing,
            Status::InvalidRequest,
            Status::ProofFailed,
            Status::ResourceExhausted,
            Status::AxiomViolation,
            Status::OracleMismatch,
            Status::Success,
        ] {
            assert_eq!(Status::parse(s.as_str()), Some(s));
        }
        assert_eq!(Status::parse("verified"), None);
        assert_eq!(Status::parse("ok"), None);
        assert_eq!(Status::parse(""), None);
    }
}
