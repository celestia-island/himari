// WGSL emission lane: the same formula → an f32 kernel usable inside a GPU compute shader.
//
// **Deliberately a subset only**, and it rejects outright:
//   - only real scalars (complex/matrix/det/inv/solve are all rejected, with a reason why);
//   - only operators that really have a matching WGSL builtin
//     (exp/log/sqrt/sin/cos/tan/sinh/cosh/tanh/
//     abs/min/max/clamp) — no `atan2`, no step, no modulo;
//   - exponents accept only small integer constants (`pow` is a transcendental function on the GPU;
//     `x*x` is both faster and more accurate).
//
// Verification boundary (to be honest): the repo sandbox has no naga / tint, so **no WGSL
// compiler verification was done**. What we have here is: a structural gate (bracket balancing,
// identifier whitelist, f32 literal form) + golden comparison against a hand-written
// reference kernel. Before it really goes on a GPU, it must be run once on a machine with wgpu.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

/// WGSL reserved words (rejected outright on collision with a user symbol, instead of
/// generating a kernel that will not compile).
// WGSL keyword + reserved word (compiled from the naga keyword table against the W3C WGSL CRD).
// Better to over-reject: a colliding name produces a shader that will not compile, which is far
// worse than "we rejected some but missed one".
// Not guaranteed to cover every new word of the spec verbatim — when a missed word is hit,
// rustc/wgslc will report it; extend the table then.
const WGSL_KEYWORDS: &[&str] = &[
    // keywords
    "alias",
    "break",
    "case",
    "const",
    "const_assert",
    "continue",
    "continuing",
    "default",
    "diagnostic",
    "discard",
    "else",
    "enable",
    "false",
    "fn",
    "for",
    "if",
    "let",
    "loop",
    "override",
    "requires",
    "return",
    "struct",
    "switch",
    "true",
    "var",
    "while",
    // Types and syntactic names
    "array",
    "atomic",
    "bitcast",
    "bool",
    "f16",
    "f32",
    "i32",
    "ptr",
    "sampler",
    "sampler_comparison",
    "storage",
    "texture_1d",
    "texture_2d",
    "texture_3d",
    "u32",
    "vec2",
    "vec3",
    "vec4",
    "mat2x2",
    "mat2x3",
    "mat2x4",
    "mat3x2",
    "mat3x3",
    "mat3x4",
    "mat4x2",
    "mat4x3",
    "mat4x4",
    "uniform",
    "private",
    "workgroup",
    // reserved words (the full table from the spec)
    "NULL",
    "Self",
    "abstract",
    "active",
    "alignas",
    "alignof",
    "as",
    "asm",
    "auto",
    "await",
    "become",
    "binding_array",
    "cast",
    "catch",
    "class",
    "co_await",
    "co_return",
    "co_yield",
    "coherent",
    "column_major",
    "common",
    "compile",
    "compose",
    "concept",
    "const_cast",
    "const_cast",
    "constexpr",
    "constexpr",
    "delete",
    "do",
    "dynamic_cast",
    "enum",
    "explicit",
    "export",
    "extends",
    "extern",
    "external",
    "fallthrough",
    "filter",
    "final",
    "finally",
    "friend",
    "from",
    "fxgroup",
    "get",
    "goto",
    "groupshared",
    "impl",
    "implements",
    "import",
    "inline",
    "instanceof",
    "interface",
    "layout",
    "mixin",
    "mut",
    "mutable",
    "namespace",
    "new",
    "noexcept",
    "null",
    "nullptr",
    "operator",
    "package",
    "packoffset",
    "pass",
    "public",
    "publish",
    "readonly",
    "ref",
    "register",
    "reinterpret_cast",
    "resource",
    "set",
    "shared",
    "signed",
    "sizeof",
    "static",
    "static_cast",
    "template",
    "this",
    "thread_local",
    "throw",
    "trait",
    "try",
    "type",
    "typedef",
    "typeid",
    "typename",
    "typeof",
    "union",
    "unless",
    "unsigned",
    "using",
    "virtual",
    "volatile",
    "yield",
    // additions from the naga/tint side (also reserved/restricted)
    "bf16",
    "asm_fragment",
    "attribute",
    "async",
    "compile_fragment",
    "consteval",
    "constinit",
    "crate",
    "debugger",
    "decltype",
    "demote",
    "demote_to_helper",
    "match",
    "matrix",
    "mediump",
    "mod",
    "module",
    "move",
    "nil",
    "noinline",
    "priv",
    "pub",
    "restrict",
    "self",
    "snorm",
    "static_assert",
    "subroutine",
    "super",
    "target",
    "unorm",
    "use",
    "varying",
    "wgsl",
    "where",
    "with",
    "writeonly",
    "patch",
    "pipeline",
    "precision",
    "premerge",
    "protected",
];

