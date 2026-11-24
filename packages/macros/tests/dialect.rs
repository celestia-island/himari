//! "The macro is the dialect": whichever macro wraps the formula, the formula is read according
//! to that specification — the macro lane **does not guess**.
//!
//! This group of tests pins down a real silent wrong value: for `si * x` with `si` declared, the
//! old Auto (try matex first, use it if it succeeds) gave `si·x`, whereas the LaTeX semantics are
//! `s·i·x`. Same string of characters, same inputs, the two differ by a factor of several, and
//! Auto reported no error at all.

/// matex: `si` is one identifier (a name), not `s·i`.
fn matex(si: f64, x: f64) -> f64 {
    himari_macros::formula!("decl { si : d; x : d; } si * x")
}

/// LaTeX: `si` is two letters juxtaposed and multiplied, `s·i`.
fn latex(s: f64, i: f64, x: f64) -> f64 {
    himari_macros::formula_latex!("decl { s : d; i : d; x : d; } si * x")
}

#[test]
fn the_macro_name_decides_the_dialect_not_a_guess() {
    // The same string `si * x`: matex reads it as si·x, LaTeX reads it as s·i·x — both are
    // "right", but it must be the one **the caller chose**, not the one picked for them by the
    // macro.
    assert_eq!(matex(100.0, 2.0), 200.0, "formula! = matex：si 是名字");
    assert_eq!(
        latex(3.0, 5.0, 2.0),
        30.0,
        "formula_latex! = LaTeX：si = s·i"
    );
}

/// Dialect-specific spellings work in their own macros.
#[test]
fn each_macro_accepts_its_own_dialects_own_writing() {
    // `\frac{a}{b}` is a LaTeX-only spelling
    fn frac(a: f64, b: f64) -> f64 {
        himari_macros::formula_latex!("decl { a : d; b : d; } \\frac{a}{b}")
    }
    assert!((frac(3.0, 4.0) - 0.75).abs() < 1e-12);
    // `a^b` (power) is a first-class operator in matex; matrices/transposes belong to the file
    // lane and are not accepted inline (that is deliberate)
    fn pow(a: f64, b: f64) -> f64 {
        himari_macros::formula_matlab!("decl { a : d; b : d; } a ^ b")
    }
    assert!((pow(2.0, 10.0) - 1024.0).abs() < 1e-9);
}

/// A LaTeX-only command inside the matex macro must **fail loudly** and must never silently
/// compute a wrong value.
/// (That is compile-time behavior, see the compile-fail note at the end of this file; here we only
/// ensure the matex macro itself is usable.)
#[test]
fn matex_macro_is_usable_standalone() {
    fn q(k: f64, x: f64) -> f64 {
        himari_macros::formula!("decl { k : d; x : d; } k * x + 1")
    }
    assert_eq!(q(2.0, 3.0), 7.0);
}

#[test]
fn multi_atom_units_survive_the_token_lane() {
    // Regression: the token lane renders punctuation with spaces, so `[m^2]`
    // used to reach the unit parser as "m " and every multi-atom unit failed.
    // `decl` only supplies dimensions; the symbols must exist at the call site.
    let s0 = 4.0f64;
    let t = 2.0f64;
    let out = himari_macros::formula_matlab! {
        decl { s0 : area [m^2]; t : time [s]; }
        s0 / t
    };
    assert!((out - 2.0).abs() < 1e-12, "{out}");
}

#[test]
fn runtime_matrix_formulas_are_fallible_not_silently_unwrapped() {
    // Regression (review S1): the runtime lane can fail, so the macro must hand the
    // caller a Result instead of binding it as if it were the value. This test exists
    // to make the contract explicit at the type level.
    let a = vec![vec![2.0f64, 0.0], vec![0.0, 4.0]];
    let b = vec![2.0f64, 8.0];
    let solved: Result<Vec<f64>, himari::runtime::NumericError> = himari_macros::formula_matlab! {
        decl { a : stiffness_matrix [N/m]; b : load_vector [N]; }
        solve(a, b)
    };
    assert_eq!(solved.unwrap(), vec![1.0, 2.0]);
}
