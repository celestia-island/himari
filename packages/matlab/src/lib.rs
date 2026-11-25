#![cfg_attr(not(feature = "std"), no_std)]

//! MATLAB expression subset front end: `src` → (`Vec<Decl>`, `Expr`).

extern crate alloc;
#[cfg(test)]
extern crate std;
#[cfg(test)]
use std::prelude::v1::*;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use himari::{add, div, mul, neg, num, pow, sub, var, Decl, Expr, FrontError};
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "matlab.pest"]
struct MatlabParser;

pub fn parse(src: &str) -> Result<himari::FormulaSet, FrontError> {
    // Reject invisible / control characters first (social-engineering surface), then parse.
    himari::scan_source(src)?;
    // Same guarantee as the LaTeX surface: the whole generated-parser + lowering descent runs on
    // a thread with a 64 MiB stack, because the grammar recurses per nesting level with large
    // frames and the crate builds without pest's `stack-guard`. The pest tree is `Rc`-based and
    // stays inside the thread; only the `FormulaSet` comes back.
    let owned = src.to_string();
    let run = parse_on_big_stack(move || {
        let parsed = MatlabParser::parse(Rule::file, owned.as_str());
        let mut pairs = parsed.map_err(|e| {
            let (line, col) = match e.line_col {
                pest::error::LineColLocation::Pos(p) => p,
                pest::error::LineColLocation::Span(p, _) => p,
            };
            // pest has its own stack limit: when it is hit its raw text is "stack limit reached",
            // which tells the user very little, so translate it into something actionable here.
            let why = format!("{}", e.variant);
            if why.contains("stack limit") {
                FrontError::new(
                    line,
                    col,
                    himari::Error::NestingTooDeep {
                        got: "pest parser stack".to_string(),
                        limit: himari::MAX_DEPTH.to_string(),
                    },
                )
            } else {
                FrontError::new(line, col, himari::Error::SyntaxError(why))
            }
        })?;
        let file = pairs.next().ok_or_else(|| {
            FrontError::new(
                0,
                0,
                himari::Error::SyntaxError("grammar yielded no file node".into()),
            )
        })?;
        let mut set = himari::FormulaSet::default();
        for p in file.into_inner() {
            match p.as_rule() {
                Rule::decl_block => {
                    for d in p.into_inner() {
                        set.decls.push(lower_decl(d)?);
                    }
                }
                Rule::formula => set.items.push(lower_formula(p)?),
                Rule::use_stmt => {
                    let path = p
                        .into_inner()
                        .next()
                        .ok_or_else(|| {
                            FrontError::plain(himari::Error::SyntaxError(
                                "`use` is missing its path".into(),
                            ))
                        })?
                        .as_str()
                        .to_string();
                    set.uses.push(path);
                }
                _ => {}
            }
        }
        if set.items.is_empty() {
            return Err(FrontError::plain(himari::Error::EmptyFormula));
        }
        // Depth guard: every downstream stage is recursive, so a stack overflow is unacceptable.
        himari::check_set_depth(&set)?;
        // A declared name colliding with a builtin function / reserved constant silently changes
        // semantics.
        himari::check_decl_names(&set)?;
        Ok(set)
    });
    finish_descent(run)
}

/// Maps the thread outcome onto the public contract: a thread that could not be spawned, or a
/// parser that panicked inside it, is reported as the catalogued `syntax-error` — never as an
/// abort of the host process (the LSP and the proc-macro host are long-lived).
fn finish_descent<T>(run: StackRun<Result<T, FrontError>>) -> Result<T, FrontError> {
    match run {
        StackRun::Ok(result) => result,
        StackRun::Failed(text) => Err(FrontError::new(0, 0, himari::Error::SyntaxError(text))),
    }
}

/// Runs the generated parser on a dedicated thread with a guaranteed stack (see `parse`).
/// Outcome of running the descent on the big-stack thread: `T` on success, or a rendered
/// failure if the thread could not be spawned or the parser panicked inside it. A parse must
/// never abort the caller because of a *thread* problem.
enum StackRun<T> {
    Ok(T),
    // Only constructed on the std path (thread spawn/join failures); bare metal runs inline.
    #[cfg_attr(not(feature = "std"), allow(dead_code))]
    Failed(String),
}

#[cfg(feature = "std")]
fn parse_on_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> StackRun<T> {
    const PARSE_STACK: usize = 64 * 1024 * 1024;
    match std::thread::Builder::new().stack_size(PARSE_STACK).spawn(f) {
        Ok(handle) => match handle.join() {
            Ok(value) => StackRun::Ok(value),
            // A panic inside the parser is a bug, but it must surface as a diagnostic, not abort
            // the host process (the LSP and the proc-macro host are long-lived).
            Err(payload) => StackRun::Failed(panic_message(&payload)),
        },
        Err(e) => StackRun::Failed(format!("could not spawn the parser thread: {e}")),
    }
}

