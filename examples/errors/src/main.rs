//! The error catalogue: what a rejection looks like as a typed value, and how the SAME error
//! renders in nine languages while its code stays identical.
//!
//! "Rejection is a feature": every diagnostic the library produces is a variant of `himari::Error`
//! with a stable machine-readable code, so tooling can branch on the failure without parsing
//! sentences.
//!
//! Run: `cargo run -p himari-example-errors`

use himari::{dim_check, eval, i18n::Locale, parse_unit, Expr};
use std::collections::HashMap;

fn show(e: &himari::Error, label: &str) {
    println!("{label}");
    println!("  code     : {}", e.code());
    for locale in [Locale::En, Locale::ZhHans, Locale::Ja] {
        println!("  {:<8}: {}", format!("{locale:?}"), e.render(locale));
    }
}

fn main() {
    // 1) An undeclared symbol in the evaluation lane.
    let env: HashMap<String, f64> = HashMap::new();
    let err = eval(&Expr::Var("altitude".into()), &env).unwrap_err();
    show(&err, "undeclared symbol (eval):");

    // 2) A dimension mismatch with the operator named: `m + s` cannot be added.
    let mut dims = HashMap::new();
    dims.insert("m".to_string(), parse_unit("m").unwrap());
    dims.insert("s".to_string(), parse_unit("s").unwrap());
    let nonsense = Expr::Add(
        Box::new(Expr::Var("m".into())),
        Box::new(Expr::Var("s".into())),
    );
    let err = dim_check(&nonsense, &dims).unwrap_err();
    show(&err, "operator dimension mismatch (m + s):");

    // 3) A unit string that does not parse.
    let err = himari::convert(1.0, "ligthyears", "m").unwrap_err();
    show(&err, "malformed unit (typo'd `lightyears`):");

    // 4) A non-linear equation reaching the solver: x·y is linear in neither unknown.
    let x = Expr::Var("x".into());
    let y = Expr::Var("y".into());
    let nonlinear = Expr::Sub(
        Box::new(Expr::Mul(Box::new(x), Box::new(y))),
        Box::new(Expr::Num(1.0)),
    );
    let err = himari::solve_system(&[nonlinear], &["x".into(), "y".into()]).unwrap_err();
    show(&err, "non-linear system (the wrapper names the equation):");

    // 5) A dimension check against a declared table: v + v² is meaningless.
    let mut dims = HashMap::new();
    dims.insert("v".to_string(), parse_unit("m/s").unwrap());
    let bad = Expr::Add(
        Box::new(Expr::Var("v".into())),
        Box::new(Expr::Pow(
            Box::new(Expr::Var("v".into())),
            Box::new(Expr::Num(2.0)),
        )),
    );
    let err = dim_check(&bad, &dims).unwrap_err();
    show(&err, "dimension check (v + v²):");

    // The contract in one place: the code is the machine key, the locale is presentation.
    println!("\nthe code never changes with the language — branch on codes, render for humans");
}
