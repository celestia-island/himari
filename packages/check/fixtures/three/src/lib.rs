// The generated functions' signatures are the formula contract (a derivative function always
// carries every parameter), so this is not dead code.
#![allow(unused_variables, dead_code, clippy::needless_range_loop)]

//! Consumer of the generated artifact: `gen.rs` is produced by build.rs (functions / analytic
//! derivatives / batch entry points / source LaTeX).
//!
//! This layer is the evidence of "formulas turned into legal functions or scaffolding at compile
//! time":
//! 3 `.formula` source files → over a dozen compilable items (plus the source formulas and
//! dimensions inside the docs).
//!
//! Note: a constant must not be named `F` / `R` — the generated function parameters are called
//! `F` / `R`, and an uppercase constant is taken as a **constant pattern** rather than a new binding
//! (rustc E0005). This was a real trap that was hit.

include!("generated.rs");

const FARADAY: f64 = 96485.33212;
const GAS_R: f64 = 8.314462618;

#[test]
fn bv_zero_overpotential_is_zero() {
    // Argument order = declaration order (i_0, eta, alpha, n, F, R, T)
    let v = bv(1.0, 0.0, 0.5, 1.0, FARADAY, GAS_R, 298.15);
    assert!(v.abs() < 1e-12, "eta=0 时 i 必须为 0，得到 {v}");
}

#[test]
fn bv_matches_hand_written_reference() {
    let (i0, eta, alpha, n, t) = (1.7, 0.08, 0.42, 2.0, 313.0);
    let x = alpha * n * FARADAY * eta / (GAS_R * t);
    let y = (1.0 - alpha) * n * FARADAY * eta / (GAS_R * t);
    let want = i0 * (x.exp() - (-y).exp());
    let got = bv(i0, eta, alpha, n, FARADAY, GAS_R, t);
    assert!((got - want).abs() < 1e-9, "got={got} want={want}");
}

#[test]
fn arrhenius_matches_reference() {
    let (a, ea, r, t) = (1.0e13, 5.0e4, GAS_R, 350.0);
    let want = a * (-(ea / (r * t))).exp();
    let got = arrhenius(a, ea, r, t);
    assert!((got - want).abs() / want < 1e-12, "got={got} want={want}");
}

#[test]
fn mech_dimensional_sqrt() {
    // sqrt((F/m)*r) = m/s
    let (f, m, r): (f64, f64, f64) = (10.0, 2.0, 5.0);
    let want = ((f / m) * r).sqrt();
    assert!((mech(f, m, r) - want).abs() < 1e-12);
}

#[test]
fn symbolic_derivative_matches_central_difference() {
    let (i0, eta, alpha, n, t) = (1.3, 0.07, 0.38, 2.0, 303.0);
    let h = 1e-7;
    let numeric = (bv(i0, eta + h, alpha, n, FARADAY, GAS_R, t)
        - bv(i0, eta - h, alpha, n, FARADAY, GAS_R, t))
        / (2.0 * h);
    let analytic = bv__d_eta(i0, eta, alpha, n, FARADAY, GAS_R, t);
    let err = (analytic - numeric).abs() / (1.0 + numeric.abs());
    assert!(err < 1e-6, "解析={analytic} 数值={numeric} 相对误差={err}");
}

#[test]
fn every_derivative_is_finite() {
    let (i0, eta, alpha, n, t) = (1.3, 0.07, 0.38, 2.0, 303.0);
    let ds = [
        bv__d_i_0(i0, eta, alpha, n, FARADAY, GAS_R, t),
        bv__d_eta(i0, eta, alpha, n, FARADAY, GAS_R, t),
        bv__d_alpha(i0, eta, alpha, n, FARADAY, GAS_R, t),
        bv__d_n(i0, eta, alpha, n, FARADAY, GAS_R, t),
        bv__d_F(i0, eta, alpha, n, FARADAY, GAS_R, t),
        bv__d_R(i0, eta, alpha, n, FARADAY, GAS_R, t),
        bv__d_T(i0, eta, alpha, n, FARADAY, GAS_R, t),
    ];
    for (k, d) in ds.iter().enumerate() {
        assert!(d.is_finite(), "第 {k} 个偏导不是有限值：{d}");
    }
}

#[test]
fn batch_matches_scalar() {
    let mut out = [0.0f64; 4];
    let eta = [0.01, 0.02, 0.03, 0.04];
    bv__batch(
        &[1.0; 4],
        &eta,
        &[0.5; 4],
        &[1.0; 4],
        &[FARADAY; 4],
        &[GAS_R; 4],
        &[300.0; 4],
        &mut out,
    );
    for (i, e) in eta.iter().enumerate() {
        let want = bv(1.0, *e, 0.5, 1.0, FARADAY, GAS_R, 300.0);
        assert!((out[i] - want).abs() < 1e-12);
    }
}

#[test]
fn source_latex_is_carried_through() {
    // Source-formula write-back: docs / UI / audit share the same source of truth
    assert!(BV__LATEX.contains("\\alpha"), "{}", BV__LATEX);
    assert!(!MECH__LATEX.is_empty());
}

#[test]
fn tridiag_pattern_ctor_matches_handwritten() {
    // The pattern matrix of heat.formula → the generated constructor vs a hand-written loop
    let k = 0.25f64;
    let n = 6usize;
    let m = heat(k, n);
    let mut want = vec![vec![0.0; n]; n];
    for i in 0..n {
        want[i][i] = 2.0 * k;
        if i + 1 < n {
            want[i][i + 1] = -k;
            want[i + 1][i] = -k;
        }
    }
    assert_eq!(m, want);
    // The far corners must be 0 (the key to the ellipsis semantics: the middle is omitted, the
    // boundary sits at the corners)
    assert_eq!(m[0][n - 1], 0.0);
    assert_eq!(m[n - 1][0], 0.0);
}

