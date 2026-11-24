//! Shared helpers for the co-process integration tests.
//!
//! ## Two kinds of Lean in these tests
//!
//! Neither is a real Lean 4 toolchain, and none of them proves anything:
//!
//! * **Stub `lean`** — a tiny shell script that emulates exactly the parts of the
//!   `lean --stdin --json -T .. -M ..` contract this crate depends on: it records
//!   its argv and stdin, and prints a `#print axioms` line whose content the test
//!   chooses. This is how the success / axiom-violation / proof-failure paths are
//!   exercised without a toolchain. A stub success is a statement about *this
//!   crate's plumbing*, never about Lean.
//! * **Real `lean`** — located via `HIMARI_LEAN_BIN` or `PATH`. The one test that
//!   needs it is `#[ignore]`d and says why when it is skipped.
//!
//! Every test that uses a stub puts the word `stub` in its name so that no one
//! can read a green stub test as "a Lean proof was checked".

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

/// A scratch directory that is removed on drop.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new(tag: &str) -> TempDir {
        let mut path = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        path.push(format!("himari-lean-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&path).expect("create temp dir");
        TempDir { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// The crate's CLI binary, as built by cargo for this test run.
pub fn cli_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_himari-lean"))
}

/// Result of one CLI invocation: exit status, stdout and stderr.
pub struct CliOutcome {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CliOutcome {
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.stdout).unwrap_or_else(|e| {
            panic!(
                "stdout was not JSON ({e}):\n--- stdout ---\n{}\n--- stderr ---\n{}",
                self.stdout, self.stderr
            )
        })
    }
}

/// Run the CLI with `HIMARI_LEAN_BIN` pointing at `lean` (or unset when `None`).
pub fn run_cli(lean: Option<&Path>, args: &[&str], stdin: Option<&str>) -> CliOutcome {
    run_cli_with_env(lean, args, stdin, &[])
}

/// Run the CLI with extra environment entries layered on top.
pub fn run_cli_with_env(
    lean: Option<&Path>,
    args: &[&str],
    stdin: Option<&str>,
    env: &[(&str, &str)],
) -> CliOutcome {
    let mut cmd = Command::new(cli_path());
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match lean {
        Some(p) => {
            cmd.env("HIMARI_LEAN_BIN", p);
        }
        None => {
            cmd.env_remove("HIMARI_LEAN_BIN");
        }
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().expect("spawn himari-lean");
    if let Some(text) = stdin {
        use std::io::Write;
        let mut si = child.stdin.take().expect("stdin");
        si.write_all(text.as_bytes()).expect("write stdin");
    } else {
        drop(child.stdin.take());
    }
    let Output {
        status,
        stdout,
        stderr,
    } = child.wait_with_output().expect("wait for himari-lean");
    CliOutcome {
        code: status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    }
}

/// Write a stub `lean` into `dir` and return its path.
///
/// The script records argv (one per line) and stdin to files next to itself, so a
/// test can assert the exact command line and the exact source that was sent.
/// `behaviour` is executed after the recording setup; it decides the exit status
/// and what goes to stdout.
///
/// The `--version` probe is answered separately and does **not** touch the stdin
/// recording, so the file always holds the source of the checking run.
#[cfg(unix)]
pub fn stub_lean(dir: &Path, behaviour: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("lean-stub");
    let script = format!(
        r#"#!/bin/sh
dir="{dir}"
if [ "$1" = "--version" ]; then
  echo "Lean (version 4.0.0-stub, x86_64-unknown-linux-gnu, commit stub) [himari-lean test stub]"
  exit 0
fi
: > "$dir/argv.txt"
for a in "$@"; do printf '%s\n' "$a" >> "$dir/argv.txt"; done
cat > "$dir/stdin.lean"
{behaviour}
"#,
        dir = dir.display(),
        behaviour = behaviour
    );
    std::fs::write(&path, script).expect("write stub");
    let mut perms = std::fs::metadata(&path).expect("stat stub").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).expect("chmod stub");
    path
}

/// A stub that claims the example theorem rests only on the allowed axioms.
#[cfg(unix)]
pub const STUB_CLEAN_AXIOMS: &str = r#"cat <<'EOF'
{"severity":"info","pos":{"line":14,"column":0},"endPos":{"line":14,"column":0},"data":"'det3_example' depends on axioms: [propext, Classical.choice, Quot.sound]"}
EOF
exit 0"#;

/// A stub that reports an axiom outside the allow-list.
#[cfg(unix)]
pub const STUB_DIRTY_AXIOMS: &str = r#"cat <<'EOF'
{"severity":"info","data":"'det3_example' depends on axioms: [propext, Lean.ofReduceBool, Lean.trustCompiler]"}
EOF
exit 0"#;

/// A stub that rejects the theorem as Lean would on a failed `decide`.
#[cfg(unix)]
pub const STUB_PROOF_FAILED: &str = r#"cat <<'EOF'
{"severity":"error","pos":{"line":14,"column":65},"data":"tactic 'decide' proved that the proposition ... is false"}
EOF
exit 1"#;

/// A stub that exits cleanly but never prints the axiom query result.
#[cfg(unix)]
pub const STUB_NO_AXIOM_LINE: &str = r#"cat <<'EOF'
{"severity":"info","data":"unrelated chatter"}
EOF
exit 0"#;

/// A stub that hangs, so the wall-clock cap has to kill it.
#[cfg(unix)]
pub const STUB_HANGS: &str = r#"sleep 30
exit 0"#;

/// Discover a real `lean`, the same way the crate does.
pub fn real_lean() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("HIMARI_LEAN_BIN") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let c = dir.join("lean");
        if c.is_file() {
            return Some(c);
        }
    }
    None
}