#[cfg(feature = "std")]
fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "parser panicked".to_string()
    }
}

#[cfg(not(feature = "std"))]
fn parse_on_big_stack<T, F>(f: F) -> StackRun<T>
where
    F: FnOnce() -> T,
{
    // Bare-metal targets have no threads (and formulas small enough for the caller's stack);
    // the pest stack risk is documented and bounded by `MAX_PARSE_NESTING`.
    StackRun::Ok(f())
}

/// A unicode / named operator call → an IR operator node.
fn op_call(name: String, pow2: bool, args: Vec<Expr>) -> Expr {
    let first = || args.first().cloned().unwrap_or(num(0.0));
    match (name.as_str(), pow2) {
        ("\u{2202}", _) => himari::deriv(
            first(),
            match args.get(1) {
                Some(Expr::Var(v)) => v.clone(),
                _ => "__missing__".to_string(),
            },
        ),
        ("\u{2207}", false) | ("grad", false) => himari::grad(first()),
        ("\u{2207}", true) | ("\u{0394}", _) | ("lap", _) => himari::laplacian(first()),
        ("\u{2211}", _) | ("sum", _) => himari::sum(
            match args.first() {
                Some(Expr::Var(v)) => v.clone(),
                _ => "i".to_string(),
            },
            args.get(1).cloned().unwrap_or(num(0.0)),
            args.get(2).cloned().unwrap_or(num(0.0)),
            args.get(3).cloned().unwrap_or(num(0.0)),
        ),
        ("\u{222b}", _) | ("intg", _) => himari::integral(
            match args.first() {
                Some(Expr::Var(v)) => v.clone(),
                _ => "x".to_string(),
            },
            args.get(1).cloned().unwrap_or(num(0.0)),
            args.get(2).cloned().unwrap_or(num(0.0)),
            args.get(3).cloned().unwrap_or(num(0.0)),
        ),
        ("\u{221a}", _) | ("sqrt", _) => Expr::Call("sqrt".into(), vec![first()]),
        ("ddt", _) => himari::deriv(first(), "t".to_string()),
        _ => Expr::Call(name, args),
    }
}

/// A matrix containing dots → `MatPattern`; otherwise it folds into a concrete `Mat`.
fn mat_from_cells(rows: Vec<Vec<himari::PatternCell>>) -> Expr {
    let has_dots = rows
        .iter()
        .flatten()
        .any(|c| matches!(c, himari::PatternCell::Dots));
    if has_dots {
        Expr::MatPattern(himari::MatPattern { rows })
    } else {
        Expr::Mat(
            rows.into_iter()
                .map(|r| {
                    r.into_iter()
                        .map(|c| match c {
                            himari::PatternCell::El(e) => e,
                            himari::PatternCell::Dots => unreachable!(),
                        })
                        .collect()
                })
                .collect(),
        )
    }
}

fn lower_decl(p: Pair<Rule>) -> Result<Decl, FrontError> {
    let mut it = p.into_inner();
    let name = it
        .next()
        .ok_or_else(|| {
            FrontError::plain(himari::Error::SyntaxError("malformed declaration".into()))
        })?
        .as_str()
        .to_string();
    let quantity = it
        .next()
        .ok_or_else(|| {
            FrontError::plain(himari::Error::SyntaxError("malformed declaration".into()))
        })?
        .as_str()
        .to_string();
    let unit = it.next().map(|u| {
        u.into_inner()
            .next()
            .map(|x| x.as_str().trim().to_string())
            .unwrap_or_default()
    });
    Ok(Decl {
        name,
        quantity,
        unit,
    })
}

fn lower_formula(p: Pair<Rule>) -> Result<himari::FormulaItem, FrontError> {
    // Start line of the formula (1-based): a pest span is a byte offset, converted here by
    // counting `\n`.
    let line = p.as_span().start_pos().line_col().0;
    let inner = p
        .into_inner()
        .next()
        .ok_or_else(|| FrontError::plain(himari::Error::EmptyFormula))?;
    match inner.as_rule() {
        Rule::assign => {
            let mut it = inner.into_inner();
            let lhs = it
                .next()
                .ok_or_else(|| {
                    FrontError::plain(himari::Error::SyntaxError("malformed assignment".into()))
                })?
                .as_str()
                .to_string();
            let rhs = lower_expr(it.next().ok_or_else(|| {
                FrontError::plain(himari::Error::SyntaxError(
                    "assignment is missing its right-hand side".into(),
                ))
            })?)?;
            Ok(himari::FormulaItem {
                name: Some(lhs),
                expr: rhs,
                line,
            })
        }
        Rule::expr => Ok(himari::FormulaItem {
            name: None,
            expr: lower_expr(inner)?,
            line,
        }),
        other => Err(FrontError::plain(himari::Error::SyntaxError(format!(
            "unexpected formula rule {other:?}"
        )))),
    }
}