// ---------------- the electrochemistry formula set (textbook numeric comparison) ----------------
// The constants reuse FARADAY / GAS_R from the top of the file.

#[test]
fn nernst_matches_textbook() {
    // 25°C, n=1, a_ox/a_red = e (ln=1) → E = E0 - RT/F = E0 - 25.693 mV
    let e = nernst(0.5, GAS_R, 298.15, 1.0, FARADAY, (1.0f64).exp(), 1.0);
    let want = 0.5 - GAS_R * 298.15 / FARADAY;
    assert!((e - want).abs() < 1e-15, "{e} vs {want}");
    let rt_f = GAS_R * 298.15 / FARADAY;
    assert!(
        (rt_f - 0.025693).abs() < 1e-6,
        "RT/F ≈ 25.693 mV，得到 {rt_f}"
    );
}

#[test]
fn tafel_one_decade() {
    // b = 0.120·ln(10) V (=120 mV/dec), i/i₀ = 10 → η = a + 0.120 V
    let b = 0.120 / 10.0f64.ln(); // 120 mV/decade → the ln-form slope = b/ln(10)
    let eta = tafel(0.0, b, 10.0, 1.0);
    assert!((eta - 0.120).abs() < 1e-12, "{eta}");
}

#[test]
fn i0_arrhenius_known_point() {
    // A=1e-9 A/m², Eₐ=50 kJ/mol, T=298.15 K
    let v = i0_arrhenius(1.0e-9, 5.0e4, GAS_R, 298.15);
    let want = 1.0e-9 * (-(5.0e4 / (GAS_R * 298.15))).exp();
    assert!((v - want).abs() / want < 1e-12);
    assert!(
        v > 1.0e-19 && v < 1.0e-17,
        "数量级检查（exp(-Ea/RT)≈1.7e-9）：{v}"
    );
}

#[test]
fn warburg_45_degree_line() {
    // The textbook form Z = σ·f^(-1/2) (σ in Ω·s^(-1/2))
    // f = 100 Hz → f^(-1/2) = 0.1; σ = 10 → Z = 1 Ω
    let z = warburg_re(10.0, 100.0);
    assert!((z - 1.0).abs() < 1e-12, "{z}");
    // f = 1 Hz → Z = σ
    assert!((warburg_re(10.0, 1.0) - 10.0).abs() < 1e-12);
    // The 45° line: halving the frequency raises |Z| by √2
    let r = warburg_re(1.0, 2.0) / warburg_re(1.0, 4.0);
    assert!((r - 2.0f64.sqrt()).abs() < 1e-12, "{r}");
}

#[test]
fn symbolic_sidecars_exist() {
    let cdl = include_str!("../formulas-symbolic/cdl.formula.json");
    assert!(
        cdl.contains("\\partial"),
        "cdl sidecar 应含偏微分 LaTeX：{cdl}"
    );
    let fick = include_str!("../formulas-symbolic/diffusion.formula.json");
    assert!(fick.contains("nabla") || fick.contains("\\nabla"), "{fick}");
}

#[test]
fn bidiag_lower_ctor_matches_handwritten() {
    // The lower bidiagonal pattern of bidiag.formula → the generated function vs a hand-written
    // loop
    let (a, b, n) = (1.5f64, 4.0f64, 5usize);
    let m = bidiag(a, b, n);
    let mut want = vec![vec![0.0; n]; n];
    for i in 0..n {
        want[i][i] = b;
        if i + 1 < n {
            want[i + 1][i] = a;
        }
    }
    assert_eq!(m, want);
    // The upper triangle must be all zeros (the shape semantics of lower bidiagonal)
    for i in 0..n {
        for j in (i + 1)..n {
            assert_eq!(m[i][j], 0.0);
        }
    }
}

#[test]
fn batch_slice_and_const_generic_agree_with_scalar() {
    let n = 64usize;
    let eta: Vec<f64> = (0..n).map(|i| i as f64 * 1e-3).collect();
    let ones = vec![1.0f64; n];
    let half = vec![0.5f64; n];
    let fs = vec![FARADAY; n];
    let rs = vec![GAS_R; n];
    let ts = vec![298.15f64; n];
    let mut out = vec![0.0f64; n];
    bv__batch(&ones, &eta, &half, &ones, &fs, &rs, &ts, &mut out);
    for k in 0..n {
        let want = bv(1.0, eta[k], 0.5, 1.0, FARADAY, GAS_R, 298.15);
        assert!((out[k] - want).abs() < 1e-15, "切片版第 {k} 点");
    }

    // The const-generic form (the length goes into the type) — the numbers must match the scalar
    // version point by point
    let e48: [f64; 48] = std::array::from_fn(|i| i as f64 * 1e-3);
    let o48 = [1.0f64; 48];
    let h48 = [0.5f64; 48];
    let f48 = [FARADAY; 48];
    let r48 = [GAS_R; 48];
    let t48 = [298.15f64; 48];
    let mut out48 = [0.0f64; 48];
    bv__batch_n(&o48, &e48, &h48, &o48, &f48, &r48, &t48, &mut out48);
    for k in 0..48 {
        let want = bv(1.0, e48[k], 0.5, 1.0, FARADAY, GAS_R, 298.15);
        assert_eq!(out48[k], want, "定长版第 {k} 点");
    }
}

#[test]
fn blockdiag_ctor_matches_handwritten() {
    // blockdiag.formula = diag(p·I₂, q·I₂)
    let (p, q, n) = (3.0f64, -2.0f64, 6usize);
    let m = blockdiag(p, q, n);
    let mut want = vec![vec![0.0; n]; n];
    for i in 0..n {
        want[i][i] = if i < n / 2 { p } else { q };
    }
    assert_eq!(m, want);
    // Block structure: the top-left half×half has only p on its diagonal, the bottom-right only q,
    // and the off-block entries are zero
    let half = n / 2;
    assert_eq!(m[0][0], p);
    assert_eq!(m[half][half], q);
    assert_eq!(m[0][half], 0.0);
    assert_eq!(m[half][0], 0.0);
}

