//! The gates of the WGSL lane: **structural checks + golden text**.
//!
//! There is no naga / tint in the sandbox, so **no WGSL compiler verification is done here** (the
//! README says so as well).
//! What can be guarded automatically:
//!   1. the signature shape (`fn name(param: f32) -> f32`);
//!   2. every float literal carries the `f` suffix (otherwise WGSL gives it AbstractFloat/f64
//!      semantics);
//!   3. brackets balance and identifiers occur only in "params ∪ builtins" (ruling out misspelled
//!      operator names);
//!   4. **verbatim comparison** against the hand-written reference kernel (a golden test: any change
//!      to the emission logic turns it red).

fn emit(src: &str, name: &str) -> String {
    let set = himari_matlab::parse(src).expect("parse");
    let (decls, e) = set.into_single();
    let params = himari::params_in_decl_order(&decls, &himari::simplify(&e));
    himari::wgsl::emit_wgsl_with_params(&himari::simplify(&e), name, &params).expect("emit wgsl")
}

const BUILTINS: &[&str] = &[
    "exp", "log", "sqrt", "sin", "cos", "tan", "sinh", "cosh", "tanh", "abs", "min", "max",
    "clamp", "pow", "f32", "fn", "return",
];

/// Structural check: bracket balancing + identifier whitelist + f32 literal suffix.
fn structural_check(kernel: &str, fname: &str, params: &[&str]) -> Result<(), String> {
    let body: String = kernel
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    if body.matches('(').count() != body.matches(')').count() {
        return Err(format!("括号不配平：\n{kernel}"));
    }
    if body.matches('{').count() != body.matches('}').count() {
        return Err(format!("花括号不配平：\n{kernel}"));
    }
    // Identifiers: start with a letter/underscore, followed by alphanumerics/underscores
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let ident: String = chars[start..i].iter().collect();
            // An `f` suffix right after a number (such as `1.0f`) has already been swallowed by the
            // previous step, so only plain identifiers are handled here
            if ident != fname
                && !params.contains(&ident.as_str())
                && !BUILTINS.contains(&ident.as_str())
            {
                return Err(format!("出现了白名单外的标识符 `{ident}`：\n{kernel}"));
            }
        } else if c.is_ascii_digit() {
            // Number literal: the f suffix is mandatory (collect up to the f)
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let num: String = chars[start..i].iter().collect();
            if chars.get(i) != Some(&'f') {
                return Err(format!("浮点字面量 `{num}` 缺 f 后缀：\n{kernel}"));
            }
            i += 1;
        } else {
            i += 1;
        }
    }
    Ok(())
}

#[test]
fn golden_kernel_for_a_known_formula() {
    // f = 0.5·rho·v² — golden text: any change to the emission logic turns this red, forcing an
    // explicit acknowledgement of the change
    let kernel = emit(
        "decl {\n    rho : density [kg/m^3];\n    v : speed [m/s];\n}\nq = 0.5 * rho * v ^ 2\n",
        "q",
    );
    let want = concat!(
        "// 由 himari 生成（f32 标量车道；经 naga 真编译校验）\n",
        "fn q(rho: f32, v: f32) -> f32 {\n",
        "    return 0.5f * rho * (v * v);\n",
        "}\n"
    );
    assert_eq!(kernel, want);
    structural_check(&kernel, "q", &["rho", "v"]).unwrap();
}

#[test]
fn transcendental_kernel_is_structurally_valid() {
    let kernel = emit(
        "decl {\n    x : gain [1];\n    k : gain [1];\n}\ny = exp(-k * x) + sqrt(x ^ 2) / (1 + x)\n",
        "y",
    );
    structural_check(&kernel, "y", &["x", "k"]).unwrap();
    assert!(kernel.contains("exp(-k * x)"), "{kernel}");
    assert!(kernel.contains("sqrt(x * x)"), "{kernel}");
}

#[test]
fn unsupported_shapes_are_rejected_with_a_reason() {
    // Complex (j) → rejected
    let set = himari_matlab::parse("decl {\n    x : d;\n}\nz = x + j\n").unwrap();
    let (decls, e) = set.into_single();
    let params = himari::params_in_decl_order(&decls, &e);
    let err = himari::wgsl::emit_wgsl_with_params(&e, "z", &params).unwrap_err();
    // The imaginary-unit rejection is catalogued: assert the stable code.
    assert_eq!(err.code(), "unsupported-operation", "{err}");

    // A 4.5 power → rejected (on the GPU pow is a transcendental function; this lane only expands
    // 2/3/0.5/-1)
    let set = himari_matlab::parse("decl {\n    x : d;\n}\ny = x ^ 3\n").unwrap();
    let (_, e) = set.into_single();
    let ok = himari::wgsl::emit_wgsl_with_params(&e, "y", &["x".to_string()]);
    assert!(ok.is_ok(), "3 次幂应可展开");

    let set = himari_matlab::parse("decl {\n    x : d;\n}\ny = x ^ 2.5\n").unwrap();
    let (_, e) = set.into_single();
    assert!(himari::wgsl::emit_wgsl_with_params(&e, "y", &["x".to_string()]).is_err());
}

