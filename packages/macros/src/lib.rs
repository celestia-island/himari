//! proc-macro entry points of himari.
//!
//! - String or bare tokens: `formula_latex!(…)` / `formula_matlab!(…)`;
//! - External file: `formula_file!("formulas/bv.formula", matlab)` (the dialect is mandatory);
//! - Complex formulas and checked-in generated code: use the build.rs lane of
//!   `himari-buildhelper`.
//!
//! All entry points share the front end + IR + emitter; the dialect is decided by the macro or by
//! an explicit argument and is never guessed.

use proc_macro::{Delimiter, Spacing, TokenStream, TokenTree};
use std::str::FromStr;

/// The macro lane has an explicit dialect **only** — there is no `Auto` in the enum.
///
/// "The macro is the dialect" (2026-11-24 user decision): whichever macro wraps the formula, the
/// formula is read according to that specification.
/// This is not merely a style preference; it removes a real **silent wrong value**: for
/// `si * x` with `si` declared, the old Auto (try matex first, use it if it succeeds) gave
/// `si·x`, whereas the LaTeX semantics are `s·i·x` — same file, same inputs, the two differ by
/// a factor of several, and Auto reported no error at all. Auto has no way to know which one the
/// author meant, so it should not exist anywhere that affects semantics. Removing `Auto` from
/// the type makes the compiler guarantee exactly that.
#[derive(Clone, Copy)]
enum Dialect {
    Latex,
    Matlab,
}

/// Inline formula, **matex semantics** (the canonical surface).
///
/// For LaTeX use [`formula_latex!]. This macro does not guess the dialect: when LaTeX is fed to
/// it, a multi-letter name is taken as a single identifier (`si` is the name, not `s·i`). This is
/// a deliberate trade-off of "the macro is the dialect" — the entry-point name is kept, but the
/// semantics are definite.
///
/// # Runtime matrix/vector results are fallible
///
/// A declaration whose quantity is `matrix`/`*_matrix` (or `vector`/`*_vector`) selects the
/// checked runtime lane. Because that lane can fail (singular or ill-conditioned system,
/// non-square matrix, out-of-range index, negative discriminant), the macro evaluates to
/// `Result<_, himari::runtime::NumericError>` rather than to the value itself.
/// `decl` only supplies dimensions — the symbols still have to exist at the call site.
///
/// ```
/// use himari::runtime::NumericError;
///
/// fn solve_small() -> Result<Vec<f64>, NumericError> {
///     let a = vec![vec![2.0f64, 0.0], vec![0.0, 4.0]];
///     let b = vec![2.0f64, 8.0];
///     let x: Vec<f64> = himari_macros::formula_matlab!(
///         r"decl { a : stiffness_matrix [N/m]; b : load_vector [N]; } solve(a, b)"
///     )?;
///     Ok(x)
/// }
///
/// assert_eq!(solve_small().unwrap(), vec![1.0, 2.0]);
/// ```
///
/// Scalar-only formulas keep returning the plain value, so existing call sites are unaffected.
#[proc_macro]
pub fn formula(input: TokenStream) -> TokenStream {
    expand_inline(input, Dialect::Matlab)
}

#[proc_macro]
pub fn formula_latex(input: TokenStream) -> TokenStream {
    expand_inline(input, Dialect::Latex)
}

#[proc_macro]
pub fn formula_matlab(input: TokenStream) -> TokenStream {
    expand_inline(input, Dialect::Matlab)
}