#[test]
fn banded_ctor_matches_handwritten() {
    // banded.formula = 5×5 with bandwidth k=2; the generated function holds for any n
    let (v, n) = (2.5f64, 7usize);
    let m = banded(v, n);
    let mut want = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            if i.abs_diff(j) <= 2 {
                want[i][j] = v;
            }
        }
    }
    assert_eq!(m, want);
    // Entries outside the band must be 0 (the k=2 semantics)
    assert_eq!(m[0][3], 0.0);
    assert_eq!(m[6][3], 0.0);
    // Symmetry
    assert_eq!(m[1][4], m[4][1]);
}

// ---------------- artifacts of the complex lane (warburg_complex.formula) ----------------

#[test]
fn complex_warburg_generated_matches_impulse() {
    // Z = σ/(jω)^(1/2): the physical commitment is that the **phase is always -45°** (the slope of
    // the Warburg line), with magnitude |Z| = σ/√ω. The generated code aligns with the analytic form
    // point by point.
    for (sigma, omega) in [(10.0f64, 1.0f64), (2.5, 628.318_530_6), (0.3, 1.0e5)] {
        let (re, im) = Z_c(sigma, omega);
        let mag = re.hypot(im);
        let want = sigma / omega.sqrt();
        assert!(
            (mag - want).abs() / want < 1e-12,
            "σ={sigma} ω={omega}: |Z|={mag} 期望 {want}"
        );
        let phase = im.atan2(re);
        assert!(
            (phase + std::f64::consts::FRAC_PI_4).abs() < 1e-12,
            "σ={sigma} ω={omega}: 相位 {phase} 不是 -45°"
        );
        // The second formula after federation expansion: the magnitude (the imaginary part must be
        // exactly 0; rounding noise must not leak out)
        let (m2, i2) = Z_mag(sigma, omega);
        assert!((m2 - mag).abs() < 1e-12, "Z_mag={m2} vs {mag}");
        assert_eq!(i2, 0.0, "复数的模长必须落在实轴上");
    }
}

#[test]
fn complex_batch_matches_scalar() {
    // Real EIS usage is frequency sweeps: the batch entry point must equal the scalar entry point
    // point by point
    let sigma = [1.0f64, 2.0, 3.0, 0.5];
    let omega = [1.0f64, 4.0, 100.0, 1.0e4];
    let mut out = [(0.0f64, 0.0f64); 4];
    Z_c__batch(&sigma, &omega, &mut out);
    for k in 0..4 {
        assert_eq!(out[k], Z_c(sigma[k], omega[k]), "第 {k} 点");
    }
    // The fixed-length version gives the same values
    let mut out_n = [(0.0f64, 0.0f64); 4];
    Z_c__batch_n(&sigma, &omega, &mut out_n);
    assert_eq!(out_n, out);
}

// ---------------- the value lane of matrix expressions (matalg / conv_matrix) ----------------

#[test]
fn matrix_expression_lane_matches_handwritten() {
    // M = ([1,2;3,4] * [5,6;7,8] + [1,1;1,1])'
    let a = [[1.0f64, 2.0], [3.0, 4.0]];
    let b = [[5.0f64, 6.0], [7.0, 8.0]];
    let mut sum = [[0.0f64; 2]; 2];
    for i in 0..2 {
        for j in 0..2 {
            let mut acc = 0.0;
            for k in 0..2 {
                acc += a[i][k] * b[k][j];
            }
            sum[i][j] = acc + 1.0;
        }
    }
    let want: Vec<Vec<f64>> = (0..2)
        .map(|i| (0..2).map(|j| sum[j][i]).collect())
        .collect();
    assert_eq!(M(), want, "矩阵乘 + 逐元素加 + 转置");

    // S = k * [1,2;3,4] (scalar broadcast multiplication)
    let k = 2.5f64;
    let want_s: Vec<Vec<f64>> = (0..2)
        .map(|i| (0..2).map(|j| k * a[i][j]).collect())
        .collect();
    assert_eq!(S(k), want_s, "标量 × 矩阵");
}

#[test]
fn conv_matrix_is_circulant() {
    // H is a 4th-order circulant matrix: row i is the tap sequence shifted cyclically right by i
    let h = [1.0f64, 2.0, 3.0, 4.0];
    let m = H(h[0], h[1], h[2], h[3]);
    assert_eq!(m.len(), 4);
    for i in 0..4 {
        for j in 0..4 {
            assert_eq!(m[i][j], h[(j + 4 - i) % 4], "H[{i}][{j}]");
        }
    }
    // Physical semantics check: the constant vector [1,1,1,1] through the convolution matrix = the
    // tap sum of each row (=10)
    let total: f64 = h.iter().sum();
    for row in &m {
        let s: f64 = row.iter().sum();
        assert!(
            (s - total).abs() < 1e-12,
            "行和应为抽头和 {total}，得到 {s}"
        );
    }
}

// ---------------- declarations carrying SI prefixes (cdl_current.formula) ----------------

#[test]
fn cdl_current_takes_base_units() {
    // The declarations are µF / mV / kHz, but **arguments are always in SI base units** (the
    // conversion factor is written in the generated doc)
    let ic = i_c(1e-6, 0.05, 1.0e3);
    assert!(
        (ic - 5.0e-5).abs() < 1e-18,
        "1 µF·50 mV·1 kHz = 50 µA，得到 {ic}"
    );
    // The conversion factor must be carried out by a doc comment (docs and code from one source)
    assert!(
        include_str!("generated.rs").contains("1 µF = 0.000001 base units"),
        "the generated artefact must carry the SI conversion hint"
    );
}

