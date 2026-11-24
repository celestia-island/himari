#![cfg_attr(not(feature = "std"), no_std)]

//! LaTeX subset front end: `src` → (`Vec<Decl>`, `Expr`).

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
use himari::{add, mul, neg, num, pow, sub, var, Decl, Expr, FrontError};
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "latex.pest"]
struct LatexParser;

/// Runs the generated parser on a dedicated thread with a guaranteed stack (std builds).
///
/// The pest-generated grammar recurses per nesting level and its frames are large; the crate
/// builds without pest's `stack-guard` (no_std compatibility), so the guard page is the only
/// backstop. A 64 MiB stack is far above any handwritten formula and the thread spawn cost is
/// negligible next to parsing itself.
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

pub fn parse(src: &str) -> Result<himari::FormulaSet, FrontError> {
    // Reject invisible / control characters first (social-engineering surface), then parse.
    himari::scan_source(src)?;
    // The whole descent (generated parser + lowering) runs on a guaranteed stack: the grammar
    // recurses per nesting level with large frames, and the crate builds without pest's
    // `stack-guard` (no_std compatibility). The pest tree itself is `Rc`-based and cannot leave
    // the thread, so everything up to the `FormulaSet` happens inside it. (`src` is cloned into
    // the thread; parsing dwarfs the copy.)
    let owned = src.to_string();
    let run = parse_on_big_stack(move || {
        let parsed = LatexParser::parse(Rule::file, owned.as_str());
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
                        .ok_or_else(|| FrontError::plain("use 缺路径"))?
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
        .ok_or_else(|| FrontError::plain("坏声明"))?
        .as_str()
        .to_string();
    let quantity = it
        .next()
        .ok_or_else(|| FrontError::plain("坏声明"))?
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
                .ok_or_else(|| FrontError::plain("坏赋值"))?
                .as_str()
                .to_string();
            let rhs = lower_expr(it.next().ok_or_else(|| FrontError::plain("赋值缺右式"))?)?;
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
        other => Err(FrontError::plain(format!("未预期公式规则 {other:?}"))),
    }
}

fn lower_expr(p: Pair<Rule>) -> Result<Expr, FrontError> {
    let mut it = p.into_inner();
    let mut acc = lower_term(it.next().ok_or_else(|| FrontError::plain("空表达式"))?)?;
    while let Some(op) = it.next() {
        let minus = is_minus(op.as_str());
        let rhs = lower_term(it.next().ok_or_else(|| FrontError::plain("加减缺右项"))?)?;
        acc = if minus { sub(acc, rhs) } else { add(acc, rhs) };
    }
    Ok(acc)
}

fn lower_term(p: Pair<Rule>) -> Result<Expr, FrontError> {
    let mut it = p.into_inner();
    let mut acc = lower_unary(it.next().ok_or_else(|| FrontError::plain("空乘积"))?)?;
    while let Some(next) = it.next() {
        match next.as_rule() {
            Rule::mul_op => {
                let rhs = lower_unary(it.next().ok_or_else(|| FrontError::plain("乘号缺右项"))?)?;
                // Tell multiplication from division by the operator's **source text**: `/`,
                // `\div` and `⁄` are division, everything else is multiplication.
                // This must be decided before mul — treating `/` as multiplication is a silent
                // wrong value, far worse than a syntax error.
                acc = if is_division(next.as_str()) {
                    himari::div(acc, rhs)
                } else {
                    mul(acc, rhs)
                };
            }
            // Juxtaposition is multiplication: unambiguous only when the identifier system is
            // LaTeX's "single-letter/single-command" one.
            Rule::unary => acc = mul(acc, lower_unary(next)?),
            other => return Err(FrontError::plain(format!("未预期乘积规则 {other:?}"))),
        }
    }
    Ok(acc)
}

/// Source text of the division operators: `/`, `\div`, U+2215 (division slash).
fn is_division(s: &str) -> bool {
    s == "/" || s == "\\div" || s == "\u{2215}"
}

fn is_minus(s: &str) -> bool {
    s == "-" || s == "\u{2212}"
}

/// Unify `\alpha` and `α` into the same ASCII name — the paper layer and the LaTeX layer are
/// indistinguishable in the IR.
fn greek_name(s: &str) -> String {
    let raw = s.strip_prefix('\\').unwrap_or(s);
    let name = match raw {
        "α" => "alpha",
        "β" => "beta",
        "γ" => "gamma",
        "δ" => "delta",
        "ε" => "epsilon",
        "η" => "eta",
        "θ" => "theta",
        "κ" => "kappa",
        "λ" => "lambda",
        "μ" => "mu",
        "ν" => "nu",
        "ξ" => "xi",
        "π" => "pi",
        "ρ" => "rho",
        "σ" => "sigma",
        "τ" => "tau",
        "φ" => "phi",
        "χ" => "chi",
        "ψ" => "psi",
        "ω" => "omega",
        "Δ" => "Delta",
        "Ω" => "Omega",
        "Γ" => "Gamma",
        "Σ" => "Sigma",
        "Π" => "Pi",
        other => other,
    };
    name.to_string()
}

/// Both `brace` and `group` are "outer containers": what really lowers to the IR is the `expr`
/// inside them.
fn lower_brace(p: Pair<Rule>) -> Result<Expr, FrontError> {
    let inner = p
        .into_inner()
        .next()
        .ok_or_else(|| FrontError::plain("空花括号"))?;
    lower_expr(inner)
}

fn lower_unary(p: Pair<Rule>) -> Result<Expr, FrontError> {
    let whole = p.as_str().to_string();
    let rules: Vec<Rule> = p.clone().into_inner().map(|x| x.as_rule()).collect();
    let mut it = p.into_inner();
    let first = it.next().ok_or_else(|| FrontError::plain("空一元式"))?;
    match first.as_rule() {
        Rule::add_op => {
            let pw = it.next().ok_or_else(|| FrontError::plain("负号缺操作数"))?;
            let inner = lower_power(pw)?;
            Ok(if is_minus(first.as_str()) {
                neg(inner)
            } else {
                inner
            })
        }
        Rule::power => lower_power(first),
        other => Err(FrontError::plain(format!(
            "未预期一元规则 {other:?}（片段 {whole:?}，子规则 {rules:?}）"
        ))),
    }
}

fn lower_power(p: Pair<Rule>) -> Result<Expr, FrontError> {
    // `^` is a literal and produces no pair — the children are all atoms, count = 1 + number of
    // exponents.
    let mut it = p.into_inner();
    let mut acc = lower_atom(it.next().ok_or_else(|| FrontError::plain("空幂式"))?)?;
    let mut exps: Vec<Expr> = Vec::new();
    for rhs in it {
        exps.push(lower_atom(rhs)?);
    }
    // Convention: the lone superscript T is reserved for transpose (x^T ≡ x').
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
    let pair = match p.as_rule() {
        Rule::atom => p
            .into_inner()
            .next()
            .ok_or_else(|| FrontError::plain("空原子"))?,
        _ => p,
    };
    match pair.as_rule() {
        Rule::frac => {
            let mut it = pair.into_inner();
            let a = lower_brace(it.next().ok_or_else(|| FrontError::plain("frac 缺分子"))?)?;
            let b = lower_brace(it.next().ok_or_else(|| FrontError::plain("frac 缺分母"))?)?;
            Ok(himari::div(a, b))
        }
        Rule::mathrm => {
            let name = pair
                .into_inner()
                .next()
                .ok_or_else(|| FrontError::plain("\\mathrm 缺名字"))?
                .as_str()
                .to_string();
            Ok(var(&name))
        }
        Rule::sqrt => {
            let mut it = pair.into_inner();
            let a = lower_brace(it.next().ok_or_else(|| FrontError::plain("sqrt 缺参数"))?)?;
            Ok(Expr::Call("sqrt".into(), vec![a]))
        }
        Rule::func => {
            let mut it = pair.into_inner();
            let head = it.next().ok_or_else(|| FrontError::plain("函数缺名字"))?;
            let name = match head.as_rule() {
                // \name(...) → the name with the backslash stripped
                Rule::func_name => head.as_str().trim_start_matches('\\').to_string(),
                // \operatorname{name}(...) → only **registered** operator names are accepted
                Rule::func_op => {
                    let (line, col) = head.as_span().start_pos().line_col();
                    let n = head
                        .into_inner()
                        .next()
                        .ok_or_else(|| FrontError::plain("\\operatorname 缺名字"))?
                        .as_str()
                        .to_string();
                    if !matches!(
                        n.as_str(),
                        "abs"
                            | "arg"
                            | "conj"
                            | "re"
                            | "im"
                            | "clamp"
                            | "trace"
                            | "adjoint"
                            | "inv"
                            | "solve"
                            | "roots"
                            | "poly_coeffs"
                            | "dot"
                            | "norm"
                    ) {
                        // Catalogued variant: `n` is the bare name the LSP quick-fix and tests
                        // match on. The spelling hint rides in the suggestion slot.
                        let suggestion = himari::closest_func(&n).unwrap_or_default().to_string();
                        return Err(FrontError::new(
                            line,
                            col,
                            himari::Error::UnknownFunction {
                                name: n.clone(),
                                suggestion,
                            },
                        ));
                    }
                    n
                }
                other => {
                    return Err(FrontError::plain(format!("函数头不合规：{other:?}")));
                }
            };
            let mut args = Vec::new();
            for a in it {
                args.push(lower_expr(a)?);
            }
            if args.is_empty() {
                return Err(FrontError::plain(format!("函数 `{name}` 缺参数")));
            }
            Ok(Expr::Call(name, args))
        }
        Rule::pfrac => {
            // \frac{\partial u}{\partial x} → Deriv(u, x)
            let mut it = pair.into_inner();
            let u = lower_expr(it.next().ok_or_else(|| FrontError::plain("∂ 缺被微分式"))?)?;
            let x = it
                .next()
                .ok_or_else(|| FrontError::plain("∂ 缺微分变量"))?
                .as_str()
                .to_string();
            Ok(himari::deriv(u, x))
        }
        Rule::nabla => {
            // Prefix form: \nabla u (Grad) / \nabla^2 u (Laplacian)
            let pairs: Vec<_> = pair.into_inner().collect();
            let mut idx = 0;
            // nabla_pow is an optional prefix: consumed only when the first pair is exactly it
            // (taking it blindly would swallow the operand)
            let pow2 = matches!(pairs.first().map(|x| x.as_rule()), Some(Rule::nabla_pow));
            if pow2 {
                idx = 1;
            }
            let operand = lower_power(
                pairs
                    .get(idx)
                    .ok_or_else(|| FrontError::plain("∇ 缺操作数"))?
                    .clone(),
            )?;
            Ok(if pow2 {
                himari::laplacian(operand)
            } else {
                himari::grad(operand)
            })
        }
        Rule::op_sum => {
            let mut it = pair.into_inner();
            let var = it
                .next()
                .ok_or_else(|| FrontError::plain("\\sum 缺指标"))?
                .as_str()
                .to_string();
            let lower = lower_expr(it.next().ok_or_else(|| FrontError::plain("\\sum 缺下界"))?)?;
            let upper = lower_expr(it.next().ok_or_else(|| FrontError::plain("\\sum 缺上界"))?)?;
            let body = lower_power(it.next().ok_or_else(|| FrontError::plain("\\sum 缺体"))?)?;
            Ok(himari::sum(var, lower, upper, body))
        }
        Rule::op_int => {
            let mut it = pair.into_inner();
            let lower = lower_atom(it.next().ok_or_else(|| FrontError::plain("\\int 缺下界"))?)?;
            let upper = lower_atom(it.next().ok_or_else(|| FrontError::plain("\\int 缺上界"))?)?;
            let body = lower_power(it.next().ok_or_else(|| FrontError::plain("\\int 缺体"))?)?;
            let var = it
                .next()
                .map(|p| p.as_str().to_string())
                .unwrap_or_else(|| "x".to_string());
            Ok(himari::integral(var, lower, upper, body))
        }
        Rule::mat_env => {
            let mut rows: Vec<Vec<himari::PatternCell>> = Vec::new();
            for row in pair.into_inner() {
                if row.as_rule() != Rule::mrow {
                    continue; // mrow_sep
                }
                let mut cells = Vec::new();
                for cell in row.into_inner() {
                    // cell is the mcell **wrapper**: only the inner pair is the dots or expr
                    let inner = cell
                        .clone()
                        .into_inner()
                        .next()
                        .ok_or_else(|| FrontError::plain("空矩阵格"))?;
                    cells.push(match inner.as_rule() {
                        Rule::dots => himari::PatternCell::Dots,
                        _ => himari::PatternCell::El(lower_expr(inner)?),
                    });
                }
                rows.push(cells);
            }
            Ok(mat_from_cells(rows))
        }
        Rule::group | Rule::brace => lower_brace(pair),
        Rule::number => {
            let s = pair.as_str();
            s.parse::<f64>()
                .map(num)
                .map_err(|_| FrontError::plain(format!("坏数字 `{s}`")))
        }
        Rule::symbol => {
            let mut name = String::new();
            let mut base_name = String::new();
            for c in pair.clone().into_inner() {
                match c.as_rule() {
                    Rule::greek | Rule::single => {
                        name.push_str(&greek_name(c.as_str()));
                        base_name.push_str(&greek_name(c.as_str()));
                    }
                    Rule::sub_idx => {
                        // Braced subscript: letters/commas → an index (a_{i,j})
                        let body = c
                            .into_inner()
                            .next()
                            .map(|b| b.as_str().to_string())
                            .unwrap_or_default();
                        let idx: Vec<String> =
                            body.split(',').map(|x| x.trim().to_string()).collect();
                        if !idx.is_empty() && !idx[0].is_empty() {
                            return Ok(himari::indexed(base_name.clone(), idx));
                        }
                    }
                    Rule::sub_name => {
                        // Bare subscript: part of the name (E_a, i_0)
                        name.push('_');
                        if let Some(b) = c.into_inner().next() {
                            name.push_str(b.as_str());
                        }
                    }
                    _ => {}
                }
            }
            Ok(var(name))
        }
        other => Err(FrontError::plain(format!("未预期原子规则 {other:?}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use himari::HashMap;
    use himari::{dim_check, dims_of, eval, simplify};

    const BV: &str = r#"decl { i_0 : current_density [A/m^2]; eta : overpotential [V]; alpha : dimensionless; n : dimensionless; }
        i = i_0 ( \exp( \frac{\alpha n F \eta}{R T} ) - \exp( -\frac{(1 - \alpha) n F \eta}{R T} ) )"#;

    #[test]
    fn parses_butler_volmer() {
        let decls_set = parse(BV).expect("应可解析");
        let (decls, e) = decls_set.into_single();
        assert_eq!(decls.len(), 4);
        // Juxtaposition multiplication + \frac + nested parentheses
        let mut env = HashMap::new();
        env.insert("i_0".into(), 1.0);
        env.insert("eta".into(), 0.0);
        env.insert("alpha".into(), 0.5);
        env.insert("n".into(), 1.0);
        env.insert("F".into(), 96485.0);
        env.insert("R".into(), 8.314);
        env.insert("T".into(), 298.15);
        env.insert("x".into(), 0.0);
        // eta = 0 ⇒ both exps equal 1 ⇒ i = 0
        assert!(eval(&e, &env).unwrap().abs() < 1e-12);
    }

    #[test]
    fn implicit_multiplication_is_unambiguous_in_latex() {
        let ___s = parse(r"decl { x : a; y : a; } x y").unwrap();
        let (_, e) = ___s.into_single();
        assert_eq!(e, himari::mul(var("x"), var("y")));
    }

    #[test]
    fn rejects_function_without_parens() {
        // `\sin^2 x` is ambiguous notation and must be rejected instead of guessed
        let err = parse(r"decl { x : a; } \sin^2 x").unwrap_err();
        assert_eq!(err.code.code(), "syntax-error", "{}", err.msg);
    }

    #[test]
    fn undeclared_symbol_rejected_by_dim_check() {
        let decls_set = parse(r"decl { x : a; } x + y").unwrap();
        let (decls, e) = decls_set.into_single();
        let dims = dims_of(&decls).unwrap();
        let err = dim_check(&e, &dims).unwrap_err();
        // Assert the stable code, not a sentence: `dim_check` is catalogued now.
        assert_eq!(err.code(), "undeclared-symbol", "{err}");
    }

    #[test]
    fn round_trip_latex_is_stable() {
        let ___s = parse(r"decl { a : l; b : l; } \frac{a}{b} + a b^2").unwrap();
        let (_, e) = ___s.into_single();
        let printed = himari::to_latex(&e);
        let ___s = parse(&format!("decl {{ a : l; b : l; }} {printed}")).unwrap();
        let (_, e2) = ___s.into_single();
        assert_eq!(himari::key(&simplify(&e)), himari::key(&simplify(&e2)));
    }

    #[test]
    fn multi_letter_names_round_trip_and_render_as_greek() {
        // Regression: `to_latex` used to be a **substring replacement** (`alphabet` →
        // `\alphabet`, `sigma` → s·i·g·m·a), so writing back a multi-letter name was neither
        // parseable nor good-looking.
        let decls = r"decl { sigma : d; sigma_0 : d; alphabet : d; h_0 : d; x : d; } ";
        // Greek names → the real glyph; with a subscript the glyph is kept; everything else →
        // \mathrm
        let cases = [
            ("sigma", r"\sigma"),
            ("sigma_0", r"\sigma_0"),
            ("alphabet", r"\mathrm{alphabet}"),
            ("h_0", r"\mathrm{h_0}"),
            ("x", "x"),
        ];
        for (name, want) in cases {
            // On the LaTeX surface a multi-letter name **must** be written `\mathrm{...}`
            // (writing it bare is implicit multiplication: `sigma` = s·i·g·m·a). Writing back
            // and reading therefore land on the same side.
            let src = if name.len() == 1 {
                name.to_string()
            } else {
                format!("\\mathrm{{{name}}}")
            };
            let set = parse(&format!("{decls}{src}")).unwrap();
            let (_, e) = set.into_single();
            let printed = himari::to_latex(&e);
            assert_eq!(printed, want, "`{name}` 的回写不对");
            // Round trip: writing it back must still parse into the same variable
            let back = parse(&format!("{decls}{printed}"))
                .unwrap_or_else(|err| panic!("{name} → {printed} 不可解析：{err:?}"));
            let (_, e2) = back.into_single();
            assert_eq!(
                himari::key(&e),
                himari::key(&e2),
                "{name} → {printed} ≠ 回环"
            );
        }
    }

    #[test]
    fn hostile_input_is_rejected_with_a_readable_error() {
        // Depth: over-deep nesting must be stopped at the **parse exit** (everything downstream
        // is recursive)
        let deep = format!("decl {{ x : d; }} {}x{}", "(".repeat(400), ")".repeat(400));
        let err = parse(&deep).unwrap_err();

        // Two gates: pest's own stack limit (translated into plain words) or our IR depth guard
        assert_eq!(err.code.code(), "nesting-too-deep", "{}", err.msg);
        // Under the limit parsing proceeds as usual
        let ok = format!("decl {{ x : d; }} {}x{}", "(".repeat(50), ")".repeat(50));
        assert!(parse(&ok).is_ok(), "50 层应放行");
        // Invisible characters
        let err = parse("decl { x : d; } x\u{200b} + 1").unwrap_err();
        assert_eq!(err.code.code(), "invisible-character", "{}", err.msg);
        // A function without arguments (`f()` is not expressible on the LaTeX surface anyway;
        // the MATLAB surface is guarded by the alias)
        assert!(parse("decl { x : d; } \\exp()").is_err());
    }

    #[test]
    fn round_trip_latex_respects_precedence() {
        // Missing parentheses on write-back = a wrong formula (`(j·ω)^(1/2)` rendered as
        // `j·√ω`). Guard the two shapes most prone to losing parentheses: the base of a power and
        // the right operand of a subtraction.
        let decls = r"decl { a : d; b : d; c : d; } ";
        for (src, printed_has) in [
            (r"(a + b)^2", "(a + b)^{2}"),
            (r"a - (b - c)", "a - (b - c)"),
        ] {
            let set = parse(&format!("{decls}{src}")).unwrap();
            let (_, e) = set.into_single();
            let printed = himari::to_latex(&e);
            assert!(
                printed.contains(printed_has),
                "回写丢了括号：{src} → {printed}"
            );
            let back = parse(&format!("{decls}{printed}")).unwrap();
            let (_, e2) = back.into_single();
            assert_eq!(
                himari::key(&himari::simplify(&e)),
                himari::key(&himari::simplify(&e2)),
                "{src} → {printed} ≠ 回环"
            );
        }
        // The base of the power is a fraction: parentheses are mandatory, otherwise `(a/b)^2` and
        // `a/b^2` get confused
        let set = parse(&format!(r"{decls}\frac{{a}}{{b}}^2")).unwrap();
        let (_, e) = set.into_single();
        let printed = himari::to_latex(&e);
        let back = parse(&format!("{decls}{printed}")).unwrap();
        let (_, e2) = back.into_single();
        assert_eq!(
            himari::key(&himari::simplify(&e)),
            himari::key(&himari::simplify(&e2)),
            r"回写不等价：\frac{{a}}{{b}}^2 → {printed}"
        );
        // Complex operators: the \operatorname spelling round-trips; unregistered names must be
        // rejected
        let set = parse(&format!(r"{decls}\operatorname{{abs}}(a)")).unwrap();
        let (_, e) = set.into_single();
        assert_eq!(himari::to_latex(&e), r"\operatorname{abs}(a)");
        assert!(
            parse(&format!(r"{decls}\operatorname{{foobar}}(a)")).is_err(),
            "未登记的 operatorname 必须拒绝"
        );
    }

    #[test]
    fn bmatrix_concrete_folds_to_mat() {
        let (_, e) = parse(r"decl { a : d; b : d; } \begin{bmatrix} a & b \\ b & a \end{bmatrix}")
            .unwrap()
            .into_single();
        match e {
            Expr::Mat(rows) => assert_eq!(rows.len(), 2),
            other => panic!("期望具体矩阵，得到 {other:?}"),
        }
    }

    #[test]
    fn bmatrix_with_dots_is_pattern() {
        let src = r"decl { b : d; c : d; a : d; } \begin{bmatrix} b & c & \cdots & 0 \\ a & b & c & \cdots \\ \cdots & a & b & c \\ 0 & \cdots & a & b \end{bmatrix}";
        let ___s = parse(src).unwrap();
        let (_, e) = ___s.into_single();
        match e {
            Expr::MatPattern(p) => {
                let kind = himari::recognize_pattern(&p).unwrap();
                assert!(matches!(kind, himari::MatrixKind::Tridiag { .. }));
            }
            other => panic!("期望模式矩阵，得到 {other:?}"),
        }
    }

    #[test]
    fn transpose_via_superscript() {
        let ___s = parse(r"decl { x : d; } x^T").unwrap();
        let (_, e) = ___s.into_single();
        assert_eq!(e, himari::transpose(var("x")));
    }

    #[test]
    fn partial_frac_and_sum() {
        // \frac{\partial u}{\partial x}
        let set = parse(r"decl { u : c [J]; x : l [m]; } \frac{\partial u}{\partial x}").unwrap();
        let (_, e) = set.into_single();
        assert!(matches!(e, himari::Expr::Deriv(_, _)));
        // \sum_{i=1}^{n} i^2
        let set2 = parse(r"decl { i : d; n : d; } \sum_{i=1}^{3} i^2").unwrap();
        let (_, e2) = set2.into_single();
        let mut env = himari::HashMap::new();
        env.insert("n".to_string(), 3.0);
        assert_eq!(himari::eval(&e2, &env).unwrap(), 14.0);
    }

    #[test]
    fn operator_latex_roundtrip() {
        // to_latex → parse again → same key (the operator registry's write-back round trip
        // closes)
        let decls = r"decl { u : c [J]; x : l [m]; i : d; } ";
        for src in [
            r"\nabla u",
            r"\nabla^2 u",
            r"\frac{\partial u}{\partial x}",
            r"\sum_{i=1}^{3} i^2",
            r"\int_{0}^{1} x^2 \, dx",
        ] {
            let full = format!("{decls}{src}");
            let set = parse(&full).unwrap_or_else(|e| panic!("{src}: {e:?}"));
            let (_, e) = set.into_single();
            let printed = himari::to_latex(&e);
            let back = parse(&format!("{decls}{printed}"))
                .unwrap_or_else(|err| panic!("回写不可再解析: {printed} ← {src}: {err:?}"));
            let (_, e2) = back.into_single();
            assert_eq!(
                himari::key(&himari::simplify(&e)),
                himari::key(&himari::simplify(&e2)),
                "{src} → {printed} ≠ 回环"
            );
        }
    }

    #[test]
    fn subscript_dispatch_name_vs_index() {
        // A purely numeric subscript = part of the name (i_0)
        let (_, a) = parse(r"decl { i_0 : d; } i_0").unwrap().into_single();
        assert_eq!(a, var("i_0"));
        // A bare letter subscript = part of the name (same for E_a); **braces** make it an index
        let (_, b) = parse(r"decl { a_i : d; i : d; } a_i")
            .unwrap()
            .into_single();
        assert_eq!(b, var("a_i"));
        let (_, b2) = parse(r"decl { a : d; i : d; } a_{i}")
            .unwrap()
            .into_single();
        assert!(matches!(b2, himari::Expr::Indexed { .. }));
        // Comma-separated multi-index (note: `j` is the reserved imaginary unit and cannot be
        // declared as an index letter — indices conventionally use i/k/l, or one writes them
        // directly without declaring index symbols)
        let (_, c) = parse(r"decl { a : d; i : d; k : d; } a_{i,k}")
            .unwrap()
            .into_single();
        match c {
            himari::Expr::Indexed { indices, .. } => assert_eq!(indices.len(), 2),
            other => panic!("期望 Indexed，得到 {other:?}"),
        }
    }

    #[test]
    fn error_carries_line_and_col() {
        let err = parse("decl { x : a; }\n x + ").unwrap_err();
        assert_eq!(err.line, 2, "{err:?}");
        assert!(err.col > 0, "{err:?}");
    }
}

#[cfg(test)]
mod op_coverage {
    use super::parse;
    use alloc::format;
    use alloc::string::{String, ToString};
    use alloc::vec::Vec;

    // Registry consistency: **every** builtin function must be spellable as `\operatorname{...}`.
    // There was a time when `poly_coeffs` worked on the matex surface but could not be parsed on
    // the LaTeX surface — the same formula silently became unreachable on another front end, and
    // no gate could detect that kind of inconsistency, so it is pinned down here.

    #[test]
    fn every_registered_builtin_is_spellable_with_operatorname() {
        // Iterate the core function list directly (`FUNC_NAMES` is the single source of truth for
        // `func_arity`): a new operator automatically enters this net instead of the hand-copied
        // list failure mode of "the name was added but the test did not follow".
        let names: Vec<&str> = himari::FUNC_NAMES.to_vec();
        assert!(names.len() >= 25, "core 的函数清单不该这么短");
        for n in &names {
            assert!(
                himari::func_arity(n).is_some(),
                "`{n}` 在 FUNC_NAMES 里但 func_arity 不认（两处不一致）"
            );
        }
        for n in names {
            // Every builtin function must have **at least one** LaTeX spelling: a dedicated
            // command such as `\exp` where one exists, `\operatorname{...}` when there is no fixed
            // glyph. Failing both = this formula is unreachable on the front end (`poly_coeffs`
            // once fell through the whitelist), and no other gate can detect that inconsistency.
            // `\sqrt` is LaTeX's brace form and cannot take parentheses
            if n == "sqrt" {
                let src = "decl {\n    x : d;\n}\ny = \\sqrt{x}\n";
                assert!(parse(src).is_ok(), "`sqrt` 的 LaTeX 写法不可用");
                continue;
            }
            let body = match n {
                "solve" => "x - 1, x".to_string(),
                "roots" | "poly_coeffs" => "x ^ 2 + x, x".to_string(),
                // Only verify "the operator name is spellable": shape/semantics are left to the
                // downstream checks
                "dot" => "x, x".to_string(),
                "min" | "max" => "x, x".to_string(),
                "clamp" => "x, x, x".to_string(),
                _ => "x".to_string(),
            };
            let spill_op = format!("decl {{\n    x : d;\n}}\ny = \\operatorname{{{n}}}({body})\n");
            let spill_cmd = format!("decl {{\n    x : d;\n}}\ny = \\{n}({body})\n");
            assert!(
                parse(&spill_op).is_ok() || parse(&spill_cmd).is_ok(),
                "`{n}` 在 LaTeX 面两种写法都不可达（专属命令与 \\operatorname 白名单都缺）"
            );
        }
    }

    #[test]
    fn division_operators_are_real_division_and_left_associative() {
        // Regression: the LaTeX surface **once had no division operator** — it only knew
        // `\frac{a}{b}`, and `a / b` was a straight syntax error.
        // That was "throwing out the baby with the bathwater": `/` is the most ordinary way to
        // write division. More insidious was that the lowerer used to emit `mul_op` always as
        // `mul`, so changing only the grammar without the lowerer turned `a / b` into `a * b`
        // (a silent wrong value).
        // Here each case is checked **numerically**, and left associativity plus mixing with
        // `\frac` are pinned down.
        use himari::HashMap;
        let env: HashMap<String, f64> = [("a", 3.0), ("b", 4.0), ("c", 5.0)]
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect();
        let cases = [
            ("a / b", 3.0 / 4.0),
            (r"a \div b", 3.0 / 4.0),                // synonymous with /
            ("a / b * c", (3.0 / 4.0) * 5.0),        // left-associative: (a/b)*c
            ("a / b / c", (3.0 / 4.0) / 5.0),        // left-associative: (a/b)/c
            ("a * b / c", (3.0 * 4.0) / 5.0),        // equal precedence, left-associative
            (r"\frac{a}{b} / c", (3.0 / 4.0) / 5.0), // mixed with \frac
        ];
        for (src, want) in cases {
            let full = format!("decl {{ a : d; b : d; c : d; }}\nq = {src}\n");
            let set = parse(&full).unwrap_or_else(|e| panic!("{src} 应可解析：{e:?}"));
            let e = set.into_single().1;
            let got = himari::eval(&e, &env).unwrap();
            assert!(
                (got - want).abs() < 1e-12,
                "{src}: 得到 {got}，应为 {want}（除法被当成乘法或结合性错了）"
            );
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod parser_thread_tests {
    use super::*;

    /// Twin of the matlab test: a panic inside the parser thread surfaces as `syntax-error`.
    #[test]
    fn a_parser_panic_becomes_a_syntax_error_not_an_abort() {
        let run = super::parse_on_big_stack(|| -> Result<himari::FormulaSet, FrontError> {
            panic!("simulated parser bug")
        });
        let err = finish_descent(run).expect_err("a panicking closure must not report Ok");
        assert_eq!(err.code.code(), "syntax-error", "{}", err.msg);
        assert!(err.msg.contains("simulated parser bug"), "{}", err.msg);
    }
}
