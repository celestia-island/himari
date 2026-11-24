// 由 build.rs 生成，勿手改。
// 每条产物的源文件与量纲都写在注释里。

/// 结果量纲：`kg·m^-3`
///
/// ```text
/// \rho_0 \cdot \exp(\frac{-h}{H})
/// ```
///
/// 源：`formulas/entry.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn rho(rho_0: f64, h: f64, H: f64) -> f64 {
        rho_0 * (-h / H).exp()
    }

/// ∂rho/∂rho_0，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn rho__d_rho_0(rho_0: f64, h: f64, H: f64) -> f64 {
        (-h / H).exp()
    }

/// ∂rho/∂h，量纲 `kg·m^-4`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn rho__d_h(rho_0: f64, h: f64, H: f64) -> f64 {
        rho_0 * ((-h / H).exp() * (-H / (H * H)))
    }

/// ∂rho/∂H，量纲 `kg·m^-4`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn rho__d_H(rho_0: f64, h: f64, H: f64) -> f64 {
        let __t0 = -h;
        rho_0 * ((__t0 / H).exp() * (-__t0 / (H * H)))
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn rho__f32(rho_0: f32, h: f32, H: f32) -> f32 {
        rho_0 * (-h / H).exp()
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn rho__batch(rho_0: &[f64], h: &[f64], H: &[f64], out: &mut [f64]) {
    let __n = out.len().min(rho_0.len()).min(h.len()).min(H.len());
    for __row in 0..__n {
        out[__row] = rho(rho_0[__row], h[__row], H[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn rho__batch_n<const __HIMARI_LEN: usize>(rho_0: &[f64; __HIMARI_LEN], h: &[f64; __HIMARI_LEN], H: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = rho(rho_0[__row], h[__row], H[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const RHO__LATEX: &str = "\\rho_0 \\cdot \\exp(\\frac{-h}{H})";

/// 结果量纲：`m·s^-1`
///
/// ```text
/// \sqrt{\frac{2 \cdot m \cdot g}{\rho_0 \cdot \exp(\frac{-h}{H}) \cdot \mathrm{Cd} \cdot A}}
/// ```
///
/// 源：`formulas/entry.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn v_terminal(rho_0: f64, h: f64, H: f64, m: f64, g: f64, Cd: f64, A: f64) -> f64 {
        (2.0 * m * g / (rho_0 * (-h / H).exp() * Cd * A)).sqrt()
    }

/// ∂v_terminal/∂rho_0，量纲 `kg^-1·m^4·s^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn v_terminal__d_rho_0(rho_0: f64, h: f64, H: f64, m: f64, g: f64, Cd: f64, A: f64) -> f64 {
        let __t0 = 2.0 * m;
        let __t1 = __t0 * g;
        let __t2 = -h;
        let __t3 = __t2 / H;
        let __t4 = (__t3).exp();
        let __t5 = rho_0 * __t4;
        let __t6 = __t5 * Cd;
        let __t7 = __t6 * A;
        -(__t1 * (__t4 * Cd * A)) / (__t7 * __t7) / (2.0 * (__t1 / __t7).sqrt())
    }

/// ∂v_terminal/∂h，量纲 `s^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn v_terminal__d_h(rho_0: f64, h: f64, H: f64, m: f64, g: f64, Cd: f64, A: f64) -> f64 {
        let __t0 = 2.0 * m;
        let __t1 = __t0 * g;
        let __t2 = -h;
        let __t3 = __t2 / H;
        let __t4 = (__t3).exp();
        let __t5 = rho_0 * __t4;
        let __t6 = __t5 * Cd;
        let __t7 = __t6 * A;
        -(__t1 * (rho_0 * (__t4 * (-H / (H * H))) * Cd * A)) / (__t7 * __t7) / (2.0 * (__t1 / __t7).sqrt())
    }

/// ∂v_terminal/∂H，量纲 `s^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn v_terminal__d_H(rho_0: f64, h: f64, H: f64, m: f64, g: f64, Cd: f64, A: f64) -> f64 {
        let __t0 = 2.0 * m;
        let __t1 = __t0 * g;
        let __t2 = -h;
        let __t3 = __t2 / H;
        let __t4 = (__t3).exp();
        let __t5 = rho_0 * __t4;
        let __t6 = __t5 * Cd;
        let __t7 = __t6 * A;
        -(__t1 * (rho_0 * (__t4 * (-__t2 / (H * H))) * Cd * A)) / (__t7 * __t7) / (2.0 * (__t1 / __t7).sqrt())
    }

/// ∂v_terminal/∂m，量纲 `kg^-1·m·s^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn v_terminal__d_m(rho_0: f64, h: f64, H: f64, m: f64, g: f64, Cd: f64, A: f64) -> f64 {
        let __t0 = -h;
        let __t1 = __t0 / H;
        let __t2 = (__t1).exp();
        let __t3 = rho_0 * __t2;
        let __t4 = __t3 * Cd;
        let __t5 = __t4 * A;
        2.0 * g * __t5 / (__t5 * __t5) / (2.0 * (2.0 * m * g / __t5).sqrt())
    }

/// ∂v_terminal/∂g，量纲 `s`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn v_terminal__d_g(rho_0: f64, h: f64, H: f64, m: f64, g: f64, Cd: f64, A: f64) -> f64 {
        let __t0 = 2.0 * m;
        let __t1 = -h;
        let __t2 = __t1 / H;
        let __t3 = (__t2).exp();
        let __t4 = rho_0 * __t3;
        let __t5 = __t4 * Cd;
        let __t6 = __t5 * A;
        __t0 * __t6 / (__t6 * __t6) / (2.0 * (__t0 * g / __t6).sqrt())
    }

/// ∂v_terminal/∂Cd，量纲 `m·s^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn v_terminal__d_Cd(rho_0: f64, h: f64, H: f64, m: f64, g: f64, Cd: f64, A: f64) -> f64 {
        let __t0 = 2.0 * m;
        let __t1 = __t0 * g;
        let __t2 = -h;
        let __t3 = __t2 / H;
        let __t4 = (__t3).exp();
        let __t5 = rho_0 * __t4;
        let __t6 = __t5 * Cd;
        let __t7 = __t6 * A;
        -(__t1 * (__t5 * A)) / (__t7 * __t7) / (2.0 * (__t1 / __t7).sqrt())
    }

/// ∂v_terminal/∂A，量纲 `m^-1·s^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn v_terminal__d_A(rho_0: f64, h: f64, H: f64, m: f64, g: f64, Cd: f64, A: f64) -> f64 {
        let __t0 = 2.0 * m;
        let __t1 = __t0 * g;
        let __t2 = -h;
        let __t3 = __t2 / H;
        let __t4 = (__t3).exp();
        let __t5 = rho_0 * __t4;
        let __t6 = __t5 * Cd;
        let __t7 = __t6 * A;
        -(__t1 * __t6) / (__t7 * __t7) / (2.0 * (__t1 / __t7).sqrt())
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn v_terminal__f32(rho_0: f32, h: f32, H: f32, m: f32, g: f32, Cd: f32, A: f32) -> f32 {
        (2.0f32 * m * g / (rho_0 * (-h / H).exp() * Cd * A)).sqrt()
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn v_terminal__batch(rho_0: &[f64], h: &[f64], H: &[f64], m: &[f64], g: &[f64], Cd: &[f64], A: &[f64], out: &mut [f64]) {
    let __n = out.len().min(rho_0.len()).min(h.len()).min(H.len()).min(m.len()).min(g.len()).min(Cd.len()).min(A.len());
    for __row in 0..__n {
        out[__row] = v_terminal(rho_0[__row], h[__row], H[__row], m[__row], g[__row], Cd[__row], A[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn v_terminal__batch_n<const __HIMARI_LEN: usize>(rho_0: &[f64; __HIMARI_LEN], h: &[f64; __HIMARI_LEN], H: &[f64; __HIMARI_LEN], m: &[f64; __HIMARI_LEN], g: &[f64; __HIMARI_LEN], Cd: &[f64; __HIMARI_LEN], A: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = v_terminal(rho_0[__row], h[__row], H[__row], m[__row], g[__row], Cd[__row], A[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const V_TERMINAL__LATEX: &str = "\\sqrt{\\frac{2 \\cdot m \\cdot g}{\\rho_0 \\cdot \\exp(\\frac{-h}{H}) \\cdot \\mathrm{Cd} \\cdot A}}";

/// 结果量纲：`kg·m^-1·s^-2`
///
/// ```text
/// 0.5 \cdot (\rho_0 \cdot \exp(\frac{-h}{H})) \cdot v^{2}
/// ```
///
/// 源：`formulas/entry.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn q_dyn(rho_0: f64, h: f64, H: f64, v: f64) -> f64 {
        0.5 * (rho_0 * (-h / H).exp()) * v * v
    }

/// ∂q_dyn/∂rho_0，量纲 `m^2·s^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn q_dyn__d_rho_0(rho_0: f64, h: f64, H: f64, v: f64) -> f64 {
        0.5 * (-h / H).exp() * (v * v)
    }

/// ∂q_dyn/∂h，量纲 `kg·m^-2·s^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn q_dyn__d_h(rho_0: f64, h: f64, H: f64, v: f64) -> f64 {
        0.5 * (rho_0 * ((-h / H).exp() * (-H / (H * H)))) * (v * v)
    }

/// ∂q_dyn/∂H，量纲 `kg·m^-2·s^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn q_dyn__d_H(rho_0: f64, h: f64, H: f64, v: f64) -> f64 {
        let __t0 = -h;
        0.5 * (rho_0 * ((__t0 / H).exp() * (-__t0 / (H * H)))) * (v * v)
    }

/// ∂q_dyn/∂v，量纲 `kg·m^-2·s^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn q_dyn__d_v(rho_0: f64, h: f64, H: f64, v: f64) -> f64 {
        0.5 * (rho_0 * (-h / H).exp()) * (2.0 * v)
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn q_dyn__f32(rho_0: f32, h: f32, H: f32, v: f32) -> f32 {
        0.5f32 * (rho_0 * (-h / H).exp()) * v * v
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn q_dyn__batch(rho_0: &[f64], h: &[f64], H: &[f64], v: &[f64], out: &mut [f64]) {
    let __n = out.len().min(rho_0.len()).min(h.len()).min(H.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = q_dyn(rho_0[__row], h[__row], H[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn q_dyn__batch_n<const __HIMARI_LEN: usize>(rho_0: &[f64; __HIMARI_LEN], h: &[f64; __HIMARI_LEN], H: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = q_dyn(rho_0[__row], h[__row], H[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Q_DYN__LATEX: &str = "0.5 \\cdot (\\rho_0 \\cdot \\exp(\\frac{-h}{H})) \\cdot v^{2}";

/// 结果量纲：`kg·m·s^-2`
///
/// ```text
/// 0.5 \cdot (\rho_0 \cdot \exp(\frac{-h}{H})) \cdot v^{2} \cdot \mathrm{Cd} \cdot A
/// ```
///
/// 源：`formulas/entry.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn drag(rho_0: f64, h: f64, H: f64, Cd: f64, A: f64, v: f64) -> f64 {
        0.5 * (rho_0 * (-h / H).exp()) * Cd * A * v * v
    }

/// ∂drag/∂rho_0，量纲 `m^4·s^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn drag__d_rho_0(rho_0: f64, h: f64, H: f64, Cd: f64, A: f64, v: f64) -> f64 {
        0.5 * (-h / H).exp() * (v * v) * Cd * A
    }

/// ∂drag/∂h，量纲 `kg·s^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn drag__d_h(rho_0: f64, h: f64, H: f64, Cd: f64, A: f64, v: f64) -> f64 {
        0.5 * (rho_0 * ((-h / H).exp() * (-H / (H * H)))) * (v * v) * Cd * A
    }

/// ∂drag/∂H，量纲 `kg·s^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn drag__d_H(rho_0: f64, h: f64, H: f64, Cd: f64, A: f64, v: f64) -> f64 {
        let __t0 = -h;
        0.5 * (rho_0 * ((__t0 / H).exp() * (-__t0 / (H * H)))) * (v * v) * Cd * A
    }

/// ∂drag/∂Cd，量纲 `kg·m·s^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn drag__d_Cd(rho_0: f64, h: f64, H: f64, Cd: f64, A: f64, v: f64) -> f64 {
        0.5 * (rho_0 * (-h / H).exp()) * (v * v) * A
    }

/// ∂drag/∂A，量纲 `kg·m^-1·s^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn drag__d_A(rho_0: f64, h: f64, H: f64, Cd: f64, A: f64, v: f64) -> f64 {
        0.5 * (rho_0 * (-h / H).exp()) * (v * v) * Cd
    }

/// ∂drag/∂v，量纲 `kg·s^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn drag__d_v(rho_0: f64, h: f64, H: f64, Cd: f64, A: f64, v: f64) -> f64 {
        0.5 * (rho_0 * (-h / H).exp()) * (2.0 * v) * Cd * A
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn drag__f32(rho_0: f32, h: f32, H: f32, Cd: f32, A: f32, v: f32) -> f32 {
        0.5f32 * (rho_0 * (-h / H).exp()) * Cd * A * v * v
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn drag__batch(rho_0: &[f64], h: &[f64], H: &[f64], Cd: &[f64], A: &[f64], v: &[f64], out: &mut [f64]) {
    let __n = out.len().min(rho_0.len()).min(h.len()).min(H.len()).min(Cd.len()).min(A.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = drag(rho_0[__row], h[__row], H[__row], Cd[__row], A[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn drag__batch_n<const __HIMARI_LEN: usize>(rho_0: &[f64; __HIMARI_LEN], h: &[f64; __HIMARI_LEN], H: &[f64; __HIMARI_LEN], Cd: &[f64; __HIMARI_LEN], A: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = drag(rho_0[__row], h[__row], H[__row], Cd[__row], A[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const DRAG__LATEX: &str = "0.5 \\cdot (\\rho_0 \\cdot \\exp(\\frac{-h}{H})) \\cdot v^{2} \\cdot \\mathrm{Cd} \\cdot A";

/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），列 = 参数（声明顺序）。
///
/// 参数 = `rho_0, h, H, m, g, Cd, A, v`；不依赖某参数的行在该列恒为 0。
/// 源：`formulas/entry.formula`
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn entry__jacobian(rho_0: f64, h: f64, H: f64, m: f64, g: f64, Cd: f64, A: f64, v: f64) -> Vec<Vec<f64>> {
    vec![
        vec![rho__d_rho_0(rho_0, h, H), rho__d_h(rho_0, h, H), rho__d_H(rho_0, h, H), 0.0, 0.0, 0.0, 0.0, 0.0],
        vec![v_terminal__d_rho_0(rho_0, h, H, m, g, Cd, A), v_terminal__d_h(rho_0, h, H, m, g, Cd, A), v_terminal__d_H(rho_0, h, H, m, g, Cd, A), v_terminal__d_m(rho_0, h, H, m, g, Cd, A), v_terminal__d_g(rho_0, h, H, m, g, Cd, A), v_terminal__d_Cd(rho_0, h, H, m, g, Cd, A), v_terminal__d_A(rho_0, h, H, m, g, Cd, A), 0.0],
        vec![q_dyn__d_rho_0(rho_0, h, H, v), q_dyn__d_h(rho_0, h, H, v), q_dyn__d_H(rho_0, h, H, v), 0.0, 0.0, 0.0, 0.0, q_dyn__d_v(rho_0, h, H, v)],
        vec![drag__d_rho_0(rho_0, h, H, Cd, A, v), drag__d_h(rho_0, h, H, Cd, A, v), drag__d_H(rho_0, h, H, Cd, A, v), 0.0, 0.0, drag__d_Cd(rho_0, h, H, Cd, A, v), drag__d_A(rho_0, h, H, Cd, A, v), drag__d_v(rho_0, h, H, Cd, A, v)]
    ]
}

/// 结果量纲：`m`
///
/// ```text
/// \operatorname{solve}(k \cdot x - F, x)
/// ```
///
/// 源：`formulas/structures.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn x_static(k: f64, F: f64) -> f64 {
        -(-F / k)
    }

/// ∂x_static/∂k，量纲 `kg^-1·m·s^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn x_static__d_k(k: f64, F: f64) -> f64 {
        -(-(-F) / (k * k))
    }

/// ∂x_static/∂F，量纲 `kg^-1·s^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn x_static__d_F(k: f64, F: f64) -> f64 {
        -(-k / (k * k))
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn x_static__f32(k: f32, F: f32) -> f32 {
        -(-F / k)
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn x_static__batch(k: &[f64], F: &[f64], out: &mut [f64]) {
    let __n = out.len().min(k.len()).min(F.len());
    for __row in 0..__n {
        out[__row] = x_static(k[__row], F[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn x_static__batch_n<const __HIMARI_LEN: usize>(k: &[f64; __HIMARI_LEN], F: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = x_static(k[__row], F[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const X_STATIC__LATEX: &str = "\\operatorname{solve}(k \\cdot x - F, x)";

/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），列 = 参数（声明顺序）。
///
/// 参数 = `k, F`；不依赖某参数的行在该列恒为 0。
/// 源：`formulas/structures.formula`
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn structures__jacobian(k: f64, F: f64) -> Vec<Vec<f64>> {
    vec![
        vec![x_static__d_k(k, F), x_static__d_F(k, F)]
    ]
}