// ---------------- per-operator comparison of the complex lane (complex_ops.formula) ----------------
//
// What this group of tests is for: every `__c_*` closure in the generated code must **really** have
// been emitted, and must equal the value layer's `C` implementation point by point. The two
// implementations are written separately, and only a comparison proves they mean the same thing.

#[test]
fn complex_lane_operators_match_value_layer() {
    use himari::C;
    let (u, v) = (0.7f64, -1.3f64);
    let z = C::new(u, v);
    let close = |got: (f64, f64), want: C, tag: &str| {
        assert!(
            (got.0 - want.re).abs() < 1e-12 && (got.1 - want.im).abs() < 1e-12,
            "{tag}: 生成 {got:?} vs 值层 {want:?}"
        );
    };
    // Basic operators
    close(z_abs(u, v), C::real(z.abs()), "abs");
    close(z_arg(u, v), C::real(z.arg()), "arg");
    close(z_conj(u, v), z.conj(), "conj");
    close(z_re(u, v), C::real(z.re), "re");
    close(z_im(u, v), C::real(z.im), "im");
    // Transcendental functions
    close(z_exp(u, v), z.exp(), "exp");
    close(z_ln(u, v), z.ln().unwrap(), "ln");
    close(z_sin(u, v), z.sin(), "sin");
    close(z_cos(u, v), z.cos(), "cos");
    close(z_tan(u, v), z.tan().unwrap(), "tan");
    close(z_sinh(u, v), z.sinh(), "sinh");
    close(z_cosh(u, v), z.cosh(), "cosh");
    close(z_tanh(u, v), z.tanh().unwrap(), "tanh");
    // Complex powers (principal-value branch)
    close(z_sqrt(u, v), z.sqrt(), "z^(1/2)");
}

#[test]
fn complex_generated_sqrt_uses_the_same_scaled_modulus_as_the_value_layer() {
    // re²+im² overflows at 1e200; hypot does not. Both implementations must return finite and
    // identical roots.
    let (re, im) = z_sqrt(1.0e200, 1.0e200);
    let want = himari::C::new(1.0e200, 1.0e200).sqrt();
    assert!(re.is_finite() && im.is_finite(), "generated=({re}, {im})");
    assert!(
        (re - want.re).abs() / want.re < 1e-15,
        "{re} vs {}",
        want.re
    );
    assert!(
        (im - want.im).abs() / want.im < 1e-15,
        "{im} vs {}",
        want.im
    );
}

#[test]
fn complex_lane_negative_real_integer_power_stays_on_axis() {
    // S1 regression: an integer power of a negative real base must be an **exact real**.
    // The polar formula gives garbage imaginary parts from sin(p·π), growing linearly with |p|.
    use himari::C;
    // The imaginary term is `j * v`: only with v = 0 can one verify that the **power itself** has no
    // imaginary part
    for u in [-0.5f64, 0.0, 1.9] {
        let (re, im) = z_pow3(u, 0.0);
        let want = C::real((u - 1.0).powi(3));
        assert_eq!(im, 0.0, "负实底整数幂的虚部必须恰为 0，得到 {im}（u={u}）");
        assert!((re - want.re).abs() < 1e-12, "实部 {re} vs {:.12}", want.re);
    }
    // The value layer gives the same value (both implementations must agree)
    let num = C::new(-1.5, 0.0).pow_real(3.0);
    assert!(num.is_real());
    assert_eq!(num.re, (-1.5f64).powi(3));
}

// ---------------- the Reynolds number (reynolds.formula): the classic Π group example ----------------

#[test]
fn reynolds_number_is_textbook() {
    // Water at 20°C: ρ=1000, v=2 m/s, L=0.1 m, μ=1e-3 Pa·s → Re = 200000 (turbulent)
    let re = Re(1000.0, 2.0, 0.1, 1.0e-3);
    assert!((re - 2.0e5).abs() < 1e-6, "{re}");
    // Linear: doubling the velocity doubles Re
    let r2 = Re(1000.0, 4.0, 0.1, 1.0e-3);
    assert!((r2 / re - 2.0).abs() < 1e-12);
    // The analytic derivative agrees with the hand calculation: ∂Re/∂v = ρL/μ
    assert!((Re__d_v(1000.0, 2.0, 0.1, 1.0e-3) - 1000.0 * 0.1 / 1.0e-3).abs() < 1e-6);
    // The system Jacobian: one row and one column, matching the partial-derivative function
    let j = reynolds__jacobian(1000.0, 2.0, 0.1, 1.0e-3);
    assert_eq!(j.len(), 1);
    assert_eq!(j[0].len(), 4);
    assert!((j[0][1] - Re__d_v(1000.0, 2.0, 0.1, 1.0e-3)).abs() < 1e-12);
}

// ---------------- the FEM element and clamping (fem_element / saturate) ----------------

