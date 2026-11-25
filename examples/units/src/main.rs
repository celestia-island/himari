//! Unit conversion and the dimension algebra: what `himari::convert` does, what a dimension
//! mismatch looks like as a typed error, and how the same diagnostic renders in another language.
//!
//! Run: `cargo run -p himari-example-units`

use himari::{convert, dim_check, i18n::Locale, parse_unit, Expr};

fn main() {
    // 1) Unit conversion across prefixes and non-SI units.
    println!("1 h   = {} s", convert(1.0, "h", "s").unwrap());
    println!("1 atm = {} Pa", convert(1.0, "atm", "Pa").unwrap());
    println!("1 Mg  = {} kg", convert(1.0, "Mg", "kg").unwrap());

    // 2) Incompatible units are a typed error with a stable code — not a string comparison.
    let err = convert(1.0, "nF", "V").unwrap_err();
    println!("nF -> V      : code={}", err.code());
    println!("  rendered   : {}", err.render(Locale::En));

    // 3) Dimension checking is the public surface for the algebra. `v / s` is fine (a speed
    //    over a time is an acceleration), `v + s` is meaningless, and the offending operator
    //    is named in the error.
    let mut dims = std::collections::HashMap::new();
    dims.insert("v".to_string(), parse_unit("m/s").unwrap());
    dims.insert("s".to_string(), parse_unit("s").unwrap());
    let acceleration = Expr::Div(
        Box::new(Expr::Var("v".into())),
        Box::new(Expr::Var("s".into())),
    );
    let checked = dim_check(&acceleration, &dims).unwrap();
    println!("v / s        = {}", checked.display());
    let nonsense = Expr::Add(
        Box::new(Expr::Var("v".into())),
        Box::new(Expr::Var("s".into())),
    );
    let err = dim_check(&nonsense, &dims).unwrap_err();
    println!("v + s        : code={}", err.code());
    println!("  rendered   : {}", err.render(Locale::En));

    // 4) The same diagnostic rendered in another locale — the code stays identical, the words
    //    change. This is the catalogue's contract.
    println!("v + s (ja)   : {}", err.render(Locale::Ja));

    // 5) Dynamic pressure shape: ρ·v² has the dimension of a pressure (kg·m^-1·s^-2).
    let mut dims2 = std::collections::HashMap::new();
    dims2.insert("rho".to_string(), parse_unit("kg/m^3").unwrap());
    dims2.insert("v".to_string(), parse_unit("m/s").unwrap());
    let q = Expr::Mul(
        Box::new(Expr::Var("rho".into())),
        Box::new(Expr::Pow(
            Box::new(Expr::Var("v".into())),
            Box::new(Expr::Num(2.0)),
        )),
    );
    let checked = dim_check(&q, &dims2).unwrap();
    println!("ρ·v²         = {} (a pressure)", checked.display());

    // ...and `v + v²` is rejected: a speed added to its square is meaningless.
    let bad = Expr::Add(
        Box::new(Expr::Var("v".into())),
        Box::new(Expr::Pow(
            Box::new(Expr::Var("v".into())),
            Box::new(Expr::Num(2.0)),
        )),
    );
    let err = dim_check(&bad, &dims2).unwrap_err();
    println!("v + v²       : code={}", err.code());
    println!("  rendered   : {}", err.render(Locale::En));
}