/// External formula file, with the dialect **explicitly required**: `formula_file!("f.formula",
/// latex)`.
///
/// Why the argument is not omitted: a file's content cannot be inferred from the macro name, and
/// letting the macro guess would just hide the "Auto ambiguity" somewhere else (see the notes on
/// [`Dialect`]). The dialect is `latex` / `matlab` (or `matex`).
///
/// Note: a proc-macro reading a file is **not tracked by cargo for rebuilds**
/// (`proc_macro::tracked_path` is still nightly), so the production form must go through
/// build.rs.
#[proc_macro]
pub fn formula_file(input: TokenStream) -> TokenStream {
    let raw = render_tokens(input).trim().to_string();
    // Form one: `"f.formula", latex` (string path + bare dialect identifier)
    // Form two: `"f.formula", "latex"` (both are strings)
    let Some(d) = ["latex", "matlab", "matex"]
        .iter()
        .find(|d| raw.ends_with(&format!("\"{d}\"")) || raw.ends_with(&format!(" {d}")))
    else {
        return err_ts(
            "FORMULA-ERROR: formula_file! 必须指定方言 —— 写 `formula_file!(\"f.formula\", latex)` 或 `formula_file!(\"f.formula\", matlab)`。\
             文件内容无法从宏名推断，本宏不猜。",
        );
    };
    let dialect = if *d == "latex" {
        Dialect::Latex
    } else {
        Dialect::Matlab
    };
    let body = raw[..raw.len() - d.len()]
        .trim_end()
        .trim_end_matches(',')
        .trim_end();
    // Strip the quotes from a string literal (`"f.formula"` → `f.formula`); a bare token stream
    // is left as is
    let path = lit_body(body).unwrap_or_else(|| body.to_string());
    let base = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let full = std::path::Path::new(&base).join(&path);
    match std::fs::read_to_string(&full) {
        Ok(src) => {
            // Rebuild tail: register it in the manifest, and the consumer's build.rs
            // himari_buildhelper::emit() turns it into a rerun-if-changed (the pragmatic stand-in
            // while tracked_path is still nightly)
            himari_buildhelper::track(&full.display().to_string());
            emit(&src, dialect, &path)
        }
        Err(e) => err_ts(&format!(
            "FORMULA-ERROR: 读不到公式文件 {}: {e}",
            full.display()
        )),
    }
}

fn expand_inline(input: TokenStream, dialect: Dialect) -> TokenStream {
    // A string literal ⇒ the text lane; otherwise ⇒ the token lane.
    let flat = render_tokens(input.clone());
    match lit_body(&flat) {
        Some(src) => emit(&src, dialect, "<literal>"),
        None => {
            // The token lane restores **newlines**: span line numbers are available on stable
            // (measured on 1.99), so "multi-line macro block + newline-separated federated
            // formulas" also works in the bare token lane, and line numbers are padded up to the
            // source file so diagnostics still point at the same line.
            let text = render_tokens_multiline(input);
            emit(&text, dialect, "<tokens>")
        }
    }
}

/// Rebuild newlines from span line numbers: line numbers are aligned to the **source file**
/// (blank lines are padded before the first token), so the `line` reported by the front end is the
/// source file's line number.
fn render_tokens_multiline(ts: TokenStream) -> String {
    let mut out = String::new();
    let mut last = 0usize;
    render_ml_into(ts, &mut out, &mut last);
    out
}

fn render_ml_into(ts: TokenStream, out: &mut String, last: &mut usize) {
    for tt in ts {
        let l = tt.span().start().line();
        let pad = if *last == 0 {
            l.saturating_sub(1)
        } else {
            l.saturating_sub(*last)
        };
        for _ in 0..pad {
            out.push('\n');
        }
        if l > *last {
            *last = l;
        }
        match tt {
            TokenTree::Group(g) => {
                let (open, close) = match g.delimiter() {
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::None => ("", ""),
                };
                out.push_str(open);
                render_ml_into(g.stream(), out, last);
                out.push_str(close);
                out.push(' ');
            }
            TokenTree::Ident(i) => {
                out.push_str(&i.to_string());
                out.push(' ');
            }
            TokenTree::Literal(l) => {
                out.push_str(&l.to_string());
                out.push(' ');
            }
            TokenTree::Punct(p) => {
                out.push(p.as_char());
                if p.spacing() == Spacing::Alone {
                    out.push(' ');
                }
            }
        }
    }
}