#[test]
fn fem_element_det_trace_adjoint() {
    // The unit right triangle (0,0) (1,0) (0,1): J = I, det = 1, area = 0.5, trace = 2
    let (x1, x2, x3, y1, y2, y3) = (0.0f64, 1.0, 0.0, 0.0, 0.0, 1.0);
    assert!((area(x1, x2, x3, y1, y2, y3) - 0.5).abs() < 1e-15);
    // The parameter list = **the variables the emitted expression really uses** (once trace is
    // expanded it only depends on x_1 x_2 y_1 y_3)
    assert!((jac_trace(x1, x2, y1, y3) - 2.0).abs() < 1e-15);
    assert_eq!(
        J_adj(x1, x2, x3, y1, y2, y3),
        vec![vec![1.0, 0.0], vec![0.0, 1.0]]
    );
    // The oblique triangle (0,0) (2,0) (0,3): J = diag(2,3) → det = 6, area = 3, trace = 5
    assert!((area(0.0, 2.0, 0.0, 0.0, 0.0, 3.0) - 3.0).abs() < 1e-15);
    assert!((jac_trace(0.0, 2.0, 0.0, 3.0) - 5.0).abs() < 1e-15);
    // The analytic derivative agrees with the closed form: ∂area/∂x_2 = (y_3-y_1)/2,
    // ∂area/∂y_2 = -(x_3-x_1)/2
    let (a, b, c, d, e, f) = (0.3f64, 1.7, -0.4, 0.9, -1.1, 2.3);
    let want_dx2 = (f - d) / 2.0;
    let want_dy2 = -(c - a) / 2.0;
    assert!((area__d_x_2(a, b, c, d, e, f) - want_dx2).abs() < 1e-12);
    assert!((area__d_y_2(a, b, c, d, e, f) - want_dy2).abs() < 1e-12);
    // The system Jacobian: only **scalar residuals** are included (J and J_adj are matrix-valued and
    // do not take part); once det is expanded into an ordinary expression it becomes differentiable
    // → one row each for area and jac_trace
    let j = fem_element__jacobian(a, b, c, d, e, f);
    assert_eq!(j.len(), 2, "两行：area / jac_trace");
    assert_eq!(j[0].len(), 6, "六列：x_1 x_2 x_3 y_1 y_2 y_3");
    assert!((j[0][1] - want_dx2).abs() < 1e-12);
    // trace = (x_2 - x_1) + (y_3 - y_1) → ∂/∂x_1 = -1
    assert!((j[1][0] + 1.0).abs() < 1e-12, "trace 对 x_1 的偏导是 -1");
}

#[test]
fn saturate_semantics_match_rust() {
    // clamp(v, lo, hi) = v.max(lo).min(hi)
    assert_eq!(sat(1.5, 1.0), 1.0, "超上限被钳");
    assert_eq!(sat(-0.5, 1.0), 0.0, "下限是 0（字面量）");
    assert_eq!(sat(0.4, 1.0), 0.4, "区间内原样");
    // With an upper bound of 0 everything is clamped to 0
    assert_eq!(sat(0.9, 0.0), 0.0);
    // Bit-identical to Rust's f64 combinations
    for (v, hi) in [(1.2f64, 0.8f64), (-3.0, 2.0), (0.25, 0.25)] {
        assert_eq!(sat(v, hi), v.max(0.0).min(hi));
    }
    assert_eq!(peak(0.2, 0.7), 0.7);
    assert_eq!(floor_(0.2, 0.7), 0.2);
}

#[test]
fn fem_jacobian_inverse_matches_closed_form() {
    // J = [[a, b], [c, d]] → J⁻¹ = 1/det · [[d, -b], [-c, a]]
    let (x1, x2, x3, y1, y2, y3) = (0.3f64, 1.7, -0.4, 0.9, -1.1, 2.3);
    let (a, b, c, d) = (x2 - x1, x3 - x1, y2 - y1, y3 - y1);
    let det = a * d - b * c;
    let got = J_inv(x1, x2, x3, y1, y2, y3);
    let want = [[d / det, -b / det], [-c / det, a / det]];
    for i in 0..2 {
        for j in 0..2 {
            assert!(
                (got[i][j] - want[i][j]).abs() < 1e-12,
                "J_inv[{i}][{j}] = {} vs {}",
                got[i][j],
                want[i][j]
            );
        }
    }
    // J·J⁻¹ = I (numeric check)
    let j = J(x1, x2, x3, y1, y2, y3);
    for i in 0..2 {
        for k in 0..2 {
            let mut acc = 0.0;
            for m in 0..2 {
                acc += j[i][m] * got[m][k];
            }
            let want_ik = if i == k { 1.0 } else { 0.0 };
            assert!((acc - want_ik).abs() < 1e-12, "J·J⁻¹ [{i}][{k}] = {acc}");
        }
    }
}

// ---------------- the f32 variant (game/SIMD lane) ----------------

#[test]
fn f32_variant_matches_f64_within_single_precision() {
    // Same inputs: the f32 result must agree with the f64 result to within single precision
    let (i0, eta, alpha, n, t) = (1.3f64, 0.07, 0.38, 2.0, 303.0);
    let wide = bv(i0, eta, alpha, n, FARADAY, GAS_R, t);
    let narrow = bv__f32(
        i0 as f32,
        eta as f32,
        alpha as f32,
        n as f32,
        FARADAY as f32,
        GAS_R as f32,
        t as f32,
    );
    let rel = ((narrow as f64) - wide).abs() / wide.abs();
    assert!(
        rel < 1e-5,
        "f32 与 f64 相对误差 {rel}（{narrow} vs {wide}）"
    );
    // Round trip: feed the f32 result back into the f32 version; behavior is stable (no panic,
    // types close over themselves)
    let again = bv__f32(
        i0 as f32,
        eta as f32,
        alpha as f32,
        n as f32,
        FARADAY as f32,
        GAS_R as f32,
        t as f32,
    );
    assert_eq!(again, narrow);
}

#[test]
fn f32_variant_covers_integer_powers_and_clamps() {
    // Integer powers (powi) and min/max must have the same semantics on f32
    let p = p3__f32(1.5f32, 2.0, 0.5);
    let want = 2.0f32 * 1.5 * 1.5 * 1.5 + 0.5 * 1.5;
    assert!((p - want).abs() < 1e-5, "{p} vs {want}");
    let s = sat__f32(1.5f32, 1.0);
    assert_eq!(s, 1.0f32);
}

// ---------------- steady-state linear solving (michaelis.formula) ----------------