#[test]
fn static_validator_rejects_malformed_kernels() {
    // Negative control: **bad input must be rejected**. A gate that only proves "good input
    // passes" is a fake gate.
    use himari::wgsl::validate_wgsl;
    let good = "fn f(x: f32) -> f32 {\n    return x * 2.0f;\n}\n";
    assert!(validate_wgsl(good).is_ok(), "好输入应通过");

    // A bare expression function body: WGSL does not support it, so the static gate must reject it
    // (otherwise it is letting through what the real compiler would reject)
    let bare = "fn f(x: f32) -> f32 {\n    x * 2.0f\n}\n";
    assert!(
        validate_wgsl(bare).is_err(),
        "裸表达式函数体应被静态门拒掉：\n{bare}"
    );
    for (bad, why) in [
        // An integer literal (`2`) is abstract-int in WGSL and legal; this lane's style requires the
        // **float** to carry f
        // (measured: naga accepts ones without f — this is a convention, not the spec, see the
        // validate_wgsl comment)
        (
            "fn f(x: f32) -> f32 {\n    return x * 2.5;\n}\n",
            "浮点字面量缺 f 后缀",
        ),
        (
            "fn f(x: f32) -> f32 {\n    return x ^ 2.0f;\n}\n",
            "WGSL 无 ^ 运算符",
        ),
        (
            "fn f(x: f32) -> f32 {\n    return x ** 2.0f;\n}\n",
            "WGSL 无 ** 运算符",
        ),
        (
            "fn f(x: f32) -> f32 {\n    return (x * 2.0f;\n}\n",
            "括号未配平",
        ),
        (
            "fn f(x: f32) -> f32 {\n    return x * 2.0f);\n}\n",
            "多余的右括号",
        ),
        (
            "fn f(x: f32) -> f32 {\n    return x\n}\n",
            "return 语句缺分号",
        ),
        ("fn f(x: f32) -> f64 {\n    x\n}\n", "签名必须是 f32"),
        ("// 只有注释，没有函数\n", "没有 fn 定义"),
        // **The defect shape the real compile gate caught on its first run**: WGSL has no expression
        // function body.
        // The static gate had let it through and had gone on to ban the correct `return` instead.
        (
            "fn f(x: f32) -> f32 {\n    return x * 2.0f;\n    return x;\n}\n",
            "两条 return",
        ),
    ] {
        let err = validate_wgsl(bad).expect_err(&format!("{why} 应被拒绝"));
        assert!(
            !err.render(himari::i18n::Locale::En).is_empty(),
            "{why}: 错误信息不能为空"
        );
    }
}

#[test]
fn every_real_scalar_fixture_survives_the_static_gate() {
    // Emit and self-validate every **real-scalar** formula in the fixture one by one: this gate
    // covers real corpus material, not just two hand-written example lines.
    use himari::wgsl::validate_wgsl;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/three/formulas");
    let mut emitted = 0usize;
    for entry in std::fs::read_dir(&dir).expect("读 fixture 目录") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("formula") {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("读公式");
        let Ok(set) = himari_loader::parse_source(&src, &path.display().to_string()) else {
            continue; // deliberate dialect/semantic errors are not this test's business
        };
        let dims = match himari::dims_of_set(&set) {
            Ok(d) => d,
            Err(_) => continue,
        };
        for item in &set.items {
            let Some(name) = &item.name else { continue };
            let folded = himari::simplify(&item.expr);
            let params = himari::params_in_decl_order(&set.decls, &folded);
            // Only the "emittable" subset must pass self-validation; non-emittable ones must give
            // Err (no panicking allowed)
            match himari::wgsl::emit_wgsl_with_params(&folded, name, &params) {
                Ok(text) => {
                    validate_wgsl(&text).unwrap_or_else(|e| panic!("{name}: {e}\n{text}"));
                    // Complex/matrix formulas must not reach this point
                    assert!(
                        !himari::uses_complex(&folded) && !himari::is_matrix_value(&folded),
                        "`{name}` 是复数或矩阵值，不该被 WGSL 车道接受"
                    );
                    emitted += 1;
                }
                Err(e) => assert!(
                    !e.render(himari::i18n::Locale::En).is_empty(),
                    "拒绝必须带原因"
                ),
            }
            let _ = &dims;
        }
    }
    assert!(
        emitted >= 3,
        "至少应有几条实数标量公式成功发射，实际 {emitted}"
    );
}
