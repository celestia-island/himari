//! Complex triage of the macro lane: `j` is the reserved imaginary unit, **not a free variable at
//! the call site**.
//!
//! Regression background: without triage, `j` was emitted as a bare identifier — if a `j`
//! happened to be in scope (a loop index is all too common) it silently computed a wrong real
//! number, dropping the imaginary part entirely; if not, E0425.

#[test]
fn complex_formula_becomes_tuple() {
    // A same-named `j` is deliberately placed at the call site: before the fix this computed
    // 7 * 2 = 14
    let j = 7.0f64;
    let x = 2.0f64;
    let z: (f64, f64) = himari_macros::formula_latex!(r"decl { x : d; } j * x");
    assert_eq!(z, (0.0, 2.0), "j*x 必须发成复数 (0+2j)，而不是 j=7 的实数");
    assert_eq!(j, 7.0, "调用点的 j 不该被改写");
}

#[test]
fn federated_complex_chain() {
    // Federated: y is complex, and the next formula references it to get the magnitude
    let x = 3.0f64;
    let m = himari_macros::formula_matlab!("decl { x : d; } y = x + j\nabs(y)");
    let want = (x * x + 1.0f64).sqrt();
    assert_eq!(m, (want, 0.0), "|x + j| = √(x²+1)");
}

#[test]
fn real_formula_stays_real() {
    // A formula without j still takes the real lane: the value is a plain f64, not a tuple
    let x = 4.0f64;
    let v: f64 = himari_macros::formula_matlab!("decl { x : d; } x ^ 2 + 1");
    assert_eq!(v, 17.0);
}
