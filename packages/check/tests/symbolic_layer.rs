//! Symbolic layer: formulas containing ∂/∇ are only parsed, dimension-checked and rendered; no
//! code is generated.
//! Source of truth = fixtures/three/formulas-symbolic/ (cdl, fick2).

const CDL: &str = include_str!("../fixtures/three/formulas-symbolic/cdl.formula");
const FICK: &str = include_str!("../fixtures/three/formulas-symbolic/diffusion.formula");
const EIS: &str = include_str!("../fixtures/three/formulas-symbolic/eis.formula");

#[test]
fn cdl_parses_dims_and_renders() {
    let set = himari_matlab::parse(CDL).unwrap();
    assert_eq!(set.items.len(), 1);
    let dims = himari::dims_of_set(&set).unwrap();
    // i_c = C_dl · ∂E/∂t → (C/V)·(V/s) = A/m²? C/V·V/s = C/s = A ✓
    let (_, e) = set.clone().into_single();
    let d = himari::dim_check(&himari::simplify(&e), &dims).unwrap();
    assert_eq!(d, himari::parse_unit("A").unwrap(), "i_c 量纲应为安培");
    // The LaTeX write-back contains \partial
    let tex = himari::to_latex(&e);
    assert!(tex.contains("\\partial"), "{tex}");
}

#[test]
fn fick_both_sides_same_dimension() {
    let set = himari_matlab::parse(FICK).unwrap();
    assert_eq!(set.items.len(), 2);
    let dims = himari::dims_of_set(&set).unwrap();
    // Both sides: ∂c/∂t = mol/(m³·s); D·∇²c = (m²/s)·mol/m⁵ = mol/(m³·s) ✓
    let (_, lhs) = set.clone().into_single(); // the last one is the rhs
    let lhs_e = himari::simplify(&set.items[0].expr);
    let rhs_e = himari::simplify(&lhs);
    let dl = himari::dim_check(&lhs_e, &dims).unwrap();
    let dr = himari::dim_check(&rhs_e, &dims).unwrap();
    assert_eq!(dl, dr, "Fick 两侧量纲必须一致");
    assert_eq!(dl, himari::parse_unit("mol/m^3/s").unwrap());
    // The MathML rendering contains nabla
    let xml = himari::to_mathml(&rhs_e);
    assert!(xml.contains("<mo>\u{2207}</mo>"), "{xml}");
}

#[test]
fn eis_complex_impedance() {
    let set = himari_matlab::parse(EIS).unwrap();
    let (decls, e) = set.into_single();
    let dims = himari::dims_of(&decls).unwrap();
    // The dimension is still Ω (j is dimensionless)
    assert_eq!(
        himari::dim_check(&himari::simplify(&e), &dims).unwrap(),
        himari::parse_unit("V/A").unwrap()
    );
    // Complex value: R_s = 3, X = -4 → Z = 3 - 4j
    let mut env = std::collections::HashMap::new();
    env.insert("R_s".to_string(), 3.0);
    env.insert("X".to_string(), -4.0);
    match himari::eval_value(&e, &env).unwrap() {
        himari::Value::Scalar(c) => {
            assert_eq!((c.re, c.im), (3.0, -4.0));
        }
        other => panic!("期望标量，得到 {other:?}"),
    }
}

// ---------------- full complex support (matrix value layer + transcendentals + complex powers) ----------------

/// Complex Warburg: Z = σ/(jω)^(1/2) — the **phase** of the 45° line is a physical commitment,
/// not a numerical coincidence: arg Z must be exactly -π/4, and |Z| = σ/√ω.
#[test]
fn complex_warburg_phase_is_45_degrees() {
    use himari::{eval_value, Value};
    // σ [Ω·s^(-1/2)], ω [Hz] → (jω)^(1/2) is s^(-1/2), and dividing by it leaves Ω
    let src = "decl {\n    sigma : warburg_coefficient [V/A*s^(-1/2)];\n    omega : angular_frequency [Hz];\n}\nZ = sigma / (j * omega) ^ (1 / 2)\n";
    let set = himari_matlab::parse(src).unwrap();
    let (decls, e) = set.into_single();
    let dims = himari::dims_of(&decls).unwrap();
    assert_eq!(
        himari::dim_check(&e, &dims).unwrap(),
        himari::parse_unit("V/A").unwrap(),
        "Z 量纲必须是 Ω"
    );
    for (sigma, omega) in [(10.0f64, 1.0f64), (2.5, 628.318_530_6), (0.3, 1.0e5)] {
        let mut env = std::collections::HashMap::new();
        env.insert("sigma".to_string(), sigma);
        env.insert("omega".to_string(), omega);
        match eval_value(&e, &env).unwrap() {
            Value::Scalar(c) => {
                let want_mag = sigma / omega.sqrt();
                assert!(
                    (c.abs() - want_mag).abs() / want_mag < 1e-12,
                    "|Z|={} 期望 {want_mag}",
                    c.abs()
                );
                assert!(
                    (c.arg() + std::f64::consts::FRAC_PI_4).abs() < 1e-12,
                    "arg Z={} 期望 -45°",
                    c.arg()
                );
            }
            other => panic!("期望标量，得到 {other:?}"),
        }
    }
}