#[test]
fn michaelis_steady_state_is_textbook() {
    // Argument order = declaration order (minus the solved-for ES) → (k_on, k_off, k_cat, E, S)
    let (kon, koff, kcat, e, s) = (1.0e6, 1.0e3, 1.0e2, 1.0e-6, 1.0e-3);
    let got = ES_ss(kon, koff, kcat, e, s);
    let want = kon * e * s / (koff + kcat);
    assert!((got - want).abs() / want < 1e-12, "{got} vs {want}");

    // The steady-state rate must agree with the Michaelis–Menten form: v = Vmax·S/(Km+S),
    // where Vmax uses the **total enzyme** E_total = E_free + ES (this step is not an approximation;
    // using the free enzyme as the total only matches when S ≫ Km)
    let km = (koff + kcat) / kon;
    let e_total = e + got;
    let vmax = kcat * e_total;
    let v1 = kcat * got;
    let v2 = vmax * s / (km + s);
    assert!(
        (v1 - v2).abs() / v1 < 1e-10,
        "v {v1} vs {v2}（Km={km}, E_total={e_total}）"
    );

    // The analytic derivative: ∂ES/∂S = k_on·E/(k_off+k_cat) (linear in the substrate)
    let d = ES_ss__d_S(kon, koff, kcat, e, s);
    assert!((d - kon * e / (koff + kcat)).abs() / d < 1e-12);
    // The solved expression does not contain ES → it is not in the parameter list either (the
    // Jacobian has 5 columns)
    let j = michaelis__jacobian(kon, koff, kcat, e, s);
    assert_eq!(j.len(), 1);
    assert_eq!(j[0].len(), 5, "被解出的变量不该出现在参数表里");
}

// ---------------- the 3D tetrahedron element (tet_element.formula) ----------------

#[test]
fn tet_element_volume_and_inverse() {
    // The unit tetrahedron (0,0,0) (1,0,0) (0,1,0) (0,0,1): J = I, det = 1, volume = 1/6
    let (x1, x2, x3, x4) = (0.0f64, 1.0, 0.0, 0.0);
    let (y1, y2, y3, y4) = (0.0f64, 0.0, 1.0, 0.0);
    let (z1, z2, z3, z4) = (0.0f64, 0.0, 0.0, 1.0);
    let v = vol(x1, x2, x3, x4, y1, y2, y3, y4, z1, z2, z3, z4);
    assert!((v - 1.0 / 6.0).abs() < 1e-15, "{v}");
    // The inverse of the identity is still the identity
    let inv = J3_inv(x1, x2, x3, x4, y1, y2, y3, y4, z1, z2, z3, z4);
    for i in 0..3 {
        for j in 0..3 {
            let want = if i == j { 1.0 } else { 0.0 };
            assert!((inv[i][j] - want).abs() < 1e-15, "inv[{i}][{j}]");
        }
    }
    // The oblique tetrahedron: J = [[2,0,0],[0,3,0],[0,0,4]] → det = 24, volume = 4
    let v2 = vol(
        0.0, 2.0, 0.0, 0.0, //
        0.0, 0.0, 3.0, 0.0, //
        0.0, 0.0, 0.0, 4.0,
    );
    assert!((v2 - 4.0).abs() < 1e-14, "{v2}");
    // J·J⁻¹ = I (oblique tetrahedron)
    let j = J3(0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 0.0, 4.0);
    let jinv = J3_inv(0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 0.0, 4.0);
    for i in 0..3 {
        for k in 0..3 {
            let mut acc = 0.0;
            for m in 0..3 {
                acc += j[i][m] * jinv[m][k];
            }
            let want = if i == k { 1.0 } else { 0.0 };
            assert!((acc - want).abs() < 1e-14, "[{i}][{k}] = {acc}");
        }
    }
}

// ---------------- integer powers of matrices (matrix_power.formula) ----------------

#[test]
fn matrix_power_is_folded_through_the_full_lane() {
    // [[a,b],[c,d]]² = [[a²+bc, ab+bd],[ca+dc, cb+d²]]
    let (a, b, c, d) = (1.0f64, 2.0, 3.0, 4.0);
    let got = M2(a, b, c, d);
    let want = [
        [a * a + b * c, a * b + b * d],
        [c * a + d * c, c * b + d * d],
    ];
    for r in 0..2 {
        for k in 0..2 {
            assert!((got[r][k] - want[r][k]).abs() < 1e-12, "[{r}][{k}]");
        }
    }
    // Symbolic bases (runtime variables) work too: the same folding holds for variables
    let (a2, b2, c2, d2) = (0.5, -1.0, 2.0, 0.25);
    let got2 = M2(a2, b2, c2, d2);
    assert!((got2[0][0] - (a2 * a2 + b2 * c2)).abs() < 1e-12);
}

// ---------------- linear equation systems (springs.formula) ----------------

