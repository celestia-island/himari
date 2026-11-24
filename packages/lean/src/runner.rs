//! Driving the `lean` executable as a co-process.
//!
//! ## Why a subprocess and not a binding
//!
//! The point of the co-process is separability: the thing that decides whether
//! a theorem holds must be apart from the tool that proposes it, so that "Lean
//! said yes" is evidence a human can re-run by hand. The command line is part of
//! the contract:
//!
//! ```text
//! lean --stdin --json -T <heartbeats> -M <MB>
//! ```
//!
//! (`-T`/`--timeout` is a heartbeat budget, `-M`/`--memory` a soft memory cap.)
//! The source goes in on stdin, so no filesystem writes are needed, and no Lake
//! project, package index or network access is involved: the generated file has
//! no imports and needs nothing beyond the toolchain's own library.
//!
//! ## Missing Lean is an outcome, not a panic
//!
//! [`LeanToolchain::discover`] returns `Ok(None)` when there is no usable Lean.
//! The caller turns that into a non-success status. Nothing was proved, and
//! saying otherwise would be the worst failure mode this crate has.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Environment variable that overrides which `lean` is run.
pub const OVERRIDE_ENV: &str = "HIMARI_LEAN_BIN";

/// How often to poll for child exit while waiting for the deadline.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// How long to wait for the child's pipes to close once it has exited.
///
/// Bounded so that a lingering grandchild cannot stretch a run past its deadline;
/// generous enough that ordinary output is never truncated.
const DRAIN_GRACE: Duration = Duration::from_secs(2);

/// A located `lean` executable and its reported version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeanToolchain {
    /// Path (or bare name) handed to the OS when spawning.
    pub path: PathBuf,
    /// How the path was obtained, for diagnostics.
    pub origin: &'static str,
    /// First non-empty line of `lean --version`, if the probe succeeded.
    pub version: Option<String>,
}

/// The result of discovering a toolchain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Discovery {
    /// A `lean` that answered `--version` was found.
    Found(LeanToolchain),
    /// No `lean` is available. This is a normal, expected state.
    Missing {
        /// Every location that was consulted, for the diagnostics payload.
        searched: Vec<String>,
    },
    /// Something was configured but did not work.
    Broken {
        /// The configuration that failed.
        detail: String,
    },
}

/// One completed (or abandoned) `lean` invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeanRun {
    /// The exact argv, excluding argv\[0\].
    pub argv: Vec<String>,
    /// Process exit code, if the child exited on its own.
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    /// True when the deadline was hit and the child was killed.
    pub timed_out: bool,
}

impl LeanToolchain {
    /// Locate `lean`: the explicit override first, then `PATH`.
    pub fn discover() -> Discovery {
        let mut searched = Vec::new();

        if let Some(raw) = std::env::var_os(OVERRIDE_ENV) {
            let raw = raw.to_string_lossy().trim().to_string();
            if !raw.is_empty() {
                let spec = format!("{OVERRIDE_ENV}={raw}");
                searched.push(spec.clone());
                let candidate = LeanToolchain {
                    path: PathBuf::from(&raw),
                    origin: "env:HIMARI_LEAN_BIN",
                    version: None,
                };
                return match candidate.probe_version() {
                    Ok(version) => Discovery::Found(LeanToolchain {
                        version: Some(version),
                        ..candidate
                    }),
                    Err(detail) => Discovery::Broken {
                        detail: format!("{spec}: {detail}"),
                    },
                };
            }
        }

        for dir in path_dirs() {
            searched.push(dir.display().to_string());
            let candidate = dir.join(exe_name("lean"));
            if !is_executable_file(&candidate) {
                continue;
            }
            let tc = LeanToolchain {
                path: candidate,
                origin: "PATH",
                version: None,
            };
            return match tc.probe_version() {
                Ok(version) => Discovery::Found(LeanToolchain {
                    version: Some(version),
                    ..tc
                }),
                // An executable named `lean` that cannot report a version is
                // reported as broken rather than silently skipped: pretending it
                // is missing would hide a real environment problem.
                Err(detail) => Discovery::Broken {
                    detail: format!("{}: {detail}", tc.path.display()),
                },
            };
        }
        Discovery::Missing { searched }
    }

