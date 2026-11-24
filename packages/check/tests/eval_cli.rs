//! End-to-end CLI tests of `himari-eval`: really spawn the process and really read stdout.
//!
//! They cover argument parsing and output formatting — the parts unit tests cannot reach (the
//! mathematical correctness of `numeric_roots` is tested in core). `--roots` is the only exit for
//! polynomials of degree three and above, so a misspelled argument makes the whole feature
//! unusable.

use std::io::Write;
use std::process::{Command, Stdio};

/// Write a temporary .formula file and return its path (the name carries the pid, so it is safe
/// under parallel tests).
fn write_formula(tag: &str, body: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("himari-eval-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("建临时目录");
    let p = dir.join("t.formula");
    let mut f = std::fs::File::create(&p).expect("写临时公式");
    f.write_all(body.as_bytes()).expect("写临时公式");
    p
}

fn eval(args: &[&std::ffi::OsStr]) -> (String, String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_himari-eval"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("起 himari-eval");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.success(),
    )
}

#[test]
fn roots_mode_solves_cubic_numerically() {
    let p = write_formula(
        "cubic",
        "decl {\n    x : normalized_input [1];\n}\n\
         cubic = x ^ 3 - 6 * x ^ 2 + 11 * x - 6\n",
    );
    let (out, err, ok) = eval(&[std::ffi::OsStr::new("--roots"), p.as_os_str()]);
    assert!(ok, "应成功，stderr = {err}");
    // (x−1)(x−2)(x−3): three real roots, and the degree and the count must be stated
    for want in ["1", "2", "3"] {
        assert!(out.contains(want), "输出缺少根 {want}：{out}");
    }
    assert!(out.contains("3 次"), "应报次数：{out}");
    assert!(out.contains("3 个根"), "应报根的个数：{out}");
    // A real root must not carry imaginary residue (the outward sign of numerical noise being
    // zeroed out)
    assert!(!out.contains("e-3"), "实根带了噪声虚部：{out}");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn roots_mode_reports_complex_pairs() {
    let p = write_formula(
        "cplx",
        "decl {\n    x : normalized_input [1];\n}\nc = x ^ 3 + 1\n",
    );
    let (out, err, ok) = eval(&[std::ffi::OsStr::new("--roots"), p.as_os_str()]);
    assert!(ok, "应成功，stderr = {err}");
    // The roots of x³+1: −1 and 0.5 ± (√3/2)j
    assert!(out.contains("-1"), "缺少实根 −1：{out}");
    assert!(out.contains("j"), "复根应带 j：{out}");
    assert!(out.contains("0.866"), "缺 √3/2 的虚部：{out}");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn roots_mode_needs_var_when_ambiguous_and_honours_flags() {
    // Two declared symbols: without --var it must error and name the candidates
    let p = write_formula(
        "ambiguous",
        "decl {\n    a : s [1];\n    b : s [1];\n}\nq = a * a + a * b\n",
    );
    let (_out, err, ok) = eval(&[std::ffi::OsStr::new("--roots"), p.as_os_str()]);
    assert!(!ok, "多候选时应失败");
    assert!(err.contains("--var"), "应提示用 --var：{err}");
    // With --var given, the roots can be found (a² + b·a is quadratic in a, with b as a
    // parameter)
    let (out, err2, ok2) = eval(&[
        std::ffi::OsStr::new("--roots"),
        p.as_os_str(),
        std::ffi::OsStr::new("--var"),
        std::ffi::OsStr::new("a"),
        std::ffi::OsStr::new("--set"),
        std::ffi::OsStr::new("b=3"),
    ]);
    assert!(ok2, "指定 --var 后应成功，stderr = {err2}");
    // a² + 3a = 0 → a = 0 and a = −3
    assert!(out.contains("2 个根"), "应是两个根：{out}");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn roots_mode_refuses_non_polynomial_with_clear_message() {
    let p = write_formula(
        "nonpoly",
        "decl {\n    x : normalized_input [1];\n}\ns = sqrt(x) + 1\n",
    );
    let (_out, err, ok) = eval(&[std::ffi::OsStr::new("--roots"), p.as_os_str()]);
    assert!(!ok, "非多项式应失败");
    assert!(err.contains("多项式"), "应说明不是多项式：{err}");
    let _ = std::fs::remove_file(&p);
}
