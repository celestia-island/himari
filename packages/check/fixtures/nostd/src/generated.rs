// 由 build.rs 生成，勿手改。
// 每条产物的源文件与量纲都写在注释里。

/// 结果量纲：`kg·m^-3`
///
/// ```text
/// \rho_0 \cdot \exp(\frac{-h}{H})
/// ```
///
/// 源：`formulas/drag.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn rho(rho_0: f64, h: f64, H: f64) -> f64 {
        rho_0 * libm::exp(-h / H)
    }

/// ∂rho/∂rho_0，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn rho__d_rho_0(rho_0: f64, h: f64, H: f64) -> f64 {
        libm::exp(-h / H)
    }

/// ∂rho/∂h，量纲 `kg·m^-4`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn rho__d_h(rho_0: f64, h: f64, H: f64) -> f64 {
        rho_0 * (libm::exp(-h / H) * (-H / (H * H)))
    }

/// ∂rho/∂H，量纲 `kg·m^-4`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn rho__d_H(rho_0: f64, h: f64, H: f64) -> f64 {
        let __t0 = -h;
        rho_0 * (libm::exp(__t0 / H) * (-__t0 / (H * H)))
    }

// 不生成 f32 变体：no_std 车道只做 f64 + libm（f32 需 libm 的 *f 系列）。

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

/// 结果量纲：`kg·m·s^-2`
///
/// ```text
/// 0.5 \cdot \rho_0 \cdot \exp(\frac{-h}{H}) \cdot v^{2} \cdot \mathrm{Cd} \cdot A
/// ```
///
/// 源：`formulas/drag.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn drag(rho_0: f64, h: f64, H: f64, Cd: f64, A: f64, v: f64) -> f64 {
        0.5 * rho_0 * libm::exp(-h / H) * Cd * A * v * v
    }

/// ∂drag/∂rho_0，量纲 `m^4·s^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn drag__d_rho_0(rho_0: f64, h: f64, H: f64, Cd: f64, A: f64, v: f64) -> f64 {
        0.5 * libm::exp(-h / H) * (v * v) * Cd * A
    }

/// ∂drag/∂h，量纲 `kg·s^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn drag__d_h(rho_0: f64, h: f64, H: f64, Cd: f64, A: f64, v: f64) -> f64 {
        0.5 * rho_0 * (libm::exp(-h / H) * (-H / (H * H))) * (v * v) * Cd * A
    }

/// ∂drag/∂H，量纲 `kg·s^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn drag__d_H(rho_0: f64, h: f64, H: f64, Cd: f64, A: f64, v: f64) -> f64 {
        let __t0 = -h;
        0.5 * rho_0 * (libm::exp(__t0 / H) * (-__t0 / (H * H))) * (v * v) * Cd * A
    }

/// ∂drag/∂Cd，量纲 `kg·m·s^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn drag__d_Cd(rho_0: f64, h: f64, H: f64, Cd: f64, A: f64, v: f64) -> f64 {
        0.5 * rho_0 * libm::exp(-h / H) * (v * v) * A
    }

/// ∂drag/∂A，量纲 `kg·m^-1·s^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn drag__d_A(rho_0: f64, h: f64, H: f64, Cd: f64, A: f64, v: f64) -> f64 {
        0.5 * rho_0 * libm::exp(-h / H) * (v * v) * Cd
    }

/// ∂drag/∂v，量纲 `kg·s^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn drag__d_v(rho_0: f64, h: f64, H: f64, Cd: f64, A: f64, v: f64) -> f64 {
        0.5 * rho_0 * libm::exp(-h / H) * (2.0 * v) * Cd * A
    }

// 不生成 f32 变体：no_std 车道只做 f64 + libm（f32 需 libm 的 *f 系列）。

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
pub const DRAG__LATEX: &str = "0.5 \\cdot \\rho_0 \\cdot \\exp(\\frac{-h}{H}) \\cdot v^{2} \\cdot \\mathrm{Cd} \\cdot A";

/// 结果量纲：`kg^(1/2)·m^(-3/2)`
///
/// ```text
/// \sqrt{\rho_0 \cdot \mathrm{Cd}}
/// ```
///
/// 源：`formulas/drag.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn gm(rho_0: f64, Cd: f64) -> f64 {
        libm::sqrt(rho_0 * Cd)
    }

/// ∂gm/∂rho_0，量纲 `kg^(-1/2)·m^(3/2)`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn gm__d_rho_0(rho_0: f64, Cd: f64) -> f64 {
        Cd / (2.0 * libm::sqrt(rho_0 * Cd))
    }

/// ∂gm/∂Cd，量纲 `kg^(1/2)·m^(-3/2)`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn gm__d_Cd(rho_0: f64, Cd: f64) -> f64 {
        rho_0 / (2.0 * libm::sqrt(rho_0 * Cd))
    }

// 不生成 f32 变体：no_std 车道只做 f64 + libm（f32 需 libm 的 *f 系列）。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn gm__batch(rho_0: &[f64], Cd: &[f64], out: &mut [f64]) {
    let __n = out.len().min(rho_0.len()).min(Cd.len());
    for __row in 0..__n {
        out[__row] = gm(rho_0[__row], Cd[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn gm__batch_n<const __HIMARI_LEN: usize>(rho_0: &[f64; __HIMARI_LEN], Cd: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = gm(rho_0[__row], Cd[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const GM__LATEX: &str = "\\sqrt{\\rho_0 \\cdot \\mathrm{Cd}}";

// 不生成系统雅可比：它需要 alloc（Vec<Vec<f64>>），本车道只承诺 core + libm。