#[test]
fn linear_system_matches_cramers_rule() {
    // Two-mass spring statics:
    //   (k1+k2)x1 − k2·x2 = F1
    //   −k2·x1 + (k2+k3)x2 = F2
    let (k1, k2, k3) = (1.0f64, 2.0, 3.0);
    let (f1, f2) = (10.0f64, 20.0);
    let got = x(k1, k2, k3, f1, f2);
    assert_eq!(got.len(), 2, "解是 2×1 列向量");
    assert_eq!(got[0].len(), 1);

    // Hand calculation by Cramer's rule: det = (k1+k2)(k2+k3) − k2² = 11
    let det = (k1 + k2) * (k2 + k3) - k2 * k2;
    let want1 = (f1 * (k2 + k3) + f2 * k2) / det;
    let want2 = (f2 * (k1 + k2) + f1 * k2) / det;
    assert!(
        (got[0][0] - want1).abs() < 1e-12,
        "x1 = {} want {want1}",
        got[0][0]
    );
    assert!(
        (got[1][0] - want2).abs() < 1e-12,
        "x2 = {} want {want2}",
        got[1][0]
    );

    // Substituting back into the original equations must give zero (an independent criterion for
    // the artifact's correctness that does not depend on my algebra)
    let r1 = (k1 + k2) * got[0][0] - k2 * got[1][0] - f1;
    let r2 = -k2 * got[0][0] + (k2 + k3) * got[1][0] - f2;
    assert!(r1.abs() < 1e-12, "残差 1 = {r1}");
    assert!(r2.abs() < 1e-12, "残差 2 = {r2}");

    // Symmetry check: with k1=k3 and F1=F2 the two nodal displacements must be equal
    let sym = x(2.0, 1.5, 2.0, 7.0, 7.0);
    assert!((sym[0][0] - sym[1][0]).abs() < 1e-12, "对称输入应得对称解");

    // The batch entry point agrees with the scalar one
    let mut out = vec![vec![vec![0.0f64]; 1]; 3];
    x__batch(
        &[1.0, 2.0, 3.0],
        &[2.0, 2.0, 1.0],
        &[3.0, 3.0, 4.0],
        &[10.0, 0.0, 5.0],
        &[20.0, 5.0, 5.0],
        &mut out,
    );
    for (i, (a, b, c, d, e)) in [
        (1.0, 2.0, 3.0, 10.0, 20.0),
        (2.0, 2.0, 3.0, 0.0, 5.0),
        (3.0, 1.0, 4.0, 5.0, 5.0),
    ]
    .iter()
    .enumerate()
    {
        let one = x(*a, *b, *c, *d, *e);
        assert_eq!(out[i], one, "批量第 {i} 行");
    }
}

// ---------------- polynomial root finding (polyroots.formula) ----------------

#[test]
fn complex_generated_quadratic_roots_match_x_squared_plus_j() {
    let got = complex_roots(1.0, 0.0, 0.0);
    let t = 0.5f64.sqrt();
    // x²+j=0, in the original (−b−√D)-first order: −(1−j)/√2, then (1−j)/√2.
    assert!(
        (got[0][0].0 + t).abs() < 1e-12 && (got[0][0].1 - t).abs() < 1e-12,
        "{got:?}"
    );
    assert!(
        (got[1][0].0 - t).abs() < 1e-12 && (got[1][0].1 + t).abs() < 1e-12,
        "{got:?}"
    );
    for (re, im) in [got[0][0], got[1][0]] {
        let z = himari::C::new(re, im);
        let residual = z.mul(z).add(himari::C::I);
        assert!(residual.abs() < 1e-12, "root={z:?}, residual={residual:?}");
    }
}

#[test]
fn roots_quadratic_matches_closed_form_and_residual() {
    // a·x² + b·x + c, taking the polynomial whose roots are 2 and 3: a=1, b=−5, c=6
    let got = roots_quad(1.0, -5.0, 6.0).unwrap();
    assert_eq!(got.len(), 2, "两个根");
    assert_eq!(got[0].len(), 1, "列向量");
    let mut vals = [got[0][0], got[1][0]];
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!((vals[0] - 2.0).abs() < 1e-12, "{vals:?}");
    assert!((vals[1] - 3.0).abs() < 1e-12, "{vals:?}");

    // Substituting the roots back into the original polynomial must give zero (independent of the
    // correctness of the generated formula itself)
    for v in got.iter().flatten() {
        let r = 1.0 * v * v - 5.0 * v + 6.0;
        assert!(r.abs() < 1e-12, "根 {v} 代回残差 {r}");
    }

    // b²≫4ac: the small root must not die from cancellation in −b+√D.
    let separated = roots_quad(1.0, -1.0e8, 1.0).unwrap();
    assert!(
        (separated[0][0] - 1.0e-8).abs() / 1.0e-8 < 1.0e-12,
        "{separated:?}"
    );
    assert!(
        (separated[1][0] - 1.0e8).abs() / 1.0e8 < 1.0e-14,
        "{separated:?}"
    );

    // A double root: x² − 2x + 1 → two 1s
    let double = roots_quad(1.0, -2.0, 1.0).unwrap();
    for v in double.iter().flatten() {
        assert!((v - 1.0).abs() < 1e-12, "重根 {v}");
    }

    // A negative discriminant: the real lane returns a structured error instead of a NaN result.
    let error = roots_quad(1.0, 0.0, 1.0).unwrap_err();
    assert!(matches!(
        error,
        himari::runtime::NumericError::NoRealRoots { .. }
    ));

    // A root at zero: when q=0 fall back to −b/(2a), never 0/0.
    assert_eq!(
        roots_quad(1.0, 0.0, 0.0).unwrap(),
        vec![vec![0.0], vec![0.0]]
    );

    // A linear equation goes through the same entry point: b·x + c = 0 → x = 3
    let lin = roots_lin(2.0, -6.0);
    assert_eq!((lin.len(), lin[0].len()), (1, 1));
    assert!((lin[0][0] - 3.0).abs() < 1e-12);

    // Consistency at scale: for random coefficients both roots satisfy the original equation (for
    // the ones with discriminant ≥ 0)
    for i in 0..64 {
        let t = i as f64;
        let b = -((t % 7.0) + 1.0) * 0.5;
        let c = (t % 5.0) * 0.75;
        if let Ok(roots) = roots_quad(1.0, b, c) {
            for v in roots.iter().flatten() {
                let r = v * v + b * v + c;
                assert!(r.abs() < 1e-9, "b={b} c={c} v={v} 残差 {r}");
            }
        }
    }
}