fn lower_expr(p: Pair<Rule>) -> Result<Expr, FrontError> {
    let mut it = p.into_inner();
    let mut acc = lower_term(it.next().ok_or_else(|| {
        FrontError::plain(himari::Error::SyntaxError("empty expression".into()))
    })?)?;
    while let Some(op) = it.next() {
        let minus = op.as_str() == "-";
        let rhs = lower_term(it.next().ok_or_else(|| {
            FrontError::plain(himari::Error::SyntaxError(
                "`+`/`-` is missing its right-hand term".into(),
            ))
        })?)?;
        acc = if minus { sub(acc, rhs) } else { add(acc, rhs) };
    }
    Ok(acc)
}

fn lower_term(p: Pair<Rule>) -> Result<Expr, FrontError> {
    let mut it = p.into_inner();
    let mut acc =
        lower_factor(it.next().ok_or_else(|| {
            FrontError::plain(himari::Error::SyntaxError("empty product".into()))
        })?)?;
    while let Some(op) = it.next() {
        let is_div = op.as_str() == "/";
        let rhs = lower_factor(it.next().ok_or_else(|| {
            FrontError::plain(himari::Error::SyntaxError(
                "`*`/`/` is missing its right-hand factor".into(),
            ))
        })?)?;
        acc = if is_div { div(acc, rhs) } else { mul(acc, rhs) };
    }
    Ok(acc)
}

fn lower_factor(p: Pair<Rule>) -> Result<Expr, FrontError> {
    let mut it = p.into_inner();
    let first = it
        .next()
        .ok_or_else(|| FrontError::plain(himari::Error::SyntaxError("empty factor".into())))?;
    match first.as_rule() {
        Rule::add_op => {
            let inner = lower_power(it.next().ok_or_else(|| {
                FrontError::plain(himari::Error::SyntaxError(
                    "unary `-` is missing its operand".into(),
                ))
            })?)?;
            // `-x'`: the unary minus branch must also consume the transpose suffix — it used to
            // be dropped silently, so the canonical write-back of `Neg(Transpose(x))` parsed back
            // as `Neg(x)` (semantic drift).
            let inner = match it.next() {
                None => inner,
                Some(t) if t.as_rule() == Rule::transpose => Expr::Transpose(Box::new(inner)),
                Some(other) => {
                    return Err(FrontError::plain(himari::Error::SyntaxError(format!(
                        "unexpected factor suffix {:?}",
                        other.as_rule()
                    ))))
                }
            };
            Ok(if first.as_str() == "-" {
                neg(inner)
            } else {
                inner
            })
        }
        Rule::power => {
            let inner = lower_power(first)?;
            // `A'` / `A′`: transpose suffix
            Ok(match it.next() {
                None => inner,
                Some(t) if t.as_rule() == Rule::transpose => Expr::Transpose(Box::new(inner)),
                Some(other) => {
                    return Err(FrontError::plain(himari::Error::SyntaxError(format!(
                        "unexpected factor suffix {:?}",
                        other.as_rule()
                    ))))
                }
            })
        }
        other => Err(FrontError::plain(himari::Error::SyntaxError(format!(
            "unexpected factor rule {other:?}"
        )))),
    }
}

fn lower_power(p: Pair<Rule>) -> Result<Expr, FrontError> {
    // `^` is a literal and produces no pair — the children are all atoms, count = 1 + number of
    // exponents.
    let mut it = p.into_inner();
    let mut acc = lower_atom(
        it.next()
            .ok_or_else(|| FrontError::plain(himari::Error::SyntaxError("empty power".into())))?,
    )?;
    let mut exps: Vec<Expr> = Vec::new();
    for rhs in it {
        exps.push(lower_atom(rhs)?);
    }
    if exps.len() == 1 {
        if let Expr::Var(n) = &exps[0] {
            if n == "T" {
                return Ok(Expr::Transpose(Box::new(acc)));
            }
        }
    }
    for rhs in exps {
        acc = pow(acc, rhs);
    }
    Ok(acc)
}

