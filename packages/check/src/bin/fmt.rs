//! `himari-fmt`: canonical formatting of `.formula` files.
//!
//! Usage:
//!   himari-fmt <file.formula>            format to stdout
//!   himari-fmt --check <file.formula>    do not rewrite; exit 1 when it differs (drift gate)
//!   himari-fmt --write <file.formula>    format in place
//!   himari-fmt --sidecar <file.formula>  generate <file>.json (latex+mathml, used by hover)
//!   himari-fmt --sidecar-check <file>    sidecar drift gate (exit 1 when it differs)
//!   himari-fmt --diff <file.formula>     do not rewrite; print a unified diff of canonical vs
//!                                        current
//!   himari-fmt --symbols <file.formula>  list decl names and formula names (for editor
//!                                        completion)
//!   himari-fmt --diagnose <file.formula> parse/dimension/shape diagnostics only (all pass = no
//!                                        output)
//!   himari-fmt --wgsl <file.formula>     emit WGSL kernels (f32 scalar lane)
//!
//! Canonical rules (see the "Formatting" section of the README):
//!   - decl is multi-line, one symbol per line;
//!   - formulas are one per line (federation keeps its order);
//!   - matrix columns are aligned: each column width = the widest content in that column + at
//!     least one space;
//!   - ellipses are normalized to `...` (⋯/… become one form).

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut mode = "print";
    let mut path: Option<PathBuf> = None;
    for a in &args {
        match a.as_str() {
            "--check" => mode = "check",
            "--write" => mode = "write",
            "--sidecar" => mode = "sidecar",
            "--sidecar-check" => mode = "sidecar-check",
            "--diff" => mode = "diff",
            // For editor completion: print decl names and formula names one per line for the
            // extension to read
            "--symbols" => mode = "symbols",
            // Diagnostics only (parse + dimension + shape), no output on success; for humans and
            // for the editor
            "--diagnose" => mode = "diagnose",
            // WGSL lane: emit the file's real-scalar formulas as GPU kernels (f32)
            "--wgsl" => mode = "wgsl",
            // Content lock: write the fingerprints of the libraries pulled in by `use` into
            // `<file>.lock` (regenerate after changes)
            "--lock" => mode = "lock",
            // Verify the lock only (no write): for hygiene gates, so a stale lock goes red at once
            "--lock-check" => mode = "lock-check",
            other => {
                if path.is_some() {
                    eprintln!("himari-fmt: 多余参数 {other}");
                    return ExitCode::from(2);
                }
                path = Some(PathBuf::from(other));
            }
        }
    }
    let path = match path {
        Some(p) => p,
        None => {
            eprintln!(
                "himari-fmt 用法: himari-fmt [--check|--write|--diff|--sidecar|--symbols|--diagnose|--wgsl|--lock|--lock-check] <file.formula>"
            );
            return ExitCode::from(2);
        }
    };
    let src = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("himari-fmt: 读不到 {}: {e}", path.display());
            return ExitCode::from(2);
        }
    };
    // matex by default; LaTeX is chosen only explicitly by `%! dialect: latex`, and the dialect
    // is never guessed as a fallback.
    let origin = path.display().to_string();
    let set = match himari_loader::parse_source(&src, &origin) {
        Ok(s) => s,
        Err(e) => {
            // The error text already carries the caret position (rendered uniformly by the loader)
            eprintln!("himari-fmt: {e}");
            return ExitCode::from(1);
        }
    };
    // Comment retention policy: **all standalone comment lines float up to the file header**
    // (order preserved). Comments are documentation for humans and do not enter the IR; putting
    // them back in place would need a line-level position map, which is not worth that complexity
    // for a first version.
    let mut header = String::new();
    for l in src.lines() {
        if l.trim().starts_with('%') {
            header.push_str(l);
            header.push('\n');
        }
    }
    let out = format!("{header}{}", himari::format_set(&set));
    if mode == "lock-check" {
        return match himari_loader::check_lock(&path) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("himari-fmt: {e}");
                ExitCode::from(1)
            }
        };
    }
    // `--lock`: go through the loader only (it collects fingerprints recursively); the entry file
    // itself is not required to render
    if mode == "lock" {
        let text = match himari_loader::lock_text(&path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("himari-fmt: {e}");
                return ExitCode::from(1);
            }
        };
        let lock_path = himari_loader::lock_path_for(&path);
        return match std::fs::write(&lock_path, &text) {
            Ok(()) => {
                println!(
                    "已写入 {}（{} 行）",
                    lock_path.display(),
                    text.lines().count()
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("himari-fmt: 写 {} 失败：{e}", lock_path.display());
                ExitCode::from(2)
            }
        };
    }

    match mode {
        // WGSL: one kernel per **real-scalar** formula (complex/matrix/operators are explicitly
        // rejected with the reason reported)
        "wgsl" => {
            let mut rc = ExitCode::SUCCESS;
            let mut n = 0usize;
            for (k, item) in set.items.iter().enumerate() {
                let folded = himari::simplify(&item.expr);
                let name = item
                    .name
                    .clone()
                    .or_else(|| {
                        path.file_stem()
                            .map(|s| format!("{}__{k}", s.to_string_lossy()))
                    })
                    .unwrap_or_else(|| format!("f{k}"));
                let params = himari::params_in_decl_order(&set.decls, &folded);
                match himari::wgsl::emit_wgsl_with_params(&folded, &name, &params) {
                    Ok(kernel) => {
                        println!("{kernel}");
                        n += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "himari-fmt: {} 的 `{name}` 无法发射 WGSL：{e}",
                            path.display()
                        );
                        rc = ExitCode::from(1);
                    }
                }
            }
            if n == 0 && rc == ExitCode::SUCCESS {
                eprintln!("himari-fmt: 没有可发射的公式");
            }
            rc
        }
        // Completion data: one `<kind> <name>` per line (kind: decl / item)
        "symbols" => {
            for d in &set.decls {
                println!("decl {}", d.name);
            }
            for item in &set.items {
                if let Some(n) = &item.name {
                    println!("item {n}");
                }
            }
            ExitCode::SUCCESS
        }
        // Diagnostics: parse + dimension + shape; all pass = no output
        "diagnose" => {
            let mut bad = 0usize;
            // A file with `use` is diagnosed on the merged set (declarations/formulas from the
            // libraries are checked together)
            let set = match himari_loader::load(&path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(1);
                }
            };
            match himari::dims_of_set(&set) {
                Ok(dims) => {
                    let runtime_shapes = match himari::runtime::shapes_of_set(&set) {
                        Ok(shapes) => shapes,
                        Err(error) => {
                            eprintln!("{}: runtime shape error: {error}", path.display());
                            return ExitCode::from(1);
                        }
                    };
                    for (k, item) in set.items.iter().enumerate() {
                        let folded = himari::simplify(&item.expr);
                        let label = item
                            .name
                            .clone()
                            .unwrap_or_else(|| format!("第 {} 条", k + 1));
                        if let Err(e) =
                            himari::runtime::dim_check_runtime(&folded, &dims, &runtime_shapes)
                        {
                            eprintln!(
                                "{}:{}: {} {}",
                                path.display(),
                                item.line,
                                label,
                                e.render(himari::i18n::Locale::from_env())
                            );
                            bad += 1;
                        }
                        if !himari::runtime::uses_runtime_values(&folded, &runtime_shapes) {
                            if let Err(e) = himari::shape_check(&folded) {
                                eprintln!(
                                    "{}:{}: {} {}",
                                    path.display(),
                                    item.line,
                                    label,
                                    e.render(himari::i18n::Locale::from_env())
                                );
                                bad += 1;
                            }
                        }
                        if himari::uses_complex(&folded)
                            && item.expr != folded
                            && himari::contains_nan_expr(&folded)
                        {
                            eprintln!("{}:{}: {label} 化简后含 NaN", path.display(), item.line);
                            bad += 1;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}: {e}", path.display());
                    bad += 1;
                }
            }
            if bad == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        "print" => {
            print!("{out}");
            ExitCode::SUCCESS
        }
        "diff" => {
            // Read-only preview: print a unified diff (empty output and exit 0 when unchanged)
            print!("{}", unified_diff(&src, &out));
            if src == out {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        "check" => {
            if out == src {
                ExitCode::SUCCESS
            } else {
                eprintln!(
                    "himari-fmt: {} 未按 canonical 格式（--write 修复）",
                    path.display()
                );
                ExitCode::FAILURE
            }
        }
        "sidecar" | "sidecar-check" => {
            // sidecar: each formula's latex + mathml + **dimension + parameters**
            // (shared by VS Code hover / the preview panel / the web, and checked in)
            // The dimension and parameters let hover show "Ω, parameters R/X" instead of only a
            // picture.
            // When the file has `use`, take the merged set from the loader (otherwise symbols from
            // the libraries would show up as `?`).
            // The loader's error must **not** be swallowed here: with `use`, a failed load (for
            // example a stale lock) means the context is incomplete — falling back to "this file
            // only" would write a degraded sidecar (`"dim": "?"`, a parameter list missing the
            // library's symbols) while the gate thinks it was fixed.
            let full = match himari_loader::load(&path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("himari-fmt: {e}");
                    return ExitCode::from(1);
                }
            };
            let dims = himari::dims_of_set(&full).ok();
            let runtime_shapes = himari::runtime::shapes_of_set(&full).ok();
            let federated = himari::federate_inline(&full).ok();
            let mut items = String::from("{\n  \"items\": [\n");
            let n = set.items.len();
            for (k, item) in set.items.iter().enumerate() {
                let latex = himari::to_latex(&item.expr);
                let mathml = himari::to_mathml(&item.expr);
                let name = item.name.clone().unwrap_or_default();
                // Dimension: computed from the federation-expanded expression (shape/dimension
                // are only complete once a later formula's reference to an earlier one is
                // resolved)
                let dim = federated
                    .as_ref()
                    .and_then(|fed| fed.get(k))
                    .map(|it| &it.expr)
                    .or(Some(&item.expr))
                    .and_then(|e| {
                        dims.as_ref()
                            .zip(runtime_shapes.as_ref())
                            .and_then(|(d, shapes)| {
                                himari::runtime::dim_check_runtime(&himari::simplify(e), d, shapes)
                                    .ok()
                            })
                    })
                    .map(|d| d.display())
                    .unwrap_or_else(|| "?".to_string());
                // The parameter list must agree with the **generated function's signature**: the
                // x in `solve(..., x)` has already been solved away and no longer enters the
                // parameter list (the generator also expands before collecting parameters).
                // Collecting parameters from the unexpanded expression would make hover /
                // completion list parameters that do not exist at all.
                let params = federated
                    .as_ref()
                    .and_then(|fed| fed.get(k))
                    .map(|it| {
                        // Deep expansion: the same rule as the generator (solve in a nested
                        // position counts too)
                        let expanded = himari::expand_deep(&it.expr);
                        himari::params_in_decl_order(&full.decls, &expanded)
                    })
                    .unwrap_or_default();
                // JSON escaping (backslashes and quotes inside latex/mathml)
                let esc = |x: &str| {
                    x.replace('\\', "\\\\")
                        .replace('"', "\\\"")
                        .replace('\n', "\\n")
                };
                let param_list = params
                    .iter()
                    .map(|p| format!("\"{}\"", esc(p)))
                    .collect::<Vec<_>>()
                    .join(", ");
                items.push_str(&format!(
                    "    {{ \"name\": \"{}\", \"line\": {}, \"dim\": \"{}\", \"params\": [{}], \"latex\": \"{}\", \"mathml\": \"{}\" }}{}\n",
                    esc(&name),
                    item.line,
                    esc(&dim),
                    param_list,
                    esc(&latex),
                    esc(&mathml),
                    if k + 1 < n { "," } else { "" }
                ));
            }
            items.push_str("  ]\n}\n");
            let side = path.with_extension("formula.json");
            if mode == "sidecar" {
                match std::fs::write(&side, &items) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("himari-fmt: sidecar 写失败 {}: {e}", side.display());
                        ExitCode::from(1)
                    }
                }
            } else if std::fs::read_to_string(&side).ok().as_deref() == Some(items.as_str()) {
                ExitCode::SUCCESS
            } else {
                eprintln!(
                    "himari-fmt: sidecar 漂移：{}（--sidecar 重生成）",
                    side.display()
                );
                ExitCode::FAILURE
            }
        }
        _ => match std::fs::write(&path, &out) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("himari-fmt: 写失败 {}: {e}", path.display());
                ExitCode::from(1)
            }
        },
    }
}

/// A minimal unified diff: trim the common prefix/suffix and replace the middle as one block.
/// In the formatting scenario changes always appear in blocks, so a full Myers diff is not
/// needed.
fn unified_diff(old: &str, new: &str) -> String {
    if old == new {
        return String::new();
    }
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let mut head = 0usize;
    while head < a.len() && head < b.len() && a[head] == b[head] {
        head += 1;
    }
    let mut tail = 0usize;
    while tail < a.len() - head
        && tail < b.len() - head
        && a[a.len() - 1 - tail] == b[b.len() - 1 - tail]
    {
        tail += 1;
    }
    let mut out = String::from("--- current\n+++ canonical\n");
    out.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        head + 1,
        a.len() - head - tail,
        head + 1,
        b.len() - head - tail
    ));
    for l in &a[head..a.len() - tail] {
        out.push('-');
        out.push_str(l);
        out.push('\n');
    }
    for l in &b[head..b.len() - tail] {
        out.push('+');
        out.push_str(l);
        out.push('\n');
    }
    out
}