    /// Run `lean --version` and return its first non-empty line.
    fn probe_version(&self) -> Result<String, String> {
        let out = Command::new(&self.path)
            .arg("--version")
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("cannot execute `--version`: {e}"))?;
        if !out.status.success() {
            return Err(format!("`--version` exited with {}", out.status));
        }
        let text = String::from_utf8_lossy(&out.stdout);
        text.lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .map(str::to_string)
            .ok_or_else(|| "`--version` printed nothing".to_string())
    }

    /// The argv (excluding argv\[0\]) used for a checking run.
    pub fn check_argv(heartbeats: u64, memory_mb: u64) -> Vec<String> {
        vec![
            "--stdin".to_string(),
            "--json".to_string(),
            "-T".to_string(),
            heartbeats.to_string(),
            "-M".to_string(),
            memory_mb.to_string(),
        ]
    }

    /// Feed `source` to Lean on stdin and collect the result.
    ///
    /// `timeout` bounds the whole exchange, including startup. On timeout the
    /// child is killed and [`LeanRun::timed_out`] is set, which the caller
    /// reports as a resource exhaustion.
    pub fn check(
        &self,
        source: &str,
        heartbeats: u64,
        memory_mb: u64,
        timeout: Duration,
    ) -> Result<LeanRun, String> {
        let argv = Self::check_argv(heartbeats, memory_mb);
        let mut child = Command::new(&self.path)
            .args(&argv)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("cannot execute: {e}"))?;

        let source = source.as_bytes().to_vec();
        let mut stdin = child.stdin.take().ok_or("child has no stdin")?;
        let writer = thread::spawn(move || {
            // A write error means Lean exited before reading everything (e.g. an
            // early parse error). The diagnosis comes from the exit status and
            // stderr, so the writer's own error is not propagated.
            let _ = stdin.write_all(&source);
            let _ = stdin.flush();
            drop(stdin);
        });

        let mut stdout = child.stdout.take().ok_or("child has no stdout")?;
        let mut stderr = child.stderr.take().ok_or("child has no stderr")?;
        let (out_tx, out_rx) = std::sync::mpsc::channel();
        let (err_tx, err_rx) = std::sync::mpsc::channel();
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = stdout.read_to_end(&mut buf);
            let _ = out_tx.send(buf);
        });
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = stderr.read_to_end(&mut buf);
            let _ = err_tx.send(buf);
        });

        let deadline = Instant::now() + timeout;
        let mut timed_out = false;
        let exit_code = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status.code(),
                Ok(None) => {}
                Err(e) => return Err(format!("waiting for lean failed: {e}")),
            }
            if Instant::now() >= deadline {
                timed_out = true;
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            thread::sleep(POLL_INTERVAL);
        };

        // The child is gone, so both pipes should hit EOF. They need not: a child
        // that forked (a wrapper script, a build helper) can leave a grandchild
        // holding the write ends. Draining unconditionally would then block for as
        // long as that process lives, which would defeat the deadline entirely.
        // The readers therefore get a bounded grace period and are detached
        // afterwards; they exit on their own once the pipe finally closes.
        let stdout = out_rx.recv_timeout(DRAIN_GRACE).unwrap_or_default();
        let stderr = err_rx.recv_timeout(DRAIN_GRACE).unwrap_or_default();
        // The writer only ever blocks on a child that is still reading, and that
        // child is gone by now.
        let _ = writer.join();
        Ok(LeanRun {
            argv,
            exit_code,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            timed_out,
        })
    }
}

fn exe_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}

/// Directories listed in `PATH`, in order, with empty entries dropped.
fn path_dirs() -> Vec<PathBuf> {
    match std::env::var_os("PATH") {
        Some(v) => std::env::split_paths(&v)
            .filter(|d| !d.as_os_str().is_empty())
            .collect(),
        None => Vec::new(),
    }
}

#[cfg(unix)]
fn is_executable_file(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(p) {
        Ok(md) => md.is_file() && md.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

#[cfg(not(unix))]
fn is_executable_file(p: &Path) -> bool {
    p.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argv_matches_the_documented_contract() {
        assert_eq!(
            LeanToolchain::check_argv(400_000, 2048),
            vec!["--stdin", "--json", "-T", "400000", "-M", "2048"]
        );
    }

    #[test]
    fn executable_detection_requires_the_execute_bit() {
        let dir = std::env::temp_dir().join(format!(
            "himari-lean-exec-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("lean");
        std::fs::write(&f, b"not a program").unwrap();
        assert!(!is_executable_file(&f), "plain file must not qualify");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&f).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&f, perms).unwrap();
            assert!(is_executable_file(&f));
        }
        assert!(!is_executable_file(&dir), "a directory must not qualify");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unspawnable_binary_is_an_error_not_a_missing_state() {
        let tc = LeanToolchain {
            path: PathBuf::from("/nonexistent/himari-lean-test/lean"),
            origin: "test",
            version: None,
        };
        let err = tc
            .check(
                "theorem x : True := trivial\n",
                1000,
                256,
                Duration::from_secs(5),
            )
            .unwrap_err();
        assert!(err.contains("cannot execute"), "{err}");
    }
}