fn lower_atom(p: Pair<Rule>) -> Result<Expr, FrontError> {
    // `atom` is a wrapper rule; what really lowers is the single rule inside it
    let p = match p.as_rule() {
        Rule::atom => p
            .into_inner()
            .next()
            .ok_or_else(|| FrontError::plain(himari::Error::SyntaxError("empty atom".into())))?,
        _ => p,
    };
    match p.as_rule() {
        Rule::matrix => {
            let mut rows: Vec<Vec<himari::PatternCell>> = Vec::new();
            for r in p.into_inner() {
                if r.as_rule() != Rule::row {
                    continue; // row_sep (semicolon/newline) is not a data row
                }
                let mut cells = Vec::new();
                for cell in r.into_inner() {
                    // cell is the mcell **wrapper**: only the inner pair is the dots or expr
                    let inner = cell.clone().into_inner().next().ok_or_else(|| {
                        FrontError::plain(himari::Error::SyntaxError("empty matrix cell".into()))
                    })?;
                    cells.push(match inner.as_rule() {
                        Rule::dots => himari::PatternCell::Dots,
                        _ => himari::PatternCell::El(lower_expr(inner)?),
                    });
                }
                rows.push(cells);
            }
            Ok(mat_from_cells(rows))
        }
        Rule::op_call => {
            let pairs: Vec<_> = p.into_inner().collect();
            let mut idx = 0;
            let name = pairs
                .get(idx)
                .ok_or_else(|| {
                    FrontError::plain(himari::Error::SyntaxError(
                        "operator is missing its name".into(),
                    ))
                })?
                .as_str()
                .to_string();
            idx += 1;
            // op_pow is an optional prefix: consumed only when the next pair is exactly it (it
            // must not be taken blindly)
            let pow2 = matches!(pairs.get(idx).map(|x| x.as_rule()), Some(Rule::op_pow));
            if pow2 {
                idx += 1;
            }
            let mut args = Vec::new();
            for a in &pairs[idx..] {
                args.push(lower_expr(a.clone())?);
            }
            Ok(op_call(name, pow2, args))
        }
        Rule::call => {
            let mut it = p.into_inner();
            let name = it
                .next()
                .ok_or_else(|| {
                    FrontError::plain(himari::Error::SyntaxError(
                        "function is missing its name".into(),
                    ))
                })?
                .as_str()
                .to_string();
            let mut args = Vec::new();
            for a in it {
                args.push(lower_expr(a)?);
            }
            // Unknown function name + all arguments bare symbols/integers → indexed symbol
            // a(i, j), b(1) (tensor atoms; numeric indices are the ingredients of a pattern
            // matrix index sequence, such as b(1) b(2) b(3))
            // The registry (core) is the single source of truth: only the one bit "is this a known
            // function" is borrowed here.
            const KNOWN: &[&str] = &[
                "exp", "ln", "log", "sin", "cos", "tan", "sinh", "cosh", "tanh", "sqrt", "abs",
                "arg", "conj", "re", "im", "min", "max", "clamp", "det", "trace", "adjoint", "inv",
                "solve", "dot", "norm",
            ];
            let idxable = |a: &Expr| match a {
                Expr::Var(v) => Some(v.clone()),
                Expr::Num(n) if himari::math_fract(*n) == 0.0 => Some(format!("{n:.0}")),
                _ => None,
            };
            if !KNOWN.contains(&name.as_str()) && !args.is_empty() {
                let all: Option<Vec<String>> = args.iter().map(idxable).collect();
                if let Some(idx) = all {
                    return Ok(himari::indexed(name, idx));
                }
            }
            // Arity is defined by the core registry (`check_calls` validates it uniformly at the
            // end of parse); here only "unknown function + several arguments" is blocked —
            // silently taking the first argument would compute a wrong value.
            if himari::func_arity(&name).is_none() {
                // Catalogued variant; the payload is the **bare name** (the template renders it
                // as ``unknown function `{name}```), so LSP can extract it for the
                // closest-registry-name quick-fix.
                // Empty = no near match; the template renders "did you mean" only for a real one.
                let suggestion = himari::closest_func(&name).unwrap_or_default().to_string();
                return Err(FrontError::new(
                    0,
                    0,
                    himari::Error::UnknownFunction { name, suggestion },
                ));
            }
            Ok(Expr::Call(name, args))
        }
        Rule::ident => Ok(var(p.as_str())),
        Rule::number => {
            let s = p.as_str();
            s.parse::<f64>().map(num).map_err(|_| {
                FrontError::plain(himari::Error::SyntaxError(format!(
                    "malformed number `{s}`"
                )))
            })
        }
        Rule::group => {
            let mut it = p.into_inner();
            lower_expr(it.next().ok_or_else(|| {
                FrontError::plain(himari::Error::SyntaxError("empty parentheses".into()))
            })?)
        }
        other => Err(FrontError::plain(himari::Error::SyntaxError(format!(
            "unexpected atom rule {other:?}"
        )))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use himari::eval;
    use himari::HashMap;

    const BV: &str = r#"decl { i0 : current_density [A/m^2]; eta : overpotential [V]; alpha : dimensionless; n : dimensionless; }
        i = i0 * (exp(alpha*n*F*eta/(R*T)) - exp(-(1-alpha)*n*F*eta/(R*T)))"#;

    #[test]
    fn parses_butler_volmer() {
        let decls_set = parse(BV).expect("应可解析");
        let (decls, e) = decls_set.into_single();
        assert_eq!(decls.len(), 4);
        let mut env = HashMap::new();
        for (k, v) in [
            ("i0", 1.0),
            ("eta", 0.0),
            ("alpha", 0.5),
            ("n", 1.0),
            ("F", 96485.0),
            ("R", 8.314),
            ("T", 298.15),
        ] {
            env.insert(k.to_string(), v);
        }
        assert!(eval(&e, &env).unwrap().abs() < 1e-12);
    }

    #[test]
    fn rejects_implicit_multiplication() {
        // In MATLAB semantics `x y` is not multiplication
        assert!(parse(r"decl { x : a; y : a; } x y").is_err());
    }

    #[test]
    fn rejects_elementwise_operators() {
        // `.*` has different semantics from this subset and must be rejected explicitly instead
        // of being silently treated as `*`
        let err = parse(r"decl { x : a; y : a; } x .* y").unwrap_err();
        // Assert the stable code: the sentence is per-locale.
        assert_eq!(err.code.code(), "syntax-error", "{}", err.msg);
    }

    #[test]
    fn multi_letter_idents_are_natural() {
        let ___s = parse(r"decl { alpha : a; } alpha + 1").unwrap();
        let (_, e) = ___s.into_single();
        assert_eq!(e, add(var("alpha"), num(1.0)));
    }

    #[test]
    fn indexed_tridiag_sequence() {
        // Diagonal b(1) b(2) b(3) b(4): an index sequence → TridiagVec (vector argument)
        let src = "decl { b : d; c : d; a : d; } [b(1), c, ..., 0; a, b(2), c, ...; ..., a, b(3), c; 0, ..., a, b(4)]";
        let set = parse(src).unwrap();
        let (_, e) = set.into_single();
        match &e {
            himari::Expr::MatPattern(p) => match himari::recognize_pattern(p).unwrap() {
                himari::MatrixKind::TridiagVec { name, .. } => {
                    assert_eq!(name, "b");
                }
                other => panic!("期望 TridiagVec，得到 {other:?}"),
            },
            other => panic!("期望模式矩阵，得到 {other:?}"),
        }
    }

    #[test]
    fn indexed_tridiag_letter_wildcard() {
        // The general-diagonal term b_{i} (same letter) is also recognized
        let src = "decl { b : d; c : d; a : d; } [b(i), c, ..., 0; a, b(i), c, ...; ..., a, b(i), c; 0, ..., a, b(i)]";
        let set = parse(src).unwrap();
        let (_, e) = set.into_single();
        match &e {
            himari::Expr::MatPattern(p) => {
                let k = himari::recognize_pattern(p).unwrap();
                assert!(
                    matches!(k, himari::MatrixKind::TridiagVec { .. }),
                    "得到 {k:?}"
                );
            }
            other => panic!("期望模式矩阵，得到 {other:?}"),
        }
    }

    #[test]
    fn pattern_matrix_with_dots() {
        let src = "decl { b : d; c : d; a : d; } [b, c, ..., 0; a, b, c, ...; ..., a, b, c; 0, ..., a, b]";
        let ___s = parse(src).unwrap();
        let (_, e) = ___s.into_single();
        match e {
            Expr::MatPattern(p) => {
                assert!(matches!(
                    himari::recognize_pattern(&p).unwrap(),
                    himari::MatrixKind::Tridiag { .. }
                ));
            }
            other => panic!("期望模式矩阵，得到 {other:?}"),
        }
    }

    #[test]
    fn transpose_suffix_and_superscript() {
        let ___s = parse("decl { a : d; b : d; } [1, a; b, 2]'").unwrap();
        let (_, e1) = ___s.into_single();
        assert!(matches!(e1, Expr::Transpose(_)));
        let ___s = parse("decl { x : d; } x^T").unwrap();
        let (_, e2) = ___s.into_single();
        assert_eq!(e2, himari::transpose(var("x")));
    }

    #[test]
    fn federated_formulas_reference_earlier() {
        // Federated: mech references accel; separated by a newline (no semicolon needed)
        let set = parse("decl { F : f [N]; m : m [kg]; r : rad [m]; }\naccel = F / m\nmech = ( accel * r )^(1/2)")
            .unwrap();
        assert_eq!(set.items.len(), 2);
        assert_eq!(set.items[0].name.as_deref(), Some("accel"));
        // Federation expansion: accel has already been substituted into mech's expression (no
        // accel symbol remains)
        let fed = himari::federate_inline(&set).unwrap();
        let names = himari::vars_of(&fed[1].expr);
        assert!(!names.contains(&"accel".to_string()), "{names:?}");
        assert!(names.contains(&"F".to_string()) && names.contains(&"m".to_string()));
    }

    #[test]
    fn federated_dims_propagate() {
        let set = parse("decl { F : f [N]; m : m [kg]; r : rad [m]; }\naccel = F / m\nmech = ( accel * r )^(1/2)")
            .unwrap();
        let dims = himari::dims_of_set(&set).unwrap();
        // accel's dimension = m/s², merged in as a declared symbol
        assert_eq!(
            dims.get("accel").unwrap(),
            &himari::parse_unit("m/s^2").unwrap()
        );
        // mech referencing accel is legal (if dimension propagation were missing, this would
        // report an undeclared symbol)
        let (_, mech) = set.clone().into_single();
        assert!(himari::dim_check(&himari::simplify(&mech), &dims).is_ok());
    }

    #[test]
    fn unicode_operators_parse() {
        // ∂(u, x): symbolic differentiation
        let set = parse("decl { u : c [J]; x : l [m]; }\n∂(u, x)").unwrap();
        let dims = himari::dims_of_set(&set).unwrap();
        let (_, e) = set.into_single();
        assert!(matches!(e, himari::Expr::Deriv(_, _)));
        // dimension = J/m
        assert_eq!(
            himari::dim_check(&e, &dims).unwrap(),
            himari::parse_unit("J/m").unwrap()
        );
        // ∇²(u) / Δ(u): Laplacian
        let set2 = parse("decl { u : c [J]; }\n∇^2(u)").unwrap();
        let (_, e2) = set2.into_single();
        assert!(matches!(e2, himari::Expr::Laplacian(_)));
        // Σ is evaluable: Σ(i, 1, 4, i^2) = 1+4+9+16 = 30
        let set3 = parse("decl { i : d; }\n∑(i, 1, 4, i^2)").unwrap();
        let (_, e3) = set3.into_single();
        let env = himari::HashMap::new();
        assert_eq!(himari::eval(&e3, &env).unwrap(), 30.0);
        // Integral: ∫(x, 0, 1, x^2) ≈ 1/3
        let set4 = parse("decl { x : d; }\n∫(x, 0, 1, x^2)").unwrap();
        let (_, e4) = set4.into_single();
        let v = himari::eval(&e4, &env).unwrap();
        assert!((v - 1.0 / 3.0).abs() < 1e-9, "{v}");
    }

    #[test]
    fn canonical_roundtrip_covers_every_node_type() {
        use himari::{key, simplify, to_matex, Expr};
        // Regression: canonical used to write an indexed symbol as `a[i,k]` (unreadable), while
        // the old version of this gate only made `contains` substring assertions and never
        // re-parsed — so the defect slipped through every gate.
        // Now: 22 variants (including MatPattern/Grad/Laplacian/Integral), canonicalized
        // → **re-parsed** → same key.
        let decls = "decl { x : d; y : d; i : d; m : d; } ";
        let v = himari::var;
        let n = himari::num;
        let cases: Vec<Expr> = vec![
            himari::add(v("x"), himari::mul(n(2.0), v("y"))),
            himari::sub(v("x"), v("y")),
            himari::mul(v("x"), v("y")),
            himari::div(v("x"), v("y")),
            himari::pow(v("x"), n(3.0)),
            himari::neg(v("x")),
            Expr::Call("exp".into(), vec![v("x")]),
            Expr::Call("sqrt".into(), vec![v("x")]),
            Expr::Call("abs".into(), vec![v("x")]),
            Expr::Call("min".into(), vec![v("x"), v("y")]),
            Expr::Call("clamp".into(), vec![v("x"), n(0.0), n(1.0)]),
            Expr::Call(
                "dot".into(),
                vec![
                    Expr::Mat(vec![vec![v("x"), v("y")]]),
                    Expr::Mat(vec![vec![v("y"), v("x")]]),
                ],
            ),
            Expr::Mat(vec![vec![n(1.0), v("x")], vec![v("y"), n(2.0)]]),
            himari::transpose(v("x")),
            himari::indexed("m", vec!["i".into()]),
            himari::indexed("m", vec!["i".into(), "y".into()]),
            Expr::Sum {
                var: "i".into(),
                lower: Box::new(n(1.0)),
                upper: Box::new(n(3.0)),
                body: Box::new(v("i")),
            },
            Expr::Deriv(Box::new(v("y")), "x".into()),
            himari::grad(v("x")),
            himari::laplacian(v("x")),
            Expr::Integral {
                var: "x".into(),
                lower: Box::new(n(0.0)),
                upper: Box::new(n(1.0)),
                body: Box::new(pow(v("x"), n(2.0))),
            },
            // The single-variable form of solve (the second argument is a bare variable name)
            Expr::Call(
                "solve".into(),
                vec![himari::sub(himari::mul(v("x"), n(2.0)), n(6.0)), v("x")],
            ),
            // The coefficient-vector form (the second argument must be a bare variable name)
            Expr::Call(
                "poly_coeffs".into(),
                vec![
                    himari::add(
                        himari::sub(
                            himari::mul(n(2.0), himari::pow(v("x"), n(2.0))),
                            himari::mul(n(3.0), v("x")),
                        ),
                        n(1.0),
                    ),
                    v("x"),
                ],
            ),
            // The polynomial-roots form (the second argument must be a bare variable name)
            Expr::Call(
                "roots".into(),
                vec![
                    himari::add(
                        himari::sub(himari::pow(v("x"), n(2.0)), himari::mul(n(5.0), v("x"))),
                        n(6.0),
                    ),
                    v("x"),
                ],
            ),
            // The equation-system form of solve (two lists: equation list + unknown list)
            Expr::Call(
                "solve".into(),
                vec![
                    Expr::Mat(vec![
                        vec![himari::sub(himari::add(v("x"), v("y")), n(3.0))],
                        vec![himari::sub(v("x"), v("y"))],
                    ]),
                    Expr::Mat(vec![vec![v("x")], vec![v("y")]]),
                ],
            ),
            // A pattern matrix with dots (canonical keeps `...`, so the round trip is still a
            // pattern matrix)
            Expr::MatPattern(himari::MatPattern {
                rows: vec![
                    vec![himari::PatternCell::El(n(1.0)), himari::PatternCell::Dots],
                    vec![himari::PatternCell::Dots, himari::PatternCell::El(n(1.0))],
                ],
            }),
        ];
        for e in &cases {
            let canonical = to_matex(e);
            let back = parse(&format!("{decls}\n{canonical}"))
                .unwrap_or_else(|err| panic!("canonical 不可解析：{canonical}（{err:?}）"));
            let reparsed = back.single().unwrap();
            assert_eq!(
                key(&simplify(e)),
                key(&simplify(reparsed)),
                "canonical 改写了语义：{canonical}"
            );
        }
    }

    #[test]
    fn indexed_symbols_via_unknown_call() {
        // Unknown function name + symbolic arguments → indexed symbol
        // (`j` is the reserved imaginary unit and cannot be declared; indices conventionally use
        // i/k)
        let set = parse("decl { a : m [kg]; i : d; k : d; }\na(i, k)").unwrap();
        let (_, e) = set.into_single();
        match e {
            himari::Expr::Indexed { name, indices } => {
                assert_eq!(name, "a");
                assert_eq!(indices, vec!["i".to_string(), "k".to_string()]);
            }
            other => panic!("期望 Indexed，得到 {other:?}"),
        }
        // Known functions are unaffected
        let (_, f) = parse("decl { x : d; }\nexp(x)").unwrap().into_single();
        assert!(matches!(f, himari::Expr::Call(..)));
    }

    #[test]
    fn bare_juxtaposition_still_rejected() {
        // Pure-space juxtaposition (no newline) must still error on the matlab surface — the
        // federation separator is a semicolon or a **newline**
        assert!(parse("decl { x : a; y : a; } x y").is_err());
    }

    #[test]
    fn matrix_columns_free_spacing() {
        // Any number of spaces between columns, alignment not required — the three spellings give
        // the same tree
        let a = parse("decl { b : d; c : d; a : d; } [b, c, ..., 0; a, b, c, ...; ..., a, b, c; 0, ..., a, b]").unwrap();
        let b = parse("decl { b : d; c : d; a : d; } [b,   c,     ..., 0;\n  a, b, c, ...;\n ..., a, b, c;\n 0, ..., a, b]").unwrap();
        let c =
            parse("decl { b : d; c : d; a : d; } [b c ... 0\n a b c ...\n ... a b c\n 0 ... a b]")
                .unwrap();
        let (_, ea) = a.into_single();
        let (_, eb) = b.into_single();
        let (_, ec) = c.into_single();
        let ka = himari::key(&ea);
        let kb = himari::key(&eb);
        let kc = himari::key(&ec);
        assert_eq!(ka, kb, "多空格不齐等价");
        assert_eq!(
            ka, kc,
            "空格分隔 + 换行分行等价\n  分号版: {ka}\n  换行版: {kc}"
        );
    }

    #[test]
    fn matrix_newline_rows_without_semicolons() {
        // Pure newline row separation (zero semicolons) + vertical dots positionally
        let set = parse("decl { b : d; c : d; a : d; }\n[2*b, -b, ..., 0\n -b, 2*b, -b, ...\n ..., -b, 2*b, -b\n 0, ..., -b, 2*b]").unwrap();
        let (_, e) = set.into_single();
        match &e {
            himari::Expr::MatPattern(p) => {
                let kind = himari::recognize_pattern(p);
                assert!(
                    matches!(kind, Ok(himari::MatrixKind::Tridiag { .. })),
                    "识别失败：{kind:?}（rows = {rows:?}）",
                    rows = p.rows
                );
            }
            other => panic!("期望模式矩阵，得到 {other:?}"),
        }
    }

    #[test]
    fn three_surfaces_same_tridiag() {
        // The paper form (glyph dots) and ASCII `...` must lower to the same tree
        let paper =
            "decl { b : d; c : d; a : d; } [b, c, ⋯, 0; a, b, c, ⋯; ⋯, a, b, c; 0, ⋯, a, b]";
        let ascii = "decl { b : d; c : d; a : d; } [b, c, ..., 0; a, b, c, ...; ..., a, b, c; 0, ..., a, b]";
        let ___s = parse(paper).unwrap();
        let (_, p1) = ___s.into_single();
        let ___s = parse(ascii).unwrap();
        let (_, p2) = ___s.into_single();
        assert_eq!(himari::key(&p1), himari::key(&p2));
    }

    #[test]
    fn matrix_literal() {
        let ___s = parse(r"decl { a : a; b : a; } [1, a; b, 2]").unwrap();
        let (_, e) = ___s.into_single();
        match e {
            Expr::Mat(rows) => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 2);
            }
            other => panic!("期望矩阵，得到 {other:?}"),
        }
    }
}