/// Complex transcendentals and the four dedicated operators: aligned pointwise with the analytic
/// form.
#[test]
fn complex_transcendentals_match_analytic() {
    use himari::{eval_value, Value};
    let mut env = std::collections::HashMap::new();
    env.insert("x".to_string(), 0.7);
    env.insert("y".to_string(), -1.3);
    // Complex numbers can only be assembled from two real arguments (env is an f64 table;
    // complex values come from the formula, not from the environment)
    const Z: &str = "(x + j * y)";
    let one = |src: &str| -> himari::C {
        let body = src.replace("zz", Z);
        let set = himari_matlab::parse(&format!(
            "decl {{\n    x : real;\n    y : real;\n}}\n{body}\n"
        ))
        .unwrap();
        let (_, e) = set.into_single();
        match eval_value(&e, &env).unwrap() {
            Value::Scalar(c) => c,
            other => panic!("期望标量，得到 {other:?}"),
        }
    };
    let z = himari::C::new(0.7, -1.3);
    let close = |a: himari::C, b: himari::C, tag: &str| {
        assert!(
            (a.re - b.re).abs() < 1e-12 && (a.im - b.im).abs() < 1e-12,
            "{tag}: {a:?} vs {b:?}"
        );
    };
    close(one("conj(zz)"), z.conj(), "conj");
    close(one("abs(zz)"), himari::C::real(z.abs()), "abs");
    close(one("arg(zz)"), himari::C::real(z.arg()), "arg");
    close(one("re(zz)"), himari::C::real(z.re), "re");
    close(one("im(zz)"), himari::C::real(z.im), "im");
    // exp and ln are inverses; sqrt² = itself
    let lz = one("ln(zz)");
    close(one("exp(ln(zz))"), z, "exp∘ln");
    assert!(lz.im.abs() <= std::f64::consts::PI + 1e-12, "主值辐角越界");
    let sq = one("sqrt(zz)");
    close(sq.mul(sq), z, "sqrt²");
    // sin² + cos² = 1 (a complex identity)
    let s2 = one("sin(zz)");
    let c2 = one("cos(zz)");
    close(s2.mul(s2).add(c2.mul(c2)), himari::C::ONE, "sin²+cos²");
    // Complex powers: i^i = e^(-π/2)
    let ipow = one("j ^ j");
    close(
        ipow,
        himari::C::real((-std::f64::consts::FRAC_PI_2).exp()),
        "j^j",
    );
}

/// Complex matrices: elements with imaginary parts, transposition, multiplication and scalar
/// scaling all compute correctly.
#[test]
fn complex_matrix_algebra() {
    use himari::{eval_value, Value, C};
    let src = "decl {\n    r : resistance [V/A];\n}\n[1 + j, 2; 3, 4 - j] * [1, 0; 0, 1]\n";
    let set = himari_matlab::parse(src).unwrap();
    let (_, e) = set.into_single();
    himari::shape_check(&e).unwrap();
    match eval_value(&e, &std::collections::HashMap::new()).unwrap() {
        Value::Matrix(m) => {
            assert_eq!(m.len(), 2);
            assert_eq!(m[0][0], C::new(1.0, 1.0));
            assert_eq!(m[1][1], C::new(4.0, -1.0));
            assert_eq!(m[0][1], C::new(2.0, 0.0));
        }
        other => panic!("期望矩阵，得到 {other:?}"),
    }
    // Transpose + complex (Hermitian conjugation is not transposition: what is verified here is
    // the plain transpose, without conjugating)
    let t = himari_matlab::parse("decl {\n    r : resistance [V/A];\n}\n[1 + j, 2]'\n").unwrap();
    let (_, te) = t.into_single();
    match eval_value(&te, &std::collections::HashMap::new()).unwrap() {
        Value::Matrix(m) => {
            assert_eq!((m.len(), m[0].len()), (2, 1));
            assert_eq!(m[0][0], C::new(1.0, 1.0));
        }
        other => panic!("期望矩阵，得到 {other:?}"),
    }
}

/// `eval` (the scalar entry point) must **report an explicit error** on complex values and must
/// not quietly drop the imaginary part.
#[test]
fn scalar_eval_rejects_complex() {
    let set = himari_matlab::parse("decl {\n    r : resistance [V/A];\n}\n1 + j\n").unwrap();
    let (_, e) = set.into_single();
    let err = himari::eval(&e, &std::collections::HashMap::new()).unwrap_err();
    assert_eq!(err.code(), "complex-eval-result", "{err}");
    let en = err.render(himari::i18n::Locale::En);
    assert!(en.contains("complex") && en.contains("eval_value"), "{en}");
}