#[test]
fn mixed_dimension_system_is_accepted_and_numerically_right() {
    // One force balance (N) + one geometric constraint (m): different dimensions between equations
    // are **legal**
    // (review S1-1: this used to be wrongly rejected by going through the generic Mat check)
    let (k1, f1, l) = (2.0f64, 6.0, 5.0);
    let got = mixed(k1, f1, l);
    assert_eq!(got.len(), 2);
    // u = L, v = L − F1/k1
    assert!((got[0][0] - l).abs() < 1e-12, "u = {}", got[0][0]);
    assert!(
        (got[1][0] - (l - f1 / k1)).abs() < 1e-12,
        "v = {}",
        got[1][0]
    );
    // Substitute back into both equations: the force-balance residual and the geometric-constraint
    // residual must both be zero
    let r_force = k1 * (got[0][0] - got[1][0]) - f1;
    let r_geom = got[0][0] - l;
    assert!(r_force.abs() < 1e-12, "力平衡残差 {r_force}");
    assert!(r_geom.abs() < 1e-12, "几何约束残差 {r_geom}");
}

// ---------------- complex equation systems + complex matrix emission (ac_system.formula) ----------------

#[test]
fn complex_system_emits_and_matches_hand_solution() {
    // Z = R + jωL; nodal equations: Z·I1 − V = 0, Z·I1 − R·I2 = 0
    //   → I1 = V/Z, I2 = Z·I1/R = V/R
    let (r, w, l, v) = (10.0f64, 2.0, 3.0, 100.0);
    let got = I(r, w, l, v);
    assert_eq!((got.len(), got[0].len()), (2, 1), "解是 2×1 复数列向量");

    let (zr, zi) = (r, w * l); // Z = 10 + 6j
                               // I1 = V/Z = V·conj(Z)/|Z|²
    let den = zr * zr + zi * zi;
    let want1 = (v * zr / den, -v * zi / den);
    // I2 = V/R
    let want2 = (v / r, 0.0);
    for (got, want) in [(&got[0][0], want1), (&got[1][0], want2)] {
        assert!((got.0 - want.0).abs() < 1e-12, "实部 {got:?} ≠ {want:?}");
        assert!((got.1 - want.1).abs() < 1e-12, "虚部 {got:?} ≠ {want:?}");
    }

    // Substitute back into the original equations (the complex residuals must be zero) —
    // independent of my algebraic derivation
    let z = (zr, zi);
    let mul = |a: (f64, f64), b: (f64, f64)| (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0);
    let zi1 = mul(z, got[0][0]); // Z·I1
    let eq1 = (zi1.0 - v, zi1.1);
    let eq2_left = zi1;
    let eq2_right = mul((r, 0.0), got[1][0]);
    let eq2 = (eq2_left.0 - eq2_right.0, eq2_left.1 - eq2_right.1);
    for (name, res) in [("eq1", eq1), ("eq2", eq2)] {
        assert!(
            res.0.abs() < 1e-12 && res.1.abs() < 1e-12,
            "{name} 残差 {res:?}"
        );
    }

    // A complex matrix expression: every element of [Z, R; -R, Z] carries an imaginary part
    let m = Zm(r, w, l);
    assert_eq!((m.len(), m[0].len()), (2, 2));
    assert_eq!(m[0][0], (r, w * l), "[0][0] = Z");
    assert_eq!(m[1][1], (r, w * l), "[1][1] = Z");
    assert_eq!(m[0][1], (r, 0.0), "[0][1] = R（实）");
    assert_eq!(m[1][0], (-r, 0.0), "[1][0] = −R（实）");
}

#[test]
fn nested_operator_expansion_reaches_the_generated_signature() {
    // A regression net (an independent review proved that reverting the generator's expand_deep to
    // expand_call keeps the whole repo green and the sidecar gate clean — only this **call site**
    // catches it).
    // If a nested solve is not deep-expanded, the generated signature gains an extra x1 parameter and
    // this line does not compile.
    let (k1, f1) = (4.0f64, 10.0);
    let got = zz(k1, f1);
    assert!((got - 2.0 * f1 / k1).abs() < 1e-12, "zz = {got}");

    // Doc comments must show **the formula the user wrote**: the LaTeX constant of `det(J)/2` must
    // contain `det`, not the sum string of the cofactor expansion (M8 mutation: reverting to
    // to_latex(body_expr) turns this red).
    assert!(
        AREA__LATEX.contains("det"),
        "文档 LaTeX 应保留 det 形态，实际：{AREA__LATEX}"
    );
    assert!(
        JAC_TRACE__LATEX.contains("trace"),
        "trace 的文档 LaTeX 也该保留算子形态，实际：{JAC_TRACE__LATEX}"
    );
}

#[test]
fn poly_coefficients_are_low_to_high_and_consistent_with_roots() {
    // The coefficient vector: from low order to high order, a 1×k row vector (this formula's
    // coefficients are constants, so it has no parameters)
    let got = coeffs_quad(); // 2s² − 3s + 1
    assert_eq!(got.len(), 1, "系数是行向量");
    assert_eq!(got[0].len(), 3);
    assert!((got[0][0] - 1.0).abs() < 1e-12, "常数项在第一位");
    assert!((got[0][1] + 3.0).abs() < 1e-12);
    assert!((got[0][2] - 2.0).abs() < 1e-12);

    // Complementary to roots: roots fed back into the coefficient-built form must give zero
    let roots = roots_quad(1.0, -5.0, 6.0).unwrap(); // the roots 2 and 3 of x² − 5x + 6
    for z in roots.iter().flatten() {
        let recon = 1.0 * z * z - 5.0 * z + 6.0;
        assert!(recon.abs() < 1e-9, "根代回重构式应为零，得到 {recon}");
    }
    // The roots of 2s² − 3s + 1 are 0.5 and 1; substituting them back into the expression rebuilt
    // from the **coefficient vector** likewise gives zero
    for z in [0.5f64, 1.0] {
        let recon = got[0][2] * z * z + got[0][1] * z + got[0][0];
        assert!(recon.abs() < 1e-12, "用系数重构：{z} → {recon}");
    }
}
