//! `himari-eval`: **evaluates** a `.formula` file (CLI lane).
//!
//! Usage:
//!   himari-eval <file.formula> [--set name=value]... [--groups] [--json] [--roots [--var x]]
//!
//! - Variables come from `--set` (`--set T=298.15`); when a variable is missing, **each formula**
//!   reports which one is missing.
//! - Each formula prints `name = value [dimension]`; complex values are written `3-4j`.
//! - `--groups` additionally prints the Buckingham Π dimensionless groups declared by the file
//!   (the entry point of dimensional analysis: no hand-deriving of combinations such as the
//!   Reynolds number).
//! - `--json` outputs machine-readable results (for editors / scripts).
//! - `--roots` treats each formula as a polynomial and finds its **numeric roots** (any degree,
//!   Durand–Kerner, complex roots supported); `--var x` names the variable (when omitted it is
//!   inferred from the only declared symbol in the formula).
//!   The language's `roots` only does closed forms for degree 1–2; degree three and above goes
//!   through this numeric lane.
//!
//! What this lane is for: a `.formula` is not merely "a source file fed to a code generator"; it
//! is itself an executable formula unit — check the order of magnitude first, then generate code.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!(
        "himari-eval 用法：himari-eval <file.formula> [--set name=value]... [--groups] [--json] [--roots [--var x]]"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut path: Option<PathBuf> = None;
    let mut env: HashMap<String, f64> = HashMap::new();
    let mut groups = false;
    let mut json = false;
    let mut roots_mode = false;
    let mut want_var: Option<String> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--groups" => groups = true,
            "--json" => json = true,
            "--roots" => roots_mode = true,
            "--var" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    eprintln!("himari-eval: --var 后面要跟变量名");
                    return usage();
                };
                want_var = Some(v.trim().to_string());
            }
            "--set" => {
                i += 1;
                let Some(kv) = args.get(i) else {
                    eprintln!("himari-eval: --set 后面要跟 name=value");
                    return usage();
                };
                let Some((k, v)) = kv.split_once('=') else {
                    eprintln!("himari-eval: `{kv}` 不是 name=value");
                    return usage();
                };
                match v.parse::<f64>() {
                    Ok(x) => {
                        env.insert(k.trim().to_string(), x);
                    }
                    Err(_) => {
                        eprintln!("himari-eval: `{v}` 不是数字");
                        return usage();
                    }
                }
            }
            other if other.starts_with("--") => {
                eprintln!("himari-eval: 未知参数 {other}");
                return usage();
            }
            other => {
                if path.is_some() {
                    eprintln!("himari-eval: 多余参数 {other}");
                    return usage();
                }
                path = Some(PathBuf::from(other));
            }
        }
        i += 1;
    }
    let Some(path) = path else {
        return usage();
    };
    let src = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("himari-eval: 读不到 {}: {e}", path.display());
            return ExitCode::from(2);
        }
    };
    let origin = path.display().to_string();
    // Go through the loader: libraries pulled in by `use "..."` are inlined (evaluation needs the
    // full context)
    let set = match himari_loader::load(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("himari-eval: {e}");
            return ExitCode::from(1);
        }
    };
    let _ = (&src, &origin);

    // Federation expansion: a later formula may reference an earlier name directly (same semantics
    // as the build.rs lane)
    let items = match himari::federate_inline(&set) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("himari-eval: {origin}: {e}");
            return ExitCode::from(1);
        }
    };
    let dims = match himari::dims_of_set(&set) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("himari-eval: {origin}: {e}");
            return ExitCode::from(1);
        }
    };

    // ---- `--roots`: the numeric root-finding lane (any degree, complex roots included) ----
    if roots_mode {
        let mut rc = ExitCode::SUCCESS;
        for (k, item) in items.iter().enumerate() {
            let label = item
                .name
                .clone()
                .unwrap_or_else(|| format!("第 {} 条", k + 1));
            // The variable: explicit --var, otherwise the **only** declared symbol in the formula
            let var = match &want_var {
                Some(v) => v.clone(),
                None => {
                    let mut cands: Vec<String> = himari::vars_of(&item.expr)
                        .into_iter()
                        .filter(|v| dims.contains_key(v))
                        .collect();
                    cands.sort();
                    cands.dedup();
                    match cands.len() {
                        1 => cands.remove(0),
                        0 => {
                            eprintln!(
                                "himari-eval: {label}: 找不到可求根的自变量（用 --var 指定）"
                            );
                            rc = ExitCode::from(1);
                            continue;
                        }
                        _ => {
                            eprintln!(
                                "himari-eval: {label}: 有多个候选自变量 {}（用 --var 指定）",
                                cands.join("、")
                            );
                            rc = ExitCode::from(1);
                            continue;
                        }
                    }
                }
            };
            let Some(coeffs) = himari::poly_coeffs(&item.expr, &var) else {
                eprintln!("himari-eval: {label}: 不是关于 `{var}` 的多项式");
                rc = ExitCode::from(1);
                continue;
            };
            // The coefficients must be computable as numbers in the current environment
            let mut num: Vec<f64> = Vec::with_capacity(coeffs.len());
            let mut bad = None;
            for c in &coeffs {
                match himari::eval(&himari::simplify(c), &env) {
                    Ok(v) if v.is_finite() => num.push(v),
                    Ok(v) => {
                        bad = Some(format!("系数非有限（{v}）"));
                        break;
                    }
                    Err(e) => {
                        bad = Some(e.render(himari::i18n::Locale::from_env()));
                        break;
                    }
                }
            }
            if let Some(e) = bad {
                eprintln!("himari-eval: {label}: 系数无法求值（{e}）—— 用 --set 给齐参数后重试");
                rc = ExitCode::from(1);
                continue;
            }
            match himari::numeric_roots(&num) {
                Ok(rs) => {
                    let text: Vec<String> = rs.iter().map(|c| fmt_value(*c)).collect();
                    let dim = dims
                        .get(&var)
                        .map(|d| d.display())
                        .unwrap_or_else(|| "?".to_string());
                    println!(
                        "{} = {}  [{}]（{} 次，{} 个根）",
                        label,
                        text.join(", "),
                        dim,
                        num.len() - 1,
                        rs.len()
                    );
                }
                Err(e) => {
                    eprintln!("himari-eval: {label}: {e}");
                    rc = ExitCode::from(1);
                }
            }
        }
        return rc;
    }

    let mut out: Vec<(String, String, String)> = Vec::new();
    let mut rc = ExitCode::SUCCESS;
    for (k, item) in items.iter().enumerate() {
        let folded = himari::simplify(&item.expr);
        let label = item
            .name
            .clone()
            .unwrap_or_else(|| format!("第 {} 条", k + 1));
        let dim = himari::dim_check(&folded, &dims)
            .map(|d| d.display())
            .unwrap_or_else(|e| format!("<量纲错: {e}>"));
        match himari::eval_value(&folded, &env) {
            Ok(v) => {
                let text = match v.as_scalar() {
                    Some(c) => fmt_value(c),
                    None => {
                        // A matrix value is printed row by row as **numbers**. Reporting only the
                        // shape is useless for solver-like formulas (an equation system yields a
                        // column vector, and those numbers are exactly what the user wants).
                        match v {
                            himari::Value::Matrix(m) => {
                                if m.len() == 1 || (m.first().map(|r| r.len()) == Some(1)) {
                                    // Single row / single column: compact into [a, b, c]
                                    let vals: Vec<String> = m
                                        .iter()
                                        .flat_map(|r| r.iter().map(|c| fmt_value(*c)))
                                        .collect();
                                    format!("[{}]", vals.join(", "))
                                } else {
                                    // A general matrix: one group per row
                                    let rows: Vec<String> = m
                                        .iter()
                                        .map(|r| {
                                            let vals: Vec<String> =
                                                r.iter().map(|c| fmt_value(*c)).collect();
                                            format!("[{}]", vals.join(", "))
                                        })
                                        .collect();
                                    rows.join("; ")
                                }
                            }
                            _ => "<未知>".to_string(),
                        }
                    }
                };
                out.push((label, text, dim));
            }
            Err(e) => {
                rc = ExitCode::from(1);
                out.push((label, format!("<无法求值: {e}>"), dim));
            }
        }
    }

    if json {
        println!("{{");
        println!("  \"file\": {:?},", origin);
        println!("  \"items\": [");
        for (k, (name, value, dim)) in out.iter().enumerate() {
            let comma = if k + 1 < out.len() { "," } else { "" };
            println!(
                "    {{ \"name\": {:?}, \"value\": {:?}, \"dim\": {:?} }}{comma}",
                name, value, dim
            );
        }
        println!("  ]");
        println!("}}");
    } else {
        for (name, value, dim) in &out {
            println!("{name} = {value}  [{dim}]");
        }
    }

    if groups {
        // Π groups: use the symbols declared in the file (dimensionless_groups only takes
        // "name + dimension")
        let vars: Vec<(String, himari::Dim)> = set
            .decls
            .iter()
            .filter_map(|d| {
                himari::parse_unit(d.unit.as_deref().unwrap_or("1"))
                    .ok()
                    .map(|dim| (d.name.clone(), dim))
            })
            .collect();
        if vars.is_empty() {
            eprintln!("himari-eval: 没有带量纲的声明，算不出 Π 群");
            rc = ExitCode::from(1);
        } else {
            match himari::dimensionless_groups(&vars) {
                Ok(gs) => {
                    let names: Vec<String> = vars.iter().map(|(n, _)| n.clone()).collect();
                    println!();
                    println!("无量纲群（Buckingham Π，{} 个）", gs.len());
                    for (k, g) in gs.iter().enumerate() {
                        println!("  Π{} = {}", k + 1, himari::pi_text(&names, g));
                    }
                }
                Err(e) => {
                    eprintln!("himari-eval: Π 群失败：{e}");
                    rc = ExitCode::from(1);
                }
            }
        }
    }
    rc
}

/// The display form of a value: reals as they are, complex numbers as `a+bj` / `a-bj`.
fn fmt_value(c: himari::C) -> String {
    if c.is_real() {
        return himari::fmt_num(c.re);
    }
    let sign = if c.im < 0.0 { '-' } else { '+' };
    format!(
        "{}{}{}j",
        himari::fmt_num(c.re),
        sign,
        himari::fmt_num(c.im.abs())
    )
}

#[cfg(test)]
mod tests {
    use super::fmt_value;
    use himari::C;

    #[test]
    fn value_formatting() {
        assert_eq!(fmt_value(C::real(3.0)), "3");
        assert_eq!(fmt_value(C::real(-2.5)), "-2.5");
        assert_eq!(fmt_value(C::new(3.0, -4.0)), "3-4j");
        assert_eq!(fmt_value(C::new(0.0, 1.0)), "0+1j");
        // A complex number on the real axis must not be written `+0j`
        assert_eq!(fmt_value(C::new(5.0, 0.0)), "5");
    }
}