fn emit(src: &str, dialect: Dialect, origin: &str) -> TokenStream {
    match compile(src, dialect) {
        Ok(code) => match TokenStream::from_str(&code) {
            Ok(ts) => ts,
            Err(e) => err_ts(&format!(
                "FORMULA-ERROR: 生成的代码不是合法 token 流（{e}）：{code}"
            )),
        },
        Err(e) => {
            let mut msg = format!("FORMULA-ERROR: {}", e.render(origin));
            if let Some(sn) = snippet(src, e.line, e.col) {
                msg.push('\n');
                msg.push_str(&sn);
            }
            err_ts(&msg)
        }
    }
}

fn compile(src: &str, dialect: Dialect) -> Result<String, himari::FrontError> {
    let set = match dialect {
        Dialect::Latex => himari_latex::parse(src),
        Dialect::Matlab => himari_matlab::parse(src),
    }?;
    // Federated dimensions: the names of earlier formulas join the declaration table with the
    // dimension of their expression.
    let dims = himari::dims_of_set(&set).map_err(himari::FrontError::plain)?;
    let runtime_shapes = himari::runtime::shapes_of_set(&set).map_err(himari::FrontError::plain)?;
    if set.items.len() != 1
        && set
            .items
            .iter()
            .any(|item| himari::runtime::uses_runtime_values(&item.expr, &runtime_shapes))
    {
        return Err(himari::FrontError::plain(
            "runtime matrix/vector macros currently support exactly one formula; use the build.rs lane for formula chains",
        ));
    }
    for item in &set.items {
        // Same rule as dims_of_set: exponents like `^(1/2)` must be folded before the dimension
        // check
        let folded = himari::simplify(&item.expr);
        himari::runtime::dim_check_runtime(&folded, &dims, &runtime_shapes)
            .map_err(himari::FrontError::plain)?;
        if !himari::runtime::uses_runtime_values(&folded, &runtime_shapes) {
            himari::shape_check(&folded).map_err(himari::FrontError::plain)?;
        }
        if contains_pattern(&item.expr) {
            return Err(himari::FrontError::plain(
                "内联车道不支持模式矩阵（构造函数车道需要文件名/生成器）；\n                 → 把这条公式放进 `.formula` 文件，用 formula_file!(\"…\") 或 build.rs 车道",
            ));
        }
    }
    // The product = a federated let chain (a later formula references an earlier one, with zero
    // recomputation), ending with the value of the last formula.
    let mut out = String::from("{\n");
    let mut last = String::new();
    // Names already bound as complex: a later formula referencing one must also take the complex
    // lane as a whole
    let mut cx_names: Vec<String> = Vec::new();
    for (k, item) in set.items.iter().enumerate() {
        // Complex triage: a formula containing the imaginary unit `j` (or referencing a complex
        // binding) must take the complex lane.
        // Before this triage, `j` was treated as a **free variable at the call site**: if a `j`
        // happened to be in scope it silently computed a wrong real number (the imaginary part
        // dropped entirely), and if not it reported E0425.
        let folded = himari::simplify(&item.expr);
        let is_cx = himari::uses_complex(&folded)
            || himari::vars_of(&folded)
                .iter()
                .any(|v| cx_names.contains(v));
        let is_runtime = himari::runtime::uses_runtime_values(&folded, &runtime_shapes);
        let fallible_roots = himari::runtime::is_real_quadratic_formula(&folded);
        if is_runtime && is_cx {
            return Err(himari::FrontError::plain(
                "complex runtime matrix parameters are not supported yet; use real matrices",
            ));
        }
        let block = if is_runtime || fallible_roots {
            himari::runtime::emit_runtime(&folded, &runtime_shapes)
                .map_err(himari::FrontError::plain)?
                .code
        } else if is_cx {
            himari::emit_block_complex_with(&folded, &cx_names)
        } else {
            himari::emit_block(&folded)
        };
        match &item.name {
            Some(name) => {
                let id = rust_ident(name);
                out.push_str(&format!("    let {id} = {block};\n"));
                if is_cx {
                    cx_names.push(name.clone());
                }
                last = id;
            }
            None => {
                if k + 1 == set.items.len() {
                    // The last formula is unnamed: the block itself is the value
                    out.push_str(&format!("    {block}\n"));
                    last = String::new();
                } else {
                    // An unnamed one in the middle: it cannot be referenced by name, so use a
                    // positional name
                    let id = format!("__f{k}");
                    out.push_str(&format!("    let {id} = {block};\n"));
                    if is_cx {
                        cx_names.push(id.clone());
                        // The alias of an unnamed intermediate = the generated id itself
                    }
                    last = id;
                }
            }
        }
    }
    if !last.is_empty() {
        out.push_str(&format!("    {last}\n"));
    }
    out.push('}');
    Ok(out)
}