#[test]
fn complex_system_solve_works_in_both_value_and_emit_lanes() {
    use himari::{emit_block_complex, eval_value, Value, C};
    // Two impedances in series/parallel: solve for the complex currents (the value layer supports
    // complex equation systems)
    let src = "decl {\n    r : resistance [V/A];\n}\n\
               solve([r * i1 + (i1 - i2) * r - 1, (i2 - i1) * r + i2 * j], [i1, i2])\n";
    let set = himari_matlab::parse(src).unwrap();
    let (_, e) = set.into_single();
    // env is a table of reals (complex values come only from `j`): the complex value is produced by
    // the j inside the expression
    let mut env = std::collections::HashMap::new();
    env.insert("r".to_string(), 2.0f64);
    match eval_value(&e, &env).unwrap() {
        Value::Matrix(m) => {
            assert_eq!((m.len(), m[0].len()), (2, 1), "解是 2×1 列向量");
            // Substitute back into the original equations (the complex residuals must be zero)
            let i1 = m[0][0];
            let i2 = m[1][0];
            let r = C::new(2.0, 0.0);
            let e1 = r.mul(i1).add(i1.sub(i2).mul(r)).sub(C::new(1.0, 0.0));
            let e2 = i2.sub(i1).mul(r).add(i2.mul(C::new(0.0, 1.0)));
            assert!(
                e1.re.abs() < 1e-12 && e1.im.abs() < 1e-12,
                "残差 1 = {e1:?}"
            );
            assert!(
                e2.re.abs() < 1e-12 && e2.im.abs() < 1e-12,
                "残差 2 = {e2:?}"
            );
        }
        other => panic!("期望矩阵，得到 {other:?}"),
    }
    // The emission lane: complex matrices **can be emitted** now (`Vec<Vec<(f64, f64)>>`).
    // The historical contract was "must fail at compile time" — that contract is void along with the
    // complex matrix lane: what the value layer can compute, the emission layer must not claim it
    // cannot do. It must never silently emit a scalar.
    let blk = emit_block_complex(&e);
    assert!(
        !blk.contains("compile_error!"),
        "复数车道的 solve 应可发射，实际：{blk}"
    );
    assert!(
        blk.contains("vec![vec!["),
        "解应是矩阵字面量（2×1 列向量），实际：{blk}"
    );
}

#[test]
fn complex_matrix_arithmetic_emits_literals_not_scalar_helpers() {
    use himari::{emit_block_complex, eval_value, Value, C};
    // Caught in review: in the complex lane `Mat±Mat` / `k*Mat` / `Mat*Mat` were treated as scalars
    // and fed to `__c_add`/`__c_mul` (which only take (f64,f64)), making the artifact
    // `error[E0308]`.
    // Now they are uniformly folded into literals by `flatten_matrix_value` and then emitted cell by
    // cell — the same strategy as the real lane.
    let cases = [
        // (source, description)
        ("[j, 1; 2, j] + [1, 2; 3, 4]", "Mat+Mat"),
        ("2 * [j, 1; 2, j]", "k*Mat"),
        ("[j, 1; 2, j] * [1, 2; 3, 4]", "Mat*Mat"),
        ("-[j, 1; 2, j]", "Neg(Mat)"),
        ("[j, 1; 2, j]'", "纯转置"),
        ("adjoint([j, 1; 2, j])", "共轭转置"),
    ];
    for (src, what) in cases {
        let full = format!("decl {{\n    r : re [V/A];\n}}\nq = {src}\n");
        let set = himari_matlab::parse(&full).unwrap_or_else(|e| panic!("{what} 解析失败：{e:?}"));
        let (_, e) = set.into_single();
        let blk = emit_block_complex(&e);
        assert!(
            !blk.contains("compile_error!"),
            "{what} 不该 compile_error：{blk}"
        );
        assert!(
            blk.contains("vec![vec!["),
            "{what} 应发射成矩阵字面量：{blk}"
        );
        // The key bad shape is "feeding a **matrix literal** to a helper that only takes scalars";
        // calling `__c_add`/`__c_mul` on individual scalars within a cell is the correct usage and
        // must not be banned along with it.
        for bad in [
            "__c_add(vec![",
            "__c_mul(vec![",
            "__c_sub(vec![",
            "__c_neg(vec![",
            "__c_div(vec![",
        ] {
            assert!(!blk.contains(bad), "{what} 把矩阵喂给了标量助手：{blk}");
        }
        // The value layer must yield a matrix (correctness is covered by the existing tests; here we
        // ensure the two layers are isomorphic)
        match eval_value(&e, &std::collections::HashMap::new()).unwrap() {
            Value::Matrix(m) => assert_eq!(m.len(), 2, "{what} 应是 2 行"),
            other => panic!("{what} 值层应是矩阵，得到 {other:?}"),
        }
    }
    // Transposition is a **plain transpose** (no conjugation): cell (0,0) of [j,1;2,j]' is still
    // +j
    let t = himari_matlab::parse("decl {\n    r : re [V/A];\n}\nq = [j, 1; 2, j]'\n").unwrap();
    let (_, te) = t.into_single();
    match eval_value(&te, &std::collections::HashMap::new()).unwrap() {
        Value::Matrix(m) => {
            assert_eq!(m[0][0], C::new(0.0, 1.0), "纯转置不取共轭");
            assert_eq!(m[1][0], C::new(1.0, 0.0), "[1][0] 应为原 [0][1]");
        }
        other => panic!("{other:?}"),
    }
}