#[cfg(test)]
mod error_code_tests {
    /// The catalogue variant must survive the front-end: `code()` is what LSP quick-fixes and
    /// tests match on.
    #[test]
    fn unknown_function_reports_the_catalogued_code_with_a_bare_name_payload() {
        let err = crate::parse("decl { x : d; }\ny = expp(x * 2)\n").unwrap_err();
        assert_eq!(err.code.code(), "unknown-function", "{}", err.msg);
        let himari::Error::UnknownFunction { name, suggestion } = &*err.code else {
            panic!("variant must be UnknownFunction, got {:?}", err.code)
        };
        assert_eq!(name, "expp", "payload must be the bare name: {name}");
        assert_eq!(
            suggestion, "exp",
            "the closest-registry-name hint must survive: {suggestion}"
        );
    }
}

#[cfg(test)]
mod empty_formula_tests {
    /// `decl` without any formula parses fine but yields zero items — that is the reachable
    /// `empty-formula` path (a truly empty source is rejected earlier by the grammar).
    #[test]
    fn a_set_with_declarations_but_no_formula_reports_the_catalogued_code() {
        // The grammar itself refuses a formula-less source (`syntax-error`), so the
        // `empty-formula` guard covers sets built by hand (federation, loaders, tests).
        let set = himari::FormulaSet {
            decls: vec![himari::Decl {
                name: "x".into(),
                quantity: "length".into(),
                unit: Some("m".into()),
            }],
            items: vec![],
            uses: vec![],
        };
        let err = himari::check_set_depth(&set).unwrap_err();
        assert_eq!(err.code.code(), "empty-formula", "{}", err.msg);
    }
}

#[cfg(all(test, feature = "std"))]
mod parser_thread_tests {
    use super::*;

    /// A panic inside the parser thread must surface as the catalogued `syntax-error`, never as
    /// an abort of the host process (the LSP and the proc-macro host are long-lived). This
    /// asserts the PUBLIC contract through `finish_descent`, not the raw pass-through text.
    #[test]
    fn a_parser_panic_becomes_a_syntax_error_not_an_abort() {
        let run = super::parse_on_big_stack(|| -> Result<himari::FormulaSet, FrontError> {
            panic!("simulated parser bug")
        });
        let err = finish_descent(run).expect_err("a panicking closure must not report Ok");
        assert_eq!(err.code.code(), "syntax-error", "{}", err.msg);
        // Pin the typed payload exactly: the panic text must be passed through verbatim for
        // diagnosis.
        match &*err.code {
            himari::Error::SyntaxError(p) => assert_eq!(p, "simulated parser bug"),
            other => panic!("expected SyntaxError, got {other:?}"),
        }
    }
}
