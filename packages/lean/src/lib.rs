//! `himari-lean` — a Lean 4 co-process for himari.
//!
//! # What this crate is for
//!
//! It takes a small, self-contained proof obligation, generates the Lean source
//! for it, hands that source to a real `lean` executable over stdin, and reports
//! what happened in a structured form. It never claims more than it observed.
//!
//! # The rules it is built to keep
//!
//! * **Subprocess, not binding.** `lean --stdin --json -T <heartbeats> -M <MB>`.
//!   No Lake project, no package index, no network access: generated files have
//!   no imports and need nothing but the toolchain's own library.
//! * **Dynamic data is literal-only.** Everything that comes from a request lands
//!   in exactly one place — the integer literals of a *named example theorem
//!   statement*. Not in a data definition, not in an axiom, not in an identifier.
//!   See [`request`] for the intake grammar and the deny-list.
//! * **Exact arithmetic, kernel-checked.** The obligation is over `Int`, closed,
//!   and discharged by `by decide`. No `native_decide`, no `#eval`, no
//!   `implemented_by`, no `extern`, no `partial`, no `sorry`, no user axioms.
//!   A test asserts the generated text against that list.
//! * **An independent oracle.** The same determinant is computed here in Rust
//!   (`i128`, a different expansion from the one written into Lean), so Lean is
//!   never the only witness. See [`oracle`].
//! * **Axioms are read, not assumed.** The `#print axioms` output is parsed and
//!   only `propext`, `Classical.choice` and `Quot.sound` are tolerated. An
//!   unreadable report is a violation, never a pass. See [`axioms`].
//! * **Missing Lean is a first-class non-success.** [`Status::LeanMissing`] means
//!   *nothing was checked*; it is not an error to swallow and it is certainly not
//!   a success. See [`status`].
//!
//! # Example
//!
//! ```
//! use himari_lean::{verify_request, RunOptions, Status};
//!
//! let req = r#"{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10]}"#;
//! // Without a toolchain the run must say so rather than pass.
//! let rep = verify_request(req, &RunOptions::default().without_lean());
//! assert_eq!(rep.status, Status::LeanMissing);
//! assert!(!rep.status.is_verified());
//! assert_eq!(rep.oracle.det3, Some(-3));
//! ```

pub mod axioms;
pub mod oracle;
pub mod report;
pub mod request;
pub mod runner;
pub mod source;
pub mod status;
pub mod verify;

pub use report::{Diagnostic, OracleReport, Report};
pub use request::{parse_request, Det3Request, RequestError};
pub use runner::{Discovery, LeanRun, LeanToolchain, OVERRIDE_ENV};
pub use status::Status;
pub use verify::{verify_request, verify_typed, RunOptions};

/// Flatten a report into the CLI's exit code. `0` only for a verified run.
pub fn exit_code_for(rep: &Report) -> u8 {
    rep.status.exit_code()
}