fn wgsl_ident(name: &str) -> Result<String, crate::Error> {
    let id = crate::rust_ident(name);
    if WGSL_KEYWORDS.contains(&id.as_str()) {
        return Err(crate::Error::WgslReservedWord {
            name: name.to_string(),
            id,
        });
    }
    Ok(id)
}

/// Emit one real-scalar formula as a WGSL function (the caller supplies the parameter list in
/// declaration order).
pub fn emit_wgsl_with_params(
    e: &crate::Expr,
    fname: &str,
    params: &[String],
) -> Result<String, crate::Error> {
    let names: Vec<String> = params
        .iter()
        .map(|p| wgsl_ident(p))
        .collect::<Result<_, _>>()?;
    let fname = wgsl_ident(fname)?;
    let body = wgsl_at(e, 0)?;
    let sig = names
        .iter()
        .map(|p| format!("{p}: f32"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = String::new();
    let _ = writeln!(out, "// 由 himari 生成（f32 标量车道；经 naga 真编译校验）");
    let _ = writeln!(out, "fn {fname}({sig}) -> f32 {{");
    // **WGSL has no expression function body**: the body is a statement block, so the value
    // must come back through `return`.
    // What we emitted before was a bare expression — and the earlier static gate specifically
    // banned `return` (the direction was inverted).
    // This is the structural blind spot of "no compiler, static checks only": it does not know
    // which rule it missed, and it can even judge a correct thing as wrong. The real compile gate
    // (naga) caught this whole class on its first run.
    let _ = writeln!(out, "    return {body};");
    let _ = writeln!(out, "}}");
    // Self-check: the emitter runs its own static gate first (never emit a structurally invalid
    // kernel).
    validate_wgsl(&out)?;
    Ok(out)
}

/// **Static validation**: without relying on a compiler, pin down the few rules that
/// "definitely hold structurally".
///
/// Positioning (to be honest): this is a **quick screen**, not a proof of WGSL correctness.
/// Its blind spot is structural — "it does not know which rule it missed", and it can even judge
/// a correct thing as wrong (it once banned `return`, whereas a WGSL function body precisely must
/// be a `return` statement). The real gate is a real naga compile
/// (`packages/check/tests/wgsl_compile.rs`, `--features wgsl-compile`).
/// The value of keeping the static gate here: it still stops the crudest errors in environments
/// without naga, and gives error messages closer to this lane than a compiler would.
/// This checks every rule that can be decided statically — the emitter runs it itself first,
/// **never emitting a structurally invalid kernel** (better than relying on human eyes
/// afterwards, and it also makes `--wgsl` output self-justifying).
pub fn validate_wgsl(text: &str) -> Result<(), crate::Error> {
    // 1) Bracket balancing (scan char by char, no string handling — there are no strings here in
    //    WGSL)
    let mut depth: i32 = 0;
    for (i, c) in text.char_indices() {
        match c {
            '(' | '{' | '[' => depth += 1,
            ')' | '}' | ']' => {
                depth -= 1;
                if depth < 0 {
                    return Err(crate::Error::NotEmittable(format!(
                        "WGSL structure error: an extra `)` at byte {i}"
                    )));
                }
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(crate::Error::NotEmittable(format!(
            "WGSL structure error: parentheses unbalanced by {depth}"
        )));
    }
    // 2) WGSL has no `^` / `**` operator (powers must be written as multiplication or pow)
    if text.contains('^') || text.contains("**") {
        return Err(crate::Error::NotEmittable(
            "WGSL structure error: `^` or `**` encountered (WGSL has no power operator)"
                .to_string(),
        ));
    }
    // 3) Every float literal must carry the `f` suffix.
    //    **This is a style convention of this lane, not a WGSL requirement**: naga was measured to
    //    accept `return 2.5;` (abstract-float settles to f32 by context). The reason to insist on
    //    `f` is explicitness, independence from context, and avoiding being inferred to another
    //    width in an intermediate position whose type is not yet determined.
    let bytes: Vec<char> = text.chars().collect();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            // Treat it as a literal start only when the preceding char is not an identifier char
            let prev = if i == 0 { ' ' } else { bytes[i - 1] };
            if prev.is_alphanumeric() || prev == '_' {
                i += 1;
                continue;
            }
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == '.') {
                i += 1;
            }
            let mut end = i;
            if end < bytes.len() && (bytes[end] == 'e' || bytes[end] == 'E') {
                end += 1;
                if end < bytes.len() && (bytes[end] == '+' || bytes[end] == '-') {
                    end += 1;
                }
                while end < bytes.len() && bytes[end].is_ascii_digit() {
                    end += 1;
                }
            }
            let is_float = bytes[start..end].contains(&'.') || end > i;
            if is_float && (end >= bytes.len() || bytes[end] != 'f') {
                let lit: String = bytes[start..end].iter().collect();
                return Err(crate::Error::NotEmittable(format!("WGSL structure error: float literal `{lit}` lacks the `f` suffix (it would be inferred as f64)")));
            }
            i = end;
            continue;
        }
        i += 1;
    }
    // 4) `inff` / `NaN` must not appear (this lane's counterpart of the non-finite constant gate
    //    in the Rust lane)
    for bad in ["inff", "NaNf", "inf ", "-inff"] {
        if text.contains(bad) {
            return Err(crate::Error::NotEmittable(format!(
                "WGSL structure error: non-finite constant `{bad}`"
            )));
        }
    }
    // 5) The function signature must be `-> f32` (this lane only promises f32 scalars)
    let mut fns = 0usize;
    let mut returns = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("fn ") {
            fns += 1;
            if !t.contains("-> f32") {
                return Err(crate::Error::NotEmittable(format!(
                    "WGSL structure error: the function signature is not `-> f32`: {t}"
                )));
            }
        }
        // 6) The function body must be a statement block: this lane emits exactly one
        //    `return <expr>;`.
        //    (WGSL has no expression function body — we once banned return the other way round,
        //    which was wrong.)
        if t.starts_with("return ") {
            returns += 1;
            if !t.ends_with(';') {
                return Err(crate::Error::NotEmittable(
                    "WGSL structure error: a `return` statement must end with `;`".to_string(),
                ));
            }
        }
    }
    if returns != 1 {
        return Err(crate::Error::NotEmittable(format!(
            "WGSL structure error: expected exactly one `return` statement, found {returns}"
        )));
    }
    if fns == 0 {
        return Err(crate::Error::NotEmittable(
            "WGSL structure error: no `fn` definition".to_string(),
        ));
    }
    Ok(())
}

