//! Symbolic differentiation on the IR: build an expression, differentiate it with respect to a
//! variable, simplify, and evaluate both the expression and its derivative — no numeric
//! differentiation, no NaN placeholders.
//!
//! Run: `cargo run -p himari-example-symbolic`

use himari::{simplify, Expr};
use std::collections::HashMap;
use std::sync::OnceLock;

/// A small pretty-printer over the public `Expr` shape (the IR is plain data).
fn fmt(e: &Expr) -> String {
    match e {
        Expr::Num(v) => {
            if v.fract() == 0.0 {
                format!("{v:.0}")
            } else {
                format!("{v}")
            }
        }
        Expr::Var(n) => n.clone(),
        Expr::Add(a, b) => format!("({} + {})", fmt(a), fmt(b)),
        Expr::Sub(a, b) => format!("({} - {})", fmt(a), fmt(b)),
        Expr::Mul(a, b) => format!("{}·{}", fmt(a), fmt(b)),
        Expr::Div(a, b) => format!("({})/({})", fmt(a), fmt(b)),
        Expr::Pow(a, b) => format!("{}^{}", fmt(a), fmt(b)),
        Expr::Neg(a) => format!("-{}", fmt(a)),
        Expr::Call(name, args) => format!(
            "{name}({})",
            args.iter().map(fmt).collect::<Vec<_>>().join(", ")
        ),
        _ => "<unsupported in this printer>".into(),
    }
}

fn x() -> &'static Expr {
    static X: OnceLock<Expr> = OnceLock::new();
    X.get_or_init(|| Expr::Var("x".into()))
}

fn main() {
    // f(x) = 3x³ − 2x² + k·x + 7 — a cubic with a free parameter.
    let cubic = Expr::Add(
        Box::new(Expr::Add(
            Box::new(Expr::Add(
                Box::new(Expr::Mul(
                    Box::new(Expr::Num(3.0)),
                    Box::new(Expr::Pow(Box::new(x().clone()), Box::new(Expr::Num(3.0)))),
                )),
                Box::new(Expr::Mul(
                    Box::new(Expr::Num(-2.0)),
                    Box::new(Expr::Pow(Box::new(x().clone()), Box::new(Expr::Num(2.0)))),
                )),
            )),
            Box::new(Expr::Mul(
                Box::new(Expr::Var("k".into())),
                Box::new(x().clone()),
            )),
        )),
        Box::new(Expr::Num(7.0)),
    );
    println!("f(x) = {}", fmt(&cubic));

    // The derivative comes back as another expression (partially simplified by construction).
    let d = simplify(&himari::diff(&cubic, "x"));
    println!("f'(x) = {}", fmt(&d));

    // Evaluate both at a few points with a hand-computed reference.
    let mut env = HashMap::new();
    env.insert("k".to_string(), 1.5);
    for point in [0.0, 1.0, 2.0, -3.0] {
        env.insert("x".to_string(), point);
        let f = himari::eval(&cubic, &env).unwrap();
        let df = himari::eval(&d, &env).unwrap();
        // d/dx = 9x² − 4x + k, checked against the closed form.
        let want = 9.0 * point * point - 4.0 * point + 1.5;
        assert!(
            (df - want).abs() < 1e-12,
            "f'({point}) = {df}, expected {want}"
        );
        println!("  x = {point:>4}: f = {f:>10.4}   f' = {df:>8.4} (closed form {want})");
    }

    // Differentiating the derivative gives the second derivative: 18x − 4.
    let d2 = simplify(&himari::diff(&d, "x"));
    println!("f''(x) = {}", fmt(&d2));

    // Non-differentiable constructs are reported, not silently NaN'd: `nondiffable` names the
    // offending function calls.
    let with_arg = Expr::Call("arg".into(), vec![x().clone()]);
    println!("nondiffable(arg(x)) = {:?}", himari::nondiffable(&with_arg));
}
