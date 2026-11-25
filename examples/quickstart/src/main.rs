//! Quickstart: the shortest possible tour of himari.
//!
//! Three ways to turn a formula into checked Rust — an inline MATLAB-style macro, an inline
//! LaTeX-style macro, and a runtime-evaluated `solve` — plus the same formula through the f64
//! evaluation lane that everything lowers to.
//!
//! Run: `cargo run -p himari-example-quickstart`

use himari::Expr;

fn main() {
    // 1) Inline macro, MATLAB dialect. Declared dimensions are checked at COMPILE time
    //    (`m/s` over `s` gives `m/s`), the decl names bind to the Rust variables in scope,
    //    and the expansion is the finished Rust expression.
    let v0 = 12.0f64;
    let g = 9.80665f64;
    let t = 3.0f64;
    let v = himari_macros::formula_matlab! {
        decl { v0 : speed [m/s]; g : acceleration [m/s^2]; t : time [s]; }
        v0 + g * t
    };
    println!("matlab macro    : v = {v} m/s (12 + 9.80665·3)");
    assert!((v - (12.0 + 9.80665 * 3.0)).abs() < 1e-9);

    // 2) Inline macro, LaTeX dialect: the same physics in paper notation.
    let m = 2.0f64;
    let v = 3.0f64;
    let energy = himari_macros::formula_latex! {
        r"decl { m : mass [kg]; v : speed [m/s]; } \frac{1}{2} \cdot m \cdot v^2"
    };
    println!("latex macro     : E = {energy} J (½·2·3²)");
    assert!((energy - 9.0).abs() < 1e-12);

    // 3) Runtime lane: `solve` returns a Result, for formulas whose shape is only known at
    //    runtime. Here: solve a·x + b = 0 symbolically.
    let a = 2.0f64; // m/s
    let b = 4.0f64; // m
                    // `solve` is an IR-level expansion: for a scalar lane the result is the finished f64
                    // (runtime `solve(A, b)` with matrix arguments is the one that returns a `Result`).
    let x = himari_macros::formula_matlab!(
        r"decl { a : speed [m/s]; b : length [m]; x : time [s]; } solve(a * x + b, x)"
    );
    println!("runtime solve   : a·x + b = 0 (a = {a} m/s, b = {b} m) -> x = {x} s");
    assert!((x - (-b / a)).abs() < 1e-12);

    // 4) The pure evaluation lane underneath everything: an `Expr` tree plus a variable
    //    environment. This is what the macros expand into and what the codegen lanes lower.
    let expr = Expr::Add(Box::new(Expr::Var("x".into())), Box::new(Expr::Num(4.0)));
    let env: std::collections::HashMap<String, f64> =
        [("x".to_string(), 3.0)].into_iter().collect();
    let value = himari::eval(&expr, &env).unwrap();
    println!("eval lane       : x + 4 at x = 3 -> {value}");
    assert_eq!(value, 7.0);
}