/// Literal: WGSL needs the `f` suffix (`1.0f`) to know it is f32.
fn lit(v: f64) -> String {
    if super::math::fract(v) == 0.0 && v.abs() < 1e15 {
        format!("{v:.1}f")
    } else {
        format!("{v}f")
    }
}

fn wrap(s: String, p: u8, min: u8) -> String {
    if p < min {
        format!("({s})")
    } else {
        s
    }
}

/// Emit with precedence: the same precedence table as the Rust side, so the two sides can be
/// compared item by item.
fn wgsl_at(e: &crate::Expr, min: u8) -> Result<String, crate::Error> {
    let (text, p) = wgsl_raw(e)?;
    Ok(wrap(text, p, min))
}

/// Returns (text, precedence).
fn wgsl_raw(e: &crate::Expr) -> Result<(String, u8), crate::Error> {
    use crate::Expr as E;
    Ok(match e {
        E::Num(v) => (lit(*v), 9),
        // `j` is the reserved imaginary unit: the GPU lane has real numbers only, so reject it
        // explicitly (never silently treat it as a variable)
        E::Var(n) if n == "j" => {
            return Err(crate::Error::UnsupportedOperation(
                "the imaginary unit `j` is not in the WGSL lane (f32 reals only): use the CPU/complex lane".to_string(),
            ))
        }
        E::Var(n) => (wgsl_ident(n)?, 9),
        E::Neg(a) => (format!("-{}", wgsl_at(a, 5)?), 4),
        E::Add(a, b) => (format!("{} + {}", wgsl_at(a, 2)?, wgsl_at(b, 3)?), 2),
        E::Sub(a, b) => (format!("{} - {}", wgsl_at(a, 2)?, wgsl_at(b, 3)?), 2),
        E::Mul(a, b) => (format!("{} * {}", wgsl_at(a, 3)?, wgsl_at(b, 4)?), 3),
        E::Div(a, b) => (format!("{} / {}", wgsl_at(a, 3)?, wgsl_at(b, 4)?), 3),
        // Powers: small integers expand to multiplication (on the GPU pow is a transcendental
        // function; multiplication is both fast and accurate)
        E::Pow(a, b) => match b.as_ref() {
            E::Num(n) if *n == 2.0 => {
                let x = wgsl_at(a, 4)?;
                (format!("{x} * {x}"), 3)
            }
            E::Num(n) if *n == 3.0 => {
                let x = wgsl_at(a, 4)?;
                (format!("{x} * {x} * {x}"), 3)
            }
            E::Num(n) if *n == 0.5 => (format!("sqrt({})", wgsl_at(a, 0)?), 9),
            E::Num(n) if *n == -1.0 => (format!("1.0f / {}", wgsl_at(a, 4)?), 3),
            _ => {
                return Err(crate::Error::UnsupportedOperation("the WGSL lane supports only powers 2 and 3, square roots and -1 (write other exponents in a hand-written shader)".to_string()))
            }
        },
        E::Call(name, args) => {
            let mut vals = Vec::with_capacity(args.len());
            for a in args {
                vals.push(wgsl_at(a, 0)?);
            }
            let mapped = match name.as_str() {
                // Same name, same semantics
                "exp" | "sin" | "cos" | "tan" | "sinh" | "cosh" | "tanh" | "sqrt" | "abs"
                | "min" | "max" | "clamp" => name.clone(),
                // ln/log in this repo are both natural log → WGSL's log
                "ln" | "log" => "log".to_string(),
                other => {
                    return Err(crate::Error::UnsupportedOperation(format!("the WGSL lane does not support the function `{other}` (supported: exp/log/sqrt/sin/cos/tan/sinh/cosh/tanh/abs/min/max/clamp)")))
                }
            };
            (format!("{mapped}({})", vals.join(", ")), 9)
        }
        E::Mat(_) | E::MatPattern(_) => {
            return Err(crate::Error::UnsupportedOperation("the WGSL lane does not emit matrices (assemble them on the CPU side or write the shader by hand)".to_string()))
        }
        E::Transpose(_) => return Err(crate::Error::UnsupportedOperation("the WGSL lane does not support transpose".to_string())),
        E::Deriv(..) | E::Grad(_) | E::Laplacian(_) => {
            // Catalogued: the GPU lane cannot lower a symbolic differential operator.
            return Err(crate::Error::NotEmittable(
                "symbolic differential operators must be differentiated before WGSL emission"
                    .to_string(),
            ));
        }
        E::Sum { .. } | E::Integral { .. } => {
            return Err(crate::Error::UnsupportedOperation("sums and integrals need explicit loops in the WGSL lane: expand them first or write the shader by hand".to_string()))
        }
        E::Indexed { .. } => {
            return Err(crate::Error::UnsupportedOperation("indexed symbols need a tensor context (the WGSL lane accepts scalar formulas only)".to_string()))
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::wgsl::validate_wgsl;
    use crate::{add, div, mul, neg, num, pow, var, Expr};

    #[test]
    fn validate_wgsl_pins_every_structure_rule() {
        // H-2: each emitter self-check gets a kernel text that violates exactly that rule —
        // before these pins, reverting a rule left every emission test green (only the staging
        // gate noticed the reverted construction, not its broken logic).
        let pin = |kernel: &str, fragment: &str| {
            let err = validate_wgsl(kernel).expect_err(kernel);
            assert_eq!(err.code(), "not-emittable", "{err:?}");
            let text = err.render(crate::i18n::Locale::En);
            assert!(text.contains(fragment), "{kernel:?} -> {text}");
        };
        // 1) an extra closing bracket
        pin("fn f() -> f32 { return 1.0f; })", "an extra `)`");
        // 1b) unbalanced opening bracket
        pin("fn f() -> f32 { return 1.0f;", "unbalanced");
        // 2) WGSL has no power operator
        pin("fn f() -> f32 { return 2.0f ^ 3.0f; }", "power operator");
        // 3) a float literal without the `f` suffix (plain and exponent forms)
        pin("fn f() -> f32 { return 2.5; }", "lacks the `f` suffix");
        pin("fn f() -> f32 { return 1e5; }", "lacks the `f` suffix");
        // 4) non-finite constants
        pin("fn f() -> f32 { return inff; }", "non-finite constant");
        // 5) the signature must promise `-> f32`
        pin("fn f() -> f64 { return 1.0f; }", "not `-> f32`");
        // 6) a `return` must end with `;` (the check is per-line: the statement starts a line)
        pin("fn f() -> f32 {\n    return 1.0f\n}", "must end with `;`");
        // 6b) exactly one `return` statement
        pin(
            "fn f() -> f32 { return 1.0f; return 2.0f; }",
            "exactly one `return`",
        );
        // 7) a `fn` definition must exist
        pin("return 1.0f;", "no `fn` definition");
    }

    use alloc::string::{String, ToString};
    use alloc::vec;
    use alloc::vec::Vec;

    fn two_refs() -> Vec<String> {
        vec!["a".to_string(), "b".to_string()]
    }

    #[test]
    fn scalar_kernel_has_f32_literals_and_signature() {
        // f = 0.5·a·b + 1
        let e = add(mul(mul(num(0.5), var("a")), var("b")), num(1.0));
        let w = super::emit_wgsl_with_params(&e, "f", &two_refs()).unwrap();
        assert!(w.contains("fn f(a: f32, b: f32) -> f32 {"), "{w}");
        assert!(w.contains("0.5f"), "{w}");
        assert!(w.contains("1.0f"), "{w}");
        // Brackets balanced (the minimum requirement of the structural gate)
        assert_eq!(w.matches('(').count(), w.matches(')').count(), "{w}");
    }

    #[test]
    fn powers_expand_to_multiplication() {
        let e = pow(var("a"), num(3.0));
        let w = super::emit_wgsl_with_params(&e, "cube", &["a".to_string()]).unwrap();
        assert!(w.contains("a * a * a"), "三次幂要展开成乘法：{w}");
        // Other exponents are rejected explicitly (on the GPU pow is a transcendental function)
        let bad = pow(var("a"), num(4.5));
        assert!(super::emit_wgsl_with_params(&bad, "p", &["a".to_string()]).is_err());
    }

    #[test]
    fn function_name_mapping_and_rejections() {
        // ln → log (WGSL calls the natural logarithm log)
        let e = Expr::Call("ln".into(), vec![var("a")]);
        let w = super::emit_wgsl_with_params(&e, "l", &["a".to_string()]).unwrap();
        assert!(w.contains("log(a)"), "{w}");
        // Unsupported function: report an explicit error
        let bad = Expr::Call("arg".into(), vec![var("a")]);
        assert!(super::emit_wgsl_with_params(&bad, "x", &["a".to_string()]).is_err());
        // Matrix/complex/operator are rejected outright
        let mat = Expr::Mat(vec![vec![num(1.0)]]);
        assert!(super::emit_wgsl_with_params(&mat, "m", &[]).is_err());
    }

    #[test]
    fn keyword_collisions_are_rejected() {
        // `var` is a WGSL keyword: rename or hand-write, never generate a kernel that will not
        // compile
        let e = add(var("var"), num(1.0));
        let err = super::emit_wgsl_with_params(&e, "f", &["var".to_string()]).unwrap_err();
        // Catalogued: the typed variant carries both identifiers, the stable code is exposed,
        // and the English rendering names the collision.
        assert_eq!(err.code(), "wgsl-reserved-word", "{err}");
        match &err {
            crate::Error::WgslReservedWord { name, id } => {
                assert_eq!(name, "var");
                assert_eq!(id, "var");
            }
            other => panic!("expected WgslReservedWord, got {other:?}"),
        }
        let en = err.render(crate::i18n::Locale::En);
        assert!(en.contains("reserved word") && en.contains("`var`"), "{en}");
    }

    #[test]
    fn reserved_word_rendering_keeps_the_two_identifiers_in_order() {
        // `{0}` is the symbol, `{1}` the reserved word: distinct values make a swapped
        // template visible (the collision test above uses `var` for both).
        let e = crate::Error::WgslReservedWord {
            name: "x".into(),
            id: "fn".into(),
        };
        assert_eq!(e.arguments(), vec!["x".to_string(), "fn".to_string()]);
        assert_eq!(
            e.render(crate::i18n::Locale::En),
            "symbol `x` collides with the WGSL reserved word `fn`: rename it or write the shader by hand"
        );
    }

    #[test]
    fn precedence_matches_the_rust_lane() {
        // (a + b) * c must keep the parentheses, otherwise the meaning changes
        let e = mul(add(var("a"), var("b")), var("c"));
        let w =
            super::emit_wgsl_with_params(&e, "f", &["a".into(), "b".into(), "c".into()]).unwrap();
        assert!(w.contains("(a + b) * c"), "{w}");
        // a - (b - c) likewise
        let e = crate::sub(var("a"), crate::sub(var("b"), var("c")));
        let w =
            super::emit_wgsl_with_params(&e, "g", &["a".into(), "b".into(), "c".into()]).unwrap();
        assert!(w.contains("a - (b - c)"), "{w}");
        // The right operand of a division
        let e = div(var("a"), add(var("b"), var("c")));
        let w =
            super::emit_wgsl_with_params(&e, "h", &["a".into(), "b".into(), "c".into()]).unwrap();
        assert!(w.contains("a / (b + c)"), "{w}");
        // Negation
        let e = neg(mul(var("a"), var("b")));
        let w = super::emit_wgsl_with_params(&e, "n", &two_refs()).unwrap();
        assert!(w.contains("-(a * b)"), "{w}");
    }
}
