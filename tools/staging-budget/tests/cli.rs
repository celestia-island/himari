//! Process-level contract of the binary itself: `main()` reads the `CI` environment once and
//! hands the flag to `cli()`. Unit tests cannot reach that read, so it is pinned here by
//! spawning the built executable. `tests/` is outside the staging scan by design.

use std::path::PathBuf;
use std::process::Command;

/// A scratch in-budget workspace with one counted site (not a git repository).
fn workspace() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let leaf = std::env::temp_dir().join(format!("sb-cli-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&leaf);
    std::fs::create_dir_all(leaf.join("packages/p/src")).unwrap();
    std::fs::create_dir_all(leaf.join("tools/staging-budget/src")).unwrap();
    std::fs::write(leaf.join("tools/staging-budget/src/ceiling.txt"), "10usize").unwrap();
    std::fs::write(
        leaf.join("packages/p/src/lib.rs"),
        r#"fn f() -> Result<(), Error> { Err(Error::from("x")) }"#,
    )
    .unwrap();
    leaf
}

#[test]
fn ci_env_implies_against_git_so_a_non_git_tree_cannot_vouch() {
    let exe = env!("CARGO_BIN_EXE_himari-staging-budget");
    let ws = workspace();

    // Without CI: clean, exit 0 (whatever the surrounding environment happens to set).
    let out = Command::new(exe)
        .arg("--root")
        .arg(&ws)
        .env_remove("CI")
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).ends_with("staging-budget: clean\n"),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );

    // With CI set, the monotonicity check is implied; the scratch tree is not a git
    // repository, so the baseline is unavailable → exit 2, never a silent downgrade.
    let out = Command::new(exe)
        .arg("--root")
        .arg(&ws)
        .env("CI", "1")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("--against-git requested but the baseline is unavailable"),
        "stderr: {err}"
    );
    assert!(
        out.stdout.is_empty(),
        "no summary on failure: {:?}",
        out.stdout
    );

    let _ = std::fs::remove_dir_all(&ws);
}

#[test]
fn list_flag_reaches_the_binary_through_argv() {
    let exe = env!("CARGO_BIN_EXE_himari-staging-budget");
    let ws = workspace();
    let out = Command::new(exe)
        .args(["--root", ws.to_str().unwrap(), "--list"])
        .env_remove("CI")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut lines = stdout.lines();
    assert_eq!(
        lines.next(),
        Some("packages/p/src/lib.rs:1  error_from     fn f() -> Result<(), Error> { Err(Error::from(\"x\")) }")
    );
    assert!(stdout.ends_with("staging-budget: clean\n"), "{stdout}");
    let _ = std::fs::remove_dir_all(&ws);
}
