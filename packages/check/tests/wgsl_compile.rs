//! The WGSL **real compile gate**: hand the emitted kernel to naga (wgpu's shader compiler) for a
//! real parse + real validation.
//!
//! Why this layer exists: all the earlier WGSL gates were **static** (bracket balancing / `f`
//! suffix / keyword table / golden text) — without a compiler they can only prove "it looks
//! right". A static gate has a built-in blind spot: **it does not know which rules it missed**.
//! naga is one of the authoritative implementations of the WGSL spec; if it accepts the code, both
//! the syntax and the types hold.
//!
//! Pure CPU, no GPU needed (naga is a compiler front end, not a runtime).
//! It only compiles under `--features wgsl-compile`: naga is a dev-dependency and does not enter
//! the build by default.
//! `required-features` in Cargo.toml guarantees that without the feature no "0 tests" fake-green
//! target is generated.

use naga::valid::{Capabilities, ValidationFlags, Validator};

/// Hand it to naga: parse + validate. Returns a human-readable error (with an offset → no line or
/// column, but enough to locate it).
fn naga_check(src: &str) -> Result<(), String> {
    let module = naga::front::wgsl::parse_str(src).map_err(|e| format!("解析失败：{e}"))?;
    Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&module)
        .map_err(|e| format!("校验失败：{e}"))?;
    Ok(())
}

/// Collect the WGSL text of every **emittable** real-scalar formula in the fixture.
fn collect_emitted() -> Vec<(String, String)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/three/formulas");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("读 fixture 目录") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("formula") {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("读公式");
        let Ok(set) = himari_loader::parse_source(&src, &path.display().to_string()) else {
            continue;
        };
        for item in &set.items {
            let Some(name) = &item.name else { continue };
            let folded = himari::simplify(&item.expr);
            let params = himari::params_in_decl_order(&set.decls, &folded);
            if let Ok(text) = himari::wgsl::emit_wgsl_with_params(&folded, name, &params) {
                out.push((
                    format!("{}::{name}", path.file_name().unwrap().to_string_lossy()),
                    text,
                ));
            }
        }
    }
    out
}

#[test]
fn every_emitted_kernel_passes_the_real_wgsl_compiler() {
    let kernels = collect_emitted();
    assert!(
        !kernels.is_empty(),
        "fixture 里应至少有一条能发射的实数标量公式"
    );
    let mut bad = Vec::new();
    for (name, text) in &kernels {
        if let Err(e) = naga_check(text) {
            bad.push(format!("{name}：{e}\n{text}"));
        }
    }
    assert!(
        bad.is_empty(),
        "以下 kernel 被 naga（真编译器）拒绝 —— 静态门放过了它们：\n{}",
        bad.join("\n---\n")
    );
}

#[test]
fn naga_rejects_what_the_static_gate_also_rejects() {
    // Negative control: the real compiler must reject these. Only when both sides agree does it
    // show the gate is not just talking to itself.
    let bads = [
        (
            "fn f(x: f32) -> f32 {\n    return x ^ 2.0f;\n}\n",
            "幂运算符",
        ),
        (
            "fn f(x: f32) -> f32 {\n    return x * 2.0f;\n}\n}\n",
            "多余右括号",
        ),
        (
            "fn f(x: f32) -> f32 {\n    return x + y;\n}\n",
            "未声明变量",
        ),
        ("fn f(x: f32) -> f64 {\n    return x;\n}\n", "返回类型不符"),
        // **This entry is the defect shape the real compile gate caught on its first run**: a bare
        // expression function body.
        // The static gate not only let it through, it went on to ban the correct `return` instead.
        (
            "fn f(x: f32) -> f32 {\n    x * 2.0f\n}\n",
            "裸表达式函数体（WGSL 不支持）",
        ),
    ];
    // **Recording naga's actual behavior** (not our assumption): `return 2.5;` is **legal** —
    // WGSL's abstract-float literals settle to f32 according to context. So "float literals must
    // carry f" is not a hard rule in WGSL but a **style convention** of this lane (explicit, no
    // reliance on contextual inference, and avoiding being inferred as another width in some
    // positions). I first wrote it as a negative control, mistaking our own convention for the
    // spec — the real compiler corrected that understanding.
    for (src, why) in bads {
        assert!(naga_check(src).is_err(), "naga 竟然接受了：{why}\n{src}");
    }
    // Positive control
    for ok in [
        "fn f(x: f32) -> f32 {\n    return x * 2.0f;\n}\n",
        "fn f(x: f32) -> f32 {\n    let y = x * 2.0f;\n    return y + 1.0f;\n}\n",
        "fn f(x: f32) -> f32 {\n    return pow(x, 2.0f);\n}\n",
        "fn f(x: f32) -> f32 {\n    return sqrt(x);\n}\n",
        "fn f(a: f32, b: f32) -> f32 {\n    return min(a, b) + max(a, b) + abs(a);\n}\n",
        // abstract-float settles to f32 by context (see above)
        "fn f() -> f32 {\n    return 2.5;\n}\n",
    ] {
        assert!(naga_check(ok).is_ok(), "合法 WGSL 被拒：{ok}");
    }
}