/// Formula symbol → a legal Rust identifier (same rule as the core emitter).
fn rust_ident(n: &str) -> String {
    let mut s: String = n
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true) {
        s.insert(0, '_');
    }
    s
}

/// A 1-based line/column → a snippet with a caret.
fn snippet(src: &str, line: usize, col: usize) -> Option<String> {
    if line == 0 {
        return None;
    }
    let text = src.lines().nth(line - 1)?;
    let pad = " ".repeat(col.saturating_sub(1));
    Some(format!("    {text}\n    {pad}^"))
}

fn err_ts(msg: &str) -> TokenStream {
    TokenStream::from_str(&format!("compile_error!({msg:?})"))
        .unwrap_or_else(|_| TokenStream::new())
}

/// Restore a token stream into text that is convenient for the grammar (closer to the source than
/// `to_string()`: no space is inserted between Joint punctuation, so `^-1` is not split into
/// `^ - 1`).
fn render_tokens(ts: TokenStream) -> String {
    let mut out = String::new();
    render_into(ts, &mut out);
    out
}

fn render_into(ts: TokenStream, out: &mut String) {
    for tt in ts {
        match tt {
            TokenTree::Group(g) => {
                let (open, close) = match g.delimiter() {
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::None => ("", ""),
                };
                out.push_str(open);
                render_into(g.stream(), out);
                out.push_str(close);
                out.push(' ');
            }
            TokenTree::Ident(i) => {
                out.push_str(&i.to_string());
                out.push(' ');
            }
            TokenTree::Literal(l) => {
                out.push_str(&l.to_string());
                out.push(' ');
            }
            TokenTree::Punct(p) => {
                out.push(p.as_char());
                if p.spacing() == Spacing::Alone {
                    out.push(' ');
                }
            }
        }
    }
}

/// Accepts only the shape "the whole input is a single string literal".
fn lit_body(s: &str) -> Option<String> {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix('r') {
        let hashes = rest.chars().take_while(|c| *c == '#').count();
        let rest = rest.get(hashes..)?;
        let rest = rest.strip_prefix('"')?;
        let close = format!("\"{}", "#".repeat(hashes));
        return rest.strip_suffix(&close).map(|x| x.to_string());
    }
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        return Some(unescape(&s[1..s.len() - 1]));
    }
    None
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('0') => out.push('\0'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Whether it contains a pattern matrix or a transpose (emitting those two belongs to the file
/// lane).
fn contains_pattern(e: &himari::Expr) -> bool {
    use himari::Expr;
    match e {
        Expr::MatPattern(_) | Expr::Transpose(_) => true,
        Expr::Num(_) | Expr::Var(_) => false,
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
            contains_pattern(a) || contains_pattern(b)
        }
        Expr::Neg(a) => contains_pattern(a),
        Expr::Call(_, args) => args.iter().any(contains_pattern),
        Expr::Mat(rows) => rows.iter().flatten().any(contains_pattern),
        // Symbolic operators / sums and integrals / indexed symbols also require the file lane
        // (their emit is a compile_error placeholder)
        Expr::Deriv(..)
        | Expr::Grad(_)
        | Expr::Laplacian(_)
        | Expr::Sum { .. }
        | Expr::Integral { .. }
        | Expr::Indexed { .. } => true,
    }
}
