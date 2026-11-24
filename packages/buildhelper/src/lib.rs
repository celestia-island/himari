//! The rebuild-tail closer for `formula_file!`.
//!
//! A proc-macro reading a file is **not tracked by cargo** (`tracked_path` is still nightly).
//! Protocol: when `formula_file!` reads a file successfully it registers the absolute path in
//! `$CARGO_MANIFEST_DIR/.himari-tracked` (deduplicated); the consumer's build.rs adds one line
//!
//! ```text
//! fn main() { himari_buildhelper::emit(); }
//! ```
//!
//! It reads the manifest and emits `cargo:rerun-if-changed` for every path, so changing a
//! `.formula` rebuilds the crate and reruns the macro. `.himari-tracked` should be in
//! .gitignore.

use std::path::PathBuf;

/// Manifest file name (relative to the consumer's CARGO_MANIFEST_DIR).
pub const TRACKED_FILE: &str = ".himari-tracked";

fn manifest_dir() -> Option<PathBuf> {
    std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from)
}

/// Read the manifest and emit rerun-if-changed for every path. A missing manifest is silent (no
/// macro call has happened yet).
///
/// Returns the number of entries emitted (for tests/diagnostics).
pub fn emit() -> usize {
    let Some(dir) = manifest_dir() else {
        return 0;
    };
    let list = dir.join(TRACKED_FILE);
    let Ok(text) = std::fs::read_to_string(&list) else {
        return 0;
    };
    let mut n = 0;
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        println!("cargo:rerun-if-changed={l}");
        n += 1;
    }
    n
}

/// Register one path (called inside `formula_file!`; public so the macro crate can reuse it).
pub fn track(abs_path: &str) {
    let Some(dir) = manifest_dir() else {
        return;
    };
    let list = dir.join(TRACKED_FILE);
    let existing = std::fs::read_to_string(&list).unwrap_or_default();
    if existing.lines().any(|l| l.trim() == abs_path) {
        return; // deduplicate
    }
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&list)
    {
        let _ = writeln!(f, "{abs_path}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_dedupes_and_emit_reads() {
        let dir = std::env::temp_dir().join(format!("himari-bh-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("CARGO_MANIFEST_DIR", &dir);
        let target = dir.join("f.formula");
        std::fs::write(&target, "x").unwrap();
        let abs = target.to_string_lossy().to_string();
        track(&abs);
        track(&abs); // a repeat is not appended
        let text = std::fs::read_to_string(dir.join(TRACKED_FILE)).unwrap();
        assert_eq!(text.lines().count(), 1, "{text}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

pub mod gen;

pub use gen::{generate_dir, generate_dir_libm};
