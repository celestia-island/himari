//! `formulas/*.formula` → generated Rust source (functions / analytic derivatives / f32 variants
//! / batch entry points / system Jacobian / source LaTeX constants).
//!
//! Of the three lanes this is the only one that achieves real file:line diagnostics, cargo-tracked
//! rebuilds and committed-artifact reviewability at the same time — but **consumers should not
//! hand-write 300 lines themselves**. One library function closes over the whole lane:
//!
//! ```ignore
//! fn main() {
//!     println!("cargo:rerun-if-changed=formulas");
//!     if let Err(e) = himari_buildhelper::generate_dir("formulas", "src/generated.rs") {
//!         panic!("{e}");
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;

/// Source comments use a stable in-repo relative path; if an absolute path is not inside the
/// current crate/working directory only the file name is kept, and the `/mnt/...` of some machine
/// is never written into a committed artifact.
fn source_origin(path: &Path) -> String {
    if !path.is_absolute() {
        return path.display().to_string();
    }
    if let Some(base) = std::env::var_os("CARGO_MANIFEST_DIR") {
        if let Ok(relative) = path.strip_prefix(Path::new(&base)) {
            return relative.display().to_string();
        }
    }
    if let Ok(base) = std::env::current_dir() {
        if let Ok(relative) = path.strip_prefix(base) {
            return relative.display().to_string();
        }
    }
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Generate every `.formula` under `dir` into `out` (the committed form: deterministic content,
/// written only when it differs).
///
/// Error messages carry `file:line:column` plus a caret (`FrontError::render_with_source`).
pub fn generate_dir(dir: impl AsRef<Path>, out: impl AsRef<Path>) -> Result<(), String> {
    generate_dir_with(dir, out, false)
}

/// The `no_std` variant: transcendental functions go through `libm::*`, integer powers through
/// `libm::pow` (the consumer needs a `libm` dependency).
pub fn generate_dir_libm(dir: impl AsRef<Path>, out: impl AsRef<Path>) -> Result<(), String> {
    generate_dir_with(dir, out, true)
}

fn generate_dir_with(
    dir: impl AsRef<Path>,
    out: impl AsRef<Path>,
    libm: bool,
) -> Result<(), String> {
    let dir = dir.as_ref();
    let out_path = out.as_ref();
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("读不到 {}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "formula").unwrap_or(false))
        .collect();
    files.sort();
    // All top-level files share one Rust namespace: a duplicate fn name should be an error in
    // build.rs carrying the source location, instead of leaving E0428 for the consumer to hit by
    // luck in generated.rs.
    let mut owners: HashMap<String, (String, usize)> = HashMap::new();

    let mut out =
        String::from("// 由 build.rs 生成，勿手改。\n// 每条产物的源文件与量纲都写在注释里。\n\n");

    for f in &files {
        // Go through this door via the loader: `use "..."` is inlined recursively (cycles,
        // conflicts and missing files all have explicit errors)
        let set = himari_loader::load(f)?;
        let id = f.file_stem().unwrap().to_string_lossy().to_string();
        let origin = source_origin(f);

        // Federation expansion: an earlier named formula is substituted into later references
        // (each formula is self-contained and its parameters are only decl variables)
        let items =
            himari::federate_inline(&set).map_err(|e| format!("{origin}: 联邦展开失败：{e}"))?;
        let decls = &set.decls;

        let dims = himari::dims_of_set(&set).map_err(|e| format!("{origin}: {e}"))?;
        let runtime_shapes = himari::runtime::shapes_of_set(&set)
            .map_err(|error| format!("{origin}: runtime shape analysis failed: {error}"))?;

        // The ingredients of the system Jacobian (rows = formulas, columns = parameters): only
        // "real scalar" entries take part
        let mut jac: Vec<(String, Vec<String>)> = Vec::new();
        let mut jac_ok = true;

        for (idx, item) in items.iter().enumerate() {
            let expr = &item.expr;
            // Function name: the federated name first, falling back to the file name /
            // file name_index
            let fname = match &item.name {
                Some(n) => n.clone(),
                None if items.len() == 1 => id.clone(),
                None => format!("{id}__{idx}"),
            };
            if let Some((previous, line)) = owners.get(&fname) {
                return Err(format!(
                    "{origin}:{}: 公式名 `{fname}` 与 {previous}:{line} 冲突；生成物共用一个 Rust 命名空间，请改名",
                    item.line
                ));
            }
            owners.insert(fname.clone(), (origin.clone(), item.line));
            let is_runtime = himari::runtime::uses_runtime_values(expr, &runtime_shapes);
            let fallible_roots = himari::runtime::is_real_quadratic_formula(expr);
            let fallible = is_runtime || fallible_roots;
            let result_dim = himari::runtime::dim_check_runtime(expr, &dims, &runtime_shapes)
                .map_err(|e| format!("{origin}: {e}"))?;
            if !is_runtime {
                himari::shape_check(expr).map_err(|e| format!("{origin}: {e}"))?;
            }

            // ---- Matrix lane: a pattern matrix must be recognized as a builtin shape, otherwise
            // it blows up right here ----
            if let himari::Expr::MatPattern(p) = expr {
                jac_ok = false;
                let kind = himari::recognize_pattern(p)
                    .map_err(|e| format!("{origin}: 模式矩阵不被支持：{e}"))?;
                let _ = writeln!(out, "/// 源：`{origin}`（量纲 `{}`）", result_dim.display());
                let _ = writeln!(out, "{}", himari::emit_matrix_ctor(&fname, &kind));
                continue;
            }

            // Complex lane: a formula containing the imaginary unit `j` is emitted with
            // `(f64, f64)` = (real part, imaginary part)
            // (the emitter carries only the closures it **uses**, so the artifact still passes
            // clippy -D warnings)
            let cplx = himari::uses_complex(expr);
            if is_runtime && cplx {
                return Err(format!(
                    "{origin}: complex runtime matrix parameters are not supported yet; use real matrices or the existing literal-complex lane"
                ));
            }
            if cplx && libm {
                // The complex emitter's closures are all std methods (hypot/atan2/cosh…), so a
                // no_std build would necessarily fail to compile — report it explicitly instead of
                // silently switching lanes.
                return Err(format!(
                    "{origin}: 复数公式无法在 no_std/libm 车道生成（闭包依赖 std）——\
                     该公式请走 std 车道，或手写 libm 版复数运算"
                ));
            }
            if cplx {
                jac_ok = false;
            }

            // Expandable operators (det/trace/adjoint/inv/**solve**) are expanded first:
            // the parameter table must match the **expression actually emitted** — otherwise
            // `solve(..., ES)` would turn the solved-for ES into a function parameter again
            // ("asking for it yet having to pass it in").
            // **Deep** expansion (not just the root node): in a nested shape such as
            // `2 * solve(...)` the solved-away unknown would stay in the parameter table —
            // generating a function that asks the caller to pass in the quantity being solved for,
            // while that parameter is never used in the function body.
            // Simplify once more after expanding: constant det/inv/solve only reveal foldable
            // arithmetic after expansion. That way a constant singular inverse becomes a
            // non-finite constant and is rejected by the gate below, instead of generating code
            // that only yields inf at runtime; and 0·x inside a symbolic derivative disappears
            // here too.
            let expanded = himari::simplify(&himari::expand_deep(expr));
            let base: &himari::Expr = &expanded;
            // Polynomials are converted to Horner form automatically: a degree-n polynomial drops
            // from 2n-1 multiplications to n, and there is no large power such as `x^n` (numerically
            // more stable). Substituted only when it **really reduces the degree**.
            let hornered = {
                let mut hit = None;
                for pn in himari::vars_of(base) {
                    if let Some(h) = himari::horner_in(base, &pn) {
                        hit = Some(h);
                        break;
                    }
                }
                hit
            };
            let body_expr: &himari::Expr = hornered.as_ref().unwrap_or(base);
            let runtime_emission = if fallible {
                Some(
                    himari::runtime::emit_runtime(expr, &runtime_shapes)
                        .map_err(|error| format!("{origin}: runtime emission failed: {error}"))?,
                )
            } else {
                None
            };

            // A static, expandable concrete matrix → return a matrix (the value lane of matrix
            // expressions).
            // **Decided after expansion**: the matrix nature of `solve([...], [...])` only holds
            // after expansion; deciding on the original expression would emit the column-vector
            // solution as a scalar (types do not line up).
            // A complex matrix uses `Vec<Vec<(f64, f64)>>` (elements are (real part, imaginary
            // part)).
            let matv = !fallible && himari::is_matrix_value(body_expr);
            let ret = match &runtime_emission {
                Some(emission) => match emission.kind {
                    himari::runtime::RuntimeKind::Scalar => {
                        "Result<f64, himari::runtime::NumericError>"
                    }
                    himari::runtime::RuntimeKind::Vector => {
                        "Result<Vec<f64>, himari::runtime::NumericError>"
                    }
                    himari::runtime::RuntimeKind::Matrix => {
                        "Result<Vec<Vec<f64>>, himari::runtime::NumericError>"
                    }
                },
                None => match (cplx, matv) {
                    (true, true) => "Vec<Vec<(f64, f64)>>",
                    (true, false) => "(f64, f64)",
                    (false, true) => "Vec<Vec<f64>>",
                    (false, false) => "f64",
                },
            };
            let out_ty = ret;

            // Emitting a non-finite constant (inf / NaN) would produce the illegal identifiers
            // `inf` / `NaN`: check the **whole tree** (this used to check only the root node, so a
            // nested NaN leaked into the artifact).
            if himari::contains_nonfinite_expr(body_expr) {
                return Err(format!(
                    "{origin}: 表达式含非有限常量（inf/NaN，无法发射成 Rust 字面量）——\
                     检查公式里有没有除以零/开负数之类的无定义运算"
                ));
            }

            let params = himari::params_in_decl_order(decls, body_expr);
            // The Jacobian is assembled from each parameter's analytic derivative: entries whose
            // derivatives are not generated cannot take part
            if fallible {
                jac_ok = false;
            } else if himari::nondiffable(expr).is_empty() {
                if !cplx && !matv {
                    jac.push((fname.clone(), params.clone()));
                }
            } else {
                jac_ok = false;
            }
            let sig = params
                .iter()
                .map(|p| {
                    let kind = decls
                        .iter()
                        .find(|decl| &decl.name == p)
                        .map(|decl| decl.runtime_kind())
                        .unwrap_or(himari::runtime::RuntimeKind::Scalar);
                    match kind {
                        himari::runtime::RuntimeKind::Scalar => format!("{p}: f64"),
                        himari::runtime::RuntimeKind::Vector => format!("{p}: &[f64]"),
                        himari::runtime::RuntimeKind::Matrix => format!("{p}: &[Vec<f64>]"),
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            // The docs show **the formula the user wrote** (not expanded, not Hornerized): the doc
            // of `det(J)/2` should be `\det(J)/2`, not the dozen terms of its cofactor expansion —
            // deep expansion is for the emitter and should not change readability. (A past drift
            // gate caught exactly this regression shape.)
            let latex = himari::to_latex(expr);

            let _ = writeln!(out, "/// 结果量纲：`{}`", result_dim.display());
            // A declaration with an SI prefix only affects readability: **arguments are always in
            // SI base units**.
            // The conversion factor goes into the doc so nobody passes 100 for [µF] expecting
            // 100 µF.
            for p in &params {
                let Some(d) = decls.iter().find(|d| &d.name == p) else {
                    continue;
                };
                let Some(u) = &d.unit else { continue };
                let Ok(spec) = himari::parse_unit_spec(u) else {
                    continue;
                };
                if (spec.scale - 1.0).abs() > 1e-15 {
                    let _ = writeln!(
                        out,
                        "/// ⚠ 参数 `{p}` 声明为 `{u}`：入参是 **SI 基单位**（1 {u} = {} 基单位，调用前请换算）",
                        spec.scale
                    );
                }
            }
            let _ = writeln!(out, "///");
            let _ = writeln!(out, "/// ```text");
            let _ = writeln!(out, "/// {latex}");
            let _ = writeln!(out, "/// ```");
            let _ = writeln!(out, "///");
            let _ = writeln!(out, "/// 源：`{origin}`");
            let _ = writeln!(out, "#[inline]");
            let _ = writeln!(out, "#[allow(non_snake_case, unused_variables)]");
            let _ = writeln!(
                out,
                "#[allow(clippy::too_many_arguments)]\npub fn {fname}({sig}) -> {ret} {}",
                if let Some(emission) = &runtime_emission {
                    format!("{{\n        {}\n    }}", emission.code)
                } else if cplx {
                    himari::emit_block_complex(body_expr)
                } else {
                    if libm {
                        himari::emit_block_libm(body_expr)
                    } else {
                        himari::emit_block(body_expr)
                    }
                }
            );
            let _ = writeln!(out);

            // The complex lane does **not** generate analytic derivatives: `abs`/`arg`/`conj`/
            // `re`/`im` are not complex-analytic operators, so the symbolic chain rule does not hold
            // for them (Wirtinger calculus would be needed).
            // Rather than emitting something that looks compilable but is wrong, simply do not
            // generate it.
            if cplx {
                let _ = writeln!(
                    out,
                    "// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。"
                );
                let _ = writeln!(out);
            }
            if matv {
                let _ = writeln!(
                    out,
                    "// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。"
                );
                let _ = writeln!(out);
            }
            // When the expression contains a **non-symbolizable** operator (min/max/clamp need an
            // indicator function, arg is a step) no analytic derivative is generated: rather than
            // panicking or emitting a wrong value, leave one comment line explaining why.
            let nondiff = himari::nondiffable(expr);
            if !nondiff.is_empty() && !cplx && !matv {
                let _ = writeln!(
                    out,
                    "// 不生成解析导数：表达式含不可符号化的算子（{}）。",
                    nondiff.join("、")
                );
                let _ = writeln!(out);
            }
            for p in params
                .iter()
                .filter(|_| !fallible && !cplx && !matv && nondiff.is_empty())
            {
                // The derivative is taken of **the original form, not the Horner one** (the
                // derivative of the power form is cleaner: differentiating the Horner form gives a
                // string of products that cancel each other)
                let d = himari::simplify(&himari::diff(base, p));
                // When the derivative is not symbolizable (operators/matrices etc.) `diff` returns
                // NaN — **hard failure**: never emit `NaN` as a literal (that would fail to
                // compile and be hard to locate).
                if himari::contains_nan_expr(&d) {
                    let bad = himari::nondiffable(expr);
                    let why = if bad.is_empty() {
                        "（含 NaN 节点）".to_string()
                    } else {
                        format!("（不可符号化：{}）", bad.join("、"))
                    };
                    return Err(format!(
                        "{origin}: 对 `{p}` 的符号导数不可用{why} —— 该公式应走符号层，\
                         或把不可微的算子换成可微表达"
                    ));
                }
                let ddim = himari::dim_check(&d, &dims)
                    .map(|v| v.display())
                    .map_err(|e| format!("{origin}: ∂{fname}/∂{p} 的量纲检查失败：{e}"))?;
                let _ = writeln!(out, "/// ∂{fname}/∂{p}，量纲 `{ddim}`");
                let _ = writeln!(out, "#[inline]");
                let _ = writeln!(out, "#[allow(non_snake_case, unused_variables)]");
                let _ = writeln!(
                    out,
                    "#[allow(clippy::too_many_arguments)]\npub fn {fname}__d_{p}({sig}) -> f64 {}",
                    if libm {
                        himari::emit_block_libm(&d)
                    } else {
                        himari::emit_block(&d)
                    }
                );
                let _ = writeln!(out);
            }

            // The f32 variant: the same formula, literals carrying an `f32` suffix (the emission
            // rules are identical).
            // Game engines / SIMD / audio DSP basically all run in single precision.
            // The f32 variant is skipped in the no_std lane: f32 transcendentals need libm's `*f`
            // family, and this lane only promises f64 + libm (enough for the vast majority of
            // embedded scenarios, without doing it half-way)
            if !fallible && libm && !cplx && !matv {
                let _ = writeln!(
                    out,
                    "// 不生成 f32 变体：no_std 车道只做 f64 + libm（f32 需 libm 的 *f 系列）。"
                );
                let _ = writeln!(out);
            }
            if !fallible && !cplx && !matv && !libm {
                let sig32 = params
                    .iter()
                    .map(|p| format!("{p}: f32"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let _ = writeln!(
                    out,
                    "/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。"
                );
                let _ = writeln!(out, "#[inline]");
                let _ = writeln!(out, "#[allow(non_snake_case, unused_variables)]");
                let _ = writeln!(
                    out,
                    "#[allow(clippy::too_many_arguments)]\npub fn {fname}__f32({sig32}) -> f32 {}",
                    himari::emit_block_f32(body_expr)
                );
                let _ = writeln!(out);
            }

            if !fallible {
                let batch_sig = params
                    .iter()
                    .map(|p| format!("{p}: &[f64]"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let batch_arg_row = params
                    .iter()
                    .map(|p| format!("{p}[__row]"))
                    .collect::<Vec<_>>()
                    .join(", ");
                // Batch entry point: **compute the common length once** and then index.
                // Checking the bound per slice kills vectorization, whereas an upper bound on the
                // common length lets LLVM prove `__row < every len` (measured comparison in bench).
                let mut min_expr = String::from("out.len()");
                for p in &params {
                    min_expr = format!("{min_expr}.min({p}.len())");
                }
                // A parameterless formula (constant matrix): no leading comma may be left in the
                // signature
                let with_out = |head: &str, out_decl: &str| -> String {
                    if head.is_empty() {
                        out_decl.to_string()
                    } else {
                        format!("{head}, {out_decl}")
                    }
                };
                let _ = writeln!(out, "/// 批量入口：公共长度一次算定，供 LLVM 向量化。");
                let _ = writeln!(out, "#[allow(non_snake_case, unused_variables)]");
                let _ = writeln!(
                out,
                "#[inline]\n#[allow(clippy::too_many_arguments)]\npub fn {fname}__batch({}) {{\n    let __n = {min_expr};\n    for __row in 0..__n {{\n        out[__row] = {fname}({batch_arg_row});\n    }}\n}}",
                with_out(&batch_sig, &format!("out: &mut [{out_ty}]"))
            );
                // The const-generic variant: the length goes into a type parameter → all bounds
                // checks are eliminated (the bench comparison is in bin/bench.rs)
                let cg_sig = params
                    .iter()
                    .map(|p| format!("{p}: &[f64; __HIMARI_LEN]"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let _ = writeln!(
                    out,
                    "/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。"
                );
                let _ = writeln!(out, "#[allow(non_snake_case, unused_variables)]");
                let _ = writeln!(
                out,
                "#[inline]\n#[allow(clippy::too_many_arguments)]\npub fn {fname}__batch_n<const __HIMARI_LEN: usize>({}) {{\n    for __row in 0..__HIMARI_LEN {{\n        out[__row] = {fname}({batch_arg_row});\n    }}\n}}",
                with_out(&cg_sig, &format!("out: &mut [{out_ty}; __HIMARI_LEN]"))
            );
                let _ = writeln!(out);
            } else {
                let _ = writeln!(
                    out,
                    "// Runtime matrix/vector formulas do not emit scalar batch or f32 variants."
                );
                let _ = writeln!(out);
            }

            let _ = writeln!(out, "/// 源公式（回写），供文档/UI 复用同一份真源。");
            let _ = writeln!(
                out,
                "pub const {}__LATEX: &str = {latex:?};",
                fname.to_uppercase()
            );
            let _ = writeln!(out);
        }

        // ---- System Jacobian: rows = the formulas, columns = parameters (declaration order) ----
        // This is what implicit solvers / Newton iterations / FEM assembly really want:
        // ∂r_i/∂p_j.
        // When a formula does not depend on a parameter, that cell is **identically zero** (not
        // "unknown").
        // The `no_std` lane does not emit a Jacobian: it returns `Vec<Vec<f64>>` and needs `alloc`.
        // On the embedded side use the per-formula `__d_*` for partials (pure scalars, zero
        // allocation).
        if libm && jac_ok && !jac.is_empty() {
            let _ = writeln!(
                out,
                "// 不生成系统雅可比：它需要 alloc（Vec<Vec<f64>>），本车道只承诺 core + libm。"
            );
            let _ = writeln!(out);
        }
        if jac_ok && !jac.is_empty() && !libm {
            let mut all: Vec<String> = Vec::new();
            for d in decls {
                if jac.iter().any(|(_, ps)| ps.contains(&d.name)) {
                    all.push(d.name.clone());
                }
            }
            for (_, ps) in &jac {
                for pn in ps {
                    if !all.contains(pn) {
                        all.push(pn.clone());
                    }
                }
            }
            if !all.is_empty() {
                let sig = all
                    .iter()
                    .map(|pn| format!("{pn}: f64"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut rows: Vec<String> = Vec::new();
                for (fname, params) in &jac {
                    let call_args = params.join(", ");
                    let mut cells: Vec<String> = Vec::new();
                    for pn in &all {
                        if params.contains(pn) {
                            cells.push(format!("{fname}__d_{pn}({call_args})"));
                        } else {
                            cells.push("0.0".to_string());
                        }
                    }
                    rows.push(format!("vec![{}]", cells.join(", ")));
                }
                let _ = writeln!(
                    out,
                    "/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），\
                     列 = 参数（声明顺序）。"
                );
                let _ = writeln!(out, "///");
                let _ = writeln!(
                    out,
                    "/// 参数 = `{}`；不依赖某参数的行在该列恒为 0。",
                    all.join(", ")
                );
                let _ = writeln!(out, "/// 源：`{origin}`");
                let _ = writeln!(out, "#[allow(non_snake_case, unused_variables)]");
                let _ = writeln!(
                    out,
                    "#[allow(clippy::too_many_arguments)]\npub fn {id}__jacobian({sig}) -> Vec<Vec<f64>> {{\n    vec![\n        {}\n    ]\n}}\n",
                    rows.join(",\n        ")
                );
            }
        }
    }

    // The artifact is committed (the plana bindings form): write an in-repo file rather than
    // OUT_DIR — the content is deterministic and it is written only when it differs (to avoid mtime
    // churn); the drift gate is git diff.
    let current = std::fs::read_to_string(out_path).ok();
    if current.as_deref() != Some(out.as_str()) {
        std::fs::write(out_path, &out)
            .map_err(|e| format!("写 {} 失败：{e}", out_path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(tag: &str, files: &[(&str, &str)]) -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("himari-gen-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let formulas = root.join("formulas");
        std::fs::create_dir_all(&formulas).unwrap();
        for (name, source) in files {
            std::fs::write(formulas.join(name), source).unwrap();
        }
        let out = root.join("generated.rs");
        (formulas, out)
    }

    #[test]
    fn constant_singular_inverse_is_rejected_before_rust_emission() {
        let (formulas, out) = fixture("singular", &[("sing.formula", "K = inv([1, 2; 2, 4])\n")]);
        let err = generate_dir(&formulas, &out).unwrap_err();
        assert!(err.contains("非有限常量"), "{err}");
        assert!(!out.exists(), "失败时不能留下半份生成物");
    }

    #[test]
    fn duplicate_formula_names_across_files_are_rejected_with_origins() {
        let (formulas, out) = fixture(
            "duplicate",
            &[("a.formula", "u = 1\n"), ("b.formula", "u = 2\n")],
        );
        let err = generate_dir(&formulas, &out).unwrap_err();
        assert!(err.contains("公式名 `u`") && err.contains("冲突"), "{err}");
        assert!(
            err.contains("a.formula") && err.contains("b.formula"),
            "{err}"
        );
    }

    #[test]
    fn generated_code_uses_safe_generic_names_horner_and_relative_origins() {
        let (formulas, out) = fixture(
            "codegen",
            &[
                ("n.formula", "N = [1, 2; 3, 4]\n"),
                (
                    "poly.formula",
                    "decl { a : d; b : d; x : d; }\np = a * x ^ 3 + b * x\n",
                ),
            ],
        );
        generate_dir(&formulas, &out).unwrap();
        let generated = std::fs::read_to_string(&out).unwrap();
        assert!(
            generated.contains("pub fn N__batch_n<const __HIMARI_LEN: usize>"),
            "{generated}"
        );
        assert!(!generated.contains("const N: usize"), "{generated}");
        let f32_body = generated
            .split("pub fn p__f32")
            .nth(1)
            .and_then(|s| s.split("/// 批量入口").next())
            .unwrap();
        assert!(f32_body.contains("(a * x * x + b) * x"), "{f32_body}");
        assert!(
            !f32_body.contains("powi"),
            "f32 也必须走 Horner：{f32_body}"
        );
        let absolute = formulas.to_string_lossy().to_string();
        assert!(
            !generated.contains(&absolute),
            "生成物泄漏绝对路径：{generated}"
        );
    }
}
