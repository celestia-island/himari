// 由 build.rs 生成，勿手改。
// 每条产物的源文件与量纲都写在注释里。

/// 结果量纲：`kg·m^2·s^-3·A^-2`
///
/// ```text
/// R + j \cdot w \cdot L
/// ```
///
/// 源：`formulas/ac_system.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn Z_branch(R: f64, w: f64, L: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        __c_add((R, 0.0), __c_mul(__c_mul((0.0, 1.0), (w, 0.0)), (L, 0.0)))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn Z_branch__batch(R: &[f64], w: &[f64], L: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(R.len()).min(w.len()).min(L.len());
    for __row in 0..__n {
        out[__row] = Z_branch(R[__row], w[__row], L[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn Z_branch__batch_n<const __HIMARI_LEN: usize>(R: &[f64; __HIMARI_LEN], w: &[f64; __HIMARI_LEN], L: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = Z_branch(R[__row], w[__row], L[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_BRANCH__LATEX: &str = "R + j \\cdot w \\cdot L";

/// 结果量纲：`A`
///
/// ```text
/// \operatorname{solve}(\begin{bmatrix}(R + j \cdot w \cdot L) \cdot \mathrm{I1} - V & (R + j \cdot w \cdot L) \cdot \mathrm{I1} - R \cdot \mathrm{I2}\end{bmatrix}, \begin{bmatrix}\mathrm{I1} & \mathrm{I2}\end{bmatrix})
/// ```
///
/// 源：`formulas/ac_system.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn I(R: f64, w: f64, L: f64, V: f64) -> Vec<Vec<(f64, f64)>> {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_div(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            let m = b.0.abs().max(b.1.abs());
            if m > 1e-150 && m < 1e150 {
                let d = b.0 * b.0 + b.1 * b.1;
                ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
            } else {
                let (br, bi) = (b.0 / m, b.1 / m);
                let s = m * (br * br + bi * bi);
                ((a.0 * br + a.1 * bi) / s, (a.1 * br - a.0 * bi) / s)
            }
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        #[inline(always)]
        fn __c_neg(a: (f64, f64)) -> (f64, f64) {
            (-a.0, -a.1)
        }
        let __t0 = __c_mul((0.0, 1.0), (w, 0.0));
        let __t1 = __c_mul(__t0, (L, 0.0));
        let __t2 = __c_add((R, 0.0), __t1);
        let __t3 = __c_div((V, 0.0), __t2);
        vec![vec![__t3], vec![__c_div(__c_neg(__c_mul(__t2, __t3)), __c_neg((R, 0.0)))]]
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn I__batch(R: &[f64], w: &[f64], L: &[f64], V: &[f64], out: &mut [Vec<Vec<(f64, f64)>>]) {
    let __n = out.len().min(R.len()).min(w.len()).min(L.len()).min(V.len());
    for __row in 0..__n {
        out[__row] = I(R[__row], w[__row], L[__row], V[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn I__batch_n<const __HIMARI_LEN: usize>(R: &[f64; __HIMARI_LEN], w: &[f64; __HIMARI_LEN], L: &[f64; __HIMARI_LEN], V: &[f64; __HIMARI_LEN], out: &mut [Vec<Vec<(f64, f64)>>; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = I(R[__row], w[__row], L[__row], V[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const I__LATEX: &str = "\\operatorname{solve}(\\begin{bmatrix}(R + j \\cdot w \\cdot L) \\cdot \\mathrm{I1} - V & (R + j \\cdot w \\cdot L) \\cdot \\mathrm{I1} - R \\cdot \\mathrm{I2}\\end{bmatrix}, \\begin{bmatrix}\\mathrm{I1} & \\mathrm{I2}\\end{bmatrix})";

/// 结果量纲：`kg·m^2·s^-3·A^-2`
///
/// ```text
/// \begin{bmatrix}R + j \cdot w \cdot L & R \\ -R & R + j \cdot w \cdot L\end{bmatrix}
/// ```
///
/// 源：`formulas/ac_system.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn Zm(R: f64, w: f64, L: f64) -> Vec<Vec<(f64, f64)>> {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        #[inline(always)]
        fn __c_neg(a: (f64, f64)) -> (f64, f64) {
            (-a.0, -a.1)
        }
        let __t0 = __c_mul((0.0, 1.0), (w, 0.0));
        let __t1 = __c_mul(__t0, (L, 0.0));
        let __t2 = __c_add((R, 0.0), __t1);
        vec![vec![__t2, (R, 0.0)], vec![__c_neg((R, 0.0)), __t2]]
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn Zm__batch(R: &[f64], w: &[f64], L: &[f64], out: &mut [Vec<Vec<(f64, f64)>>]) {
    let __n = out.len().min(R.len()).min(w.len()).min(L.len());
    for __row in 0..__n {
        out[__row] = Zm(R[__row], w[__row], L[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn Zm__batch_n<const __HIMARI_LEN: usize>(R: &[f64; __HIMARI_LEN], w: &[f64; __HIMARI_LEN], L: &[f64; __HIMARI_LEN], out: &mut [Vec<Vec<(f64, f64)>>; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = Zm(R[__row], w[__row], L[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const ZM__LATEX: &str = "\\begin{bmatrix}R + j \\cdot w \\cdot L & R \\\\ -R & R + j \\cdot w \\cdot L\\end{bmatrix}";

/// 结果量纲：`s^-1`
///
/// ```text
/// A \cdot \exp(-(\frac{\mathrm{E_a}}{R \cdot T}))
/// ```
///
/// 源：`formulas/arrhenius.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn arrhenius(A: f64, E_a: f64, R: f64, T: f64) -> f64 {
        A * (-(E_a / (R * T))).exp()
    }

/// ∂arrhenius/∂A，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn arrhenius__d_A(A: f64, E_a: f64, R: f64, T: f64) -> f64 {
        (-(E_a / (R * T))).exp()
    }

/// ∂arrhenius/∂E_a，量纲 `kg^-1·m^-2·s·mol`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn arrhenius__d_E_a(A: f64, E_a: f64, R: f64, T: f64) -> f64 {
        let __t0 = R * T;
        A * ((-(E_a / __t0)).exp() * -(__t0 / (__t0 * __t0)))
    }

/// ∂arrhenius/∂R，量纲 `kg^-1·m^-2·s·K·mol`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn arrhenius__d_R(A: f64, E_a: f64, R: f64, T: f64) -> f64 {
        let __t0 = R * T;
        A * ((-(E_a / __t0)).exp() * -(-(E_a * T) / (__t0 * __t0)))
    }

/// ∂arrhenius/∂T，量纲 `s^-1·K^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn arrhenius__d_T(A: f64, E_a: f64, R: f64, T: f64) -> f64 {
        let __t0 = R * T;
        A * ((-(E_a / __t0)).exp() * -(-(E_a * R) / (__t0 * __t0)))
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn arrhenius__f32(A: f32, E_a: f32, R: f32, T: f32) -> f32 {
        A * (-(E_a / (R * T))).exp()
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn arrhenius__batch(A: &[f64], E_a: &[f64], R: &[f64], T: &[f64], out: &mut [f64]) {
    let __n = out.len().min(A.len()).min(E_a.len()).min(R.len()).min(T.len());
    for __row in 0..__n {
        out[__row] = arrhenius(A[__row], E_a[__row], R[__row], T[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn arrhenius__batch_n<const __HIMARI_LEN: usize>(A: &[f64; __HIMARI_LEN], E_a: &[f64; __HIMARI_LEN], R: &[f64; __HIMARI_LEN], T: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = arrhenius(A[__row], E_a[__row], R[__row], T[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const ARRHENIUS__LATEX: &str = "A \\cdot \\exp(-(\\frac{\\mathrm{E_a}}{R \\cdot T}))";

/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），列 = 参数（声明顺序）。
///
/// 参数 = `A, E_a, R, T`；不依赖某参数的行在该列恒为 0。
/// 源：`formulas/arrhenius.formula`
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn arrhenius__jacobian(A: f64, E_a: f64, R: f64, T: f64) -> Vec<Vec<f64>> {
    vec![
        vec![arrhenius__d_A(A, E_a, R, T), arrhenius__d_E_a(A, E_a, R, T), arrhenius__d_R(A, E_a, R, T), arrhenius__d_T(A, E_a, R, T)]
    ]
}

/// 源：`formulas/banded.formula`（量纲 `1`）
/// 对称带状阵（半带宽 2：主对角与 2 条副对角同值）
#[allow(non_snake_case, clippy::needless_range_loop)]
pub fn banded(v: f64, n: usize) -> Vec<Vec<f64>> {
    let vv = {
        v
    };
    let mut m = vec![vec![0.0; n]; n];
    for __row in 0..n {
        let __hi = (__row + 2).min(n - 1);
        for __col in __row..=__hi {
            m[__row][__col] = vv;
            m[__col][__row] = vv;
        }
    }
    m
}

/// 源：`formulas/bidiag.formula`（量纲 `1`）
/// 下双对角阵（主对角 b + 下次对角 a）
#[allow(non_snake_case, clippy::needless_range_loop)]
pub fn bidiag(a: f64, b: f64, n: usize) -> Vec<Vec<f64>> {
    let av = {
        a
    };
    let bv = {
        b
    };
    let mut m = vec![vec![0.0; n]; n];
    for __row in 0..n {
        m[__row][__row] = bv;
        let __next = __row + 1;
        if __next < n {
            m[__next][__row] = av;
        }
    }
    m
}

/// 源：`formulas/blockdiag.formula`（量纲 `1`）
/// 分块对角阵 diag(P·I₂, Q·I₂)（由模式矩阵生成）
#[allow(non_snake_case, clippy::needless_range_loop)]
pub fn blockdiag(p: f64, q: f64, n: usize) -> Vec<Vec<f64>> {
    // n 是**总**维数；两块各占一半（奇数时右下块多一）
    let pv = {
        p
    };
    let qv = {
        q
    };
    let half = n / 2;
    let mut m = vec![vec![0.0; n]; n];
    for __row in 0..half {
        m[__row][__row] = pv;
        m[n - 1 - __row][n - 1 - __row] = qv;
    }
    if n % 2 == 1 {
        m[half][half] = qv;
    }
    m
}

/// 结果量纲：`m^-2·A`
///
/// ```text
/// \mathrm{i_0} \cdot (\exp(\frac{\alpha \cdot n \cdot F \cdot \eta}{R \cdot T}) - \exp(-(\frac{(1 - \alpha) \cdot n \cdot F \cdot \eta}{R \cdot T})))
/// ```
///
/// 源：`formulas/bv.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn bv(i_0: f64, eta: f64, alpha: f64, n: f64, F: f64, R: f64, T: f64) -> f64 {
        let __t0 = R * T;
        i_0 * ((alpha * n * F * eta / __t0).exp() - (-((1.0 - alpha) * n * F * eta / __t0)).exp())
    }

/// ∂bv/∂i_0，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn bv__d_i_0(i_0: f64, eta: f64, alpha: f64, n: f64, F: f64, R: f64, T: f64) -> f64 {
        let __t0 = R * T;
        (alpha * n * F * eta / __t0).exp() - (-((1.0 - alpha) * n * F * eta / __t0)).exp()
    }

/// ∂bv/∂eta，量纲 `kg^-1·m^-4·s^3·A^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn bv__d_eta(i_0: f64, eta: f64, alpha: f64, n: f64, F: f64, R: f64, T: f64) -> f64 {
        let __t0 = alpha * n;
        let __t1 = __t0 * F;
        let __t2 = R * T;
        let __t3 = __t2 * __t2;
        let __t4 = 1.0 - alpha;
        let __t5 = __t4 * n;
        let __t6 = __t5 * F;
        i_0 * ((__t1 * eta / __t2).exp() * (__t1 * __t2 / __t3) - (-(__t6 * eta / __t2)).exp() * -(__t6 * __t2 / __t3))
    }

/// ∂bv/∂alpha，量纲 `m^-2·A`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn bv__d_alpha(i_0: f64, eta: f64, alpha: f64, n: f64, F: f64, R: f64, T: f64) -> f64 {
        let __t0 = R * T;
        let __t1 = __t0 * __t0;
        i_0 * ((alpha * n * F * eta / __t0).exp() * (n * F * eta * __t0 / __t1) - (-((1.0 - alpha) * n * F * eta / __t0)).exp() * -(-n * F * eta * __t0 / __t1))
    }

/// ∂bv/∂n，量纲 `m^-2·A`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn bv__d_n(i_0: f64, eta: f64, alpha: f64, n: f64, F: f64, R: f64, T: f64) -> f64 {
        let __t0 = R * T;
        let __t1 = __t0 * __t0;
        let __t2 = 1.0 - alpha;
        i_0 * ((alpha * n * F * eta / __t0).exp() * (alpha * F * eta * __t0 / __t1) - (-(__t2 * n * F * eta / __t0)).exp() * -(__t2 * F * eta * __t0 / __t1))
    }

/// ∂bv/∂F，量纲 `m^-2·s^-1·mol`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn bv__d_F(i_0: f64, eta: f64, alpha: f64, n: f64, F: f64, R: f64, T: f64) -> f64 {
        let __t0 = alpha * n;
        let __t1 = R * T;
        let __t2 = __t1 * __t1;
        let __t3 = 1.0 - alpha;
        let __t4 = __t3 * n;
        i_0 * ((__t0 * F * eta / __t1).exp() * (__t0 * eta * __t1 / __t2) - (-(__t4 * F * eta / __t1)).exp() * -(__t4 * eta * __t1 / __t2))
    }

/// ∂bv/∂R，量纲 `kg^-1·m^-4·s^2·A·K·mol`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn bv__d_R(i_0: f64, eta: f64, alpha: f64, n: f64, F: f64, R: f64, T: f64) -> f64 {
        let __t0 = alpha * n;
        let __t1 = __t0 * F;
        let __t2 = __t1 * eta;
        let __t3 = R * T;
        let __t4 = __t3 * __t3;
        let __t5 = 1.0 - alpha;
        let __t6 = __t5 * n;
        let __t7 = __t6 * F;
        let __t8 = __t7 * eta;
        i_0 * ((__t2 / __t3).exp() * (-(__t2 * T) / __t4) - (-(__t8 / __t3)).exp() * -(-(__t8 * T) / __t4))
    }

/// ∂bv/∂T，量纲 `m^-2·A·K^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn bv__d_T(i_0: f64, eta: f64, alpha: f64, n: f64, F: f64, R: f64, T: f64) -> f64 {
        let __t0 = alpha * n;
        let __t1 = __t0 * F;
        let __t2 = __t1 * eta;
        let __t3 = R * T;
        let __t4 = __t3 * __t3;
        let __t5 = 1.0 - alpha;
        let __t6 = __t5 * n;
        let __t7 = __t6 * F;
        let __t8 = __t7 * eta;
        i_0 * ((__t2 / __t3).exp() * (-(__t2 * R) / __t4) - (-(__t8 / __t3)).exp() * -(-(__t8 * R) / __t4))
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn bv__f32(i_0: f32, eta: f32, alpha: f32, n: f32, F: f32, R: f32, T: f32) -> f32 {
        let __t0 = R * T;
        i_0 * ((alpha * n * F * eta / __t0).exp() - (-((1.0f32 - alpha) * n * F * eta / __t0)).exp())
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn bv__batch(i_0: &[f64], eta: &[f64], alpha: &[f64], n: &[f64], F: &[f64], R: &[f64], T: &[f64], out: &mut [f64]) {
    let __n = out.len().min(i_0.len()).min(eta.len()).min(alpha.len()).min(n.len()).min(F.len()).min(R.len()).min(T.len());
    for __row in 0..__n {
        out[__row] = bv(i_0[__row], eta[__row], alpha[__row], n[__row], F[__row], R[__row], T[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn bv__batch_n<const __HIMARI_LEN: usize>(i_0: &[f64; __HIMARI_LEN], eta: &[f64; __HIMARI_LEN], alpha: &[f64; __HIMARI_LEN], n: &[f64; __HIMARI_LEN], F: &[f64; __HIMARI_LEN], R: &[f64; __HIMARI_LEN], T: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = bv(i_0[__row], eta[__row], alpha[__row], n[__row], F[__row], R[__row], T[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const BV__LATEX: &str = "\\mathrm{i_0} \\cdot (\\exp(\\frac{\\alpha \\cdot n \\cdot F \\cdot \\eta}{R \\cdot T}) - \\exp(-(\\frac{(1 - \\alpha) \\cdot n \\cdot F \\cdot \\eta}{R \\cdot T})))";

/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），列 = 参数（声明顺序）。
///
/// 参数 = `i_0, eta, alpha, n, F, R, T`；不依赖某参数的行在该列恒为 0。
/// 源：`formulas/bv.formula`
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn bv__jacobian(i_0: f64, eta: f64, alpha: f64, n: f64, F: f64, R: f64, T: f64) -> Vec<Vec<f64>> {
    vec![
        vec![bv__d_i_0(i_0, eta, alpha, n, F, R, T), bv__d_eta(i_0, eta, alpha, n, F, R, T), bv__d_alpha(i_0, eta, alpha, n, F, R, T), bv__d_n(i_0, eta, alpha, n, F, R, T), bv__d_F(i_0, eta, alpha, n, F, R, T), bv__d_R(i_0, eta, alpha, n, F, R, T), bv__d_T(i_0, eta, alpha, n, F, R, T)]
    ]
}

/// 结果量纲：`A`
/// ⚠ 参数 `C_dl` 声明为 `µF`：入参是 **SI 基单位**（1 µF = 0.000001 基单位，调用前请换算）
/// ⚠ 参数 `E` 声明为 `mV`：入参是 **SI 基单位**（1 mV = 0.001 基单位，调用前请换算）
/// ⚠ 参数 `f` 声明为 `kHz`：入参是 **SI 基单位**（1 kHz = 1000 基单位，调用前请换算）
///
/// ```text
/// \mathrm{C_dl} \cdot E \cdot f
/// ```
///
/// 源：`formulas/cdl_current.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn i_c(C_dl: f64, E: f64, f: f64) -> f64 {
        C_dl * E * f
    }

/// ∂i_c/∂C_dl，量纲 `kg·m^2·s^-4·A^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn i_c__d_C_dl(C_dl: f64, E: f64, f: f64) -> f64 {
        E * f
    }

/// ∂i_c/∂E，量纲 `kg^-1·m^-2·s^3·A^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn i_c__d_E(C_dl: f64, E: f64, f: f64) -> f64 {
        C_dl * f
    }

/// ∂i_c/∂f，量纲 `s·A`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn i_c__d_f(C_dl: f64, E: f64, f: f64) -> f64 {
        C_dl * E
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn i_c__f32(C_dl: f32, E: f32, f: f32) -> f32 {
        C_dl * E * f
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn i_c__batch(C_dl: &[f64], E: &[f64], f: &[f64], out: &mut [f64]) {
    let __n = out.len().min(C_dl.len()).min(E.len()).min(f.len());
    for __row in 0..__n {
        out[__row] = i_c(C_dl[__row], E[__row], f[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn i_c__batch_n<const __HIMARI_LEN: usize>(C_dl: &[f64; __HIMARI_LEN], E: &[f64; __HIMARI_LEN], f: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = i_c(C_dl[__row], E[__row], f[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const I_C__LATEX: &str = "\\mathrm{C_dl} \\cdot E \\cdot f";

/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），列 = 参数（声明顺序）。
///
/// 参数 = `C_dl, E, f`；不依赖某参数的行在该列恒为 0。
/// 源：`formulas/cdl_current.formula`
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn cdl_current__jacobian(C_dl: f64, E: f64, f: f64) -> Vec<Vec<f64>> {
    vec![
        vec![i_c__d_C_dl(C_dl, E, f), i_c__d_E(C_dl, E, f), i_c__d_f(C_dl, E, f)]
    ]
}

/// 结果量纲：`1`
///
/// ```text
/// u + j \cdot v
/// ```
///
/// 源：`formulas/complex_ops.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn z(u: f64, v: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        __c_add((u, 0.0), __c_mul((0.0, 1.0), (v, 0.0)))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z__batch(u: &[f64], v: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(u.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = z(u[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z__batch_n<const __HIMARI_LEN: usize>(u: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = z(u[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z__LATEX: &str = "u + j \\cdot v";

/// 结果量纲：`1`
///
/// ```text
/// \operatorname{abs}(u + j \cdot v)
/// ```
///
/// 源：`formulas/complex_ops.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn z_abs(u: f64, v: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_abs(a: (f64, f64)) -> (f64, f64) {
            (a.0.hypot(a.1), 0.0)
        }
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        __c_abs(__c_add((u, 0.0), __c_mul((0.0, 1.0), (v, 0.0))))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_abs__batch(u: &[f64], v: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(u.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = z_abs(u[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_abs__batch_n<const __HIMARI_LEN: usize>(u: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = z_abs(u[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_ABS__LATEX: &str = "\\operatorname{abs}(u + j \\cdot v)";

/// 结果量纲：`1`
///
/// ```text
/// \operatorname{arg}(u + j \cdot v)
/// ```
///
/// 源：`formulas/complex_ops.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn z_arg(u: f64, v: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_arg(a: (f64, f64)) -> (f64, f64) {
            (a.1.atan2(a.0), 0.0)
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        __c_arg(__c_add((u, 0.0), __c_mul((0.0, 1.0), (v, 0.0))))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_arg__batch(u: &[f64], v: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(u.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = z_arg(u[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_arg__batch_n<const __HIMARI_LEN: usize>(u: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = z_arg(u[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_ARG__LATEX: &str = "\\operatorname{arg}(u + j \\cdot v)";

/// 结果量纲：`1`
///
/// ```text
/// \operatorname{conj}(u + j \cdot v)
/// ```
///
/// 源：`formulas/complex_ops.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn z_conj(u: f64, v: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_conj(a: (f64, f64)) -> (f64, f64) {
            (a.0, -a.1)
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        __c_conj(__c_add((u, 0.0), __c_mul((0.0, 1.0), (v, 0.0))))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_conj__batch(u: &[f64], v: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(u.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = z_conj(u[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_conj__batch_n<const __HIMARI_LEN: usize>(u: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = z_conj(u[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_CONJ__LATEX: &str = "\\operatorname{conj}(u + j \\cdot v)";

/// 结果量纲：`1`
///
/// ```text
/// \operatorname{re}(u + j \cdot v)
/// ```
///
/// 源：`formulas/complex_ops.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn z_re(u: f64, v: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        #[inline(always)]
        fn __c_re(a: (f64, f64)) -> (f64, f64) {
            (a.0, 0.0)
        }
        __c_re(__c_add((u, 0.0), __c_mul((0.0, 1.0), (v, 0.0))))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_re__batch(u: &[f64], v: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(u.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = z_re(u[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_re__batch_n<const __HIMARI_LEN: usize>(u: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = z_re(u[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_RE__LATEX: &str = "\\operatorname{re}(u + j \\cdot v)";

/// 结果量纲：`1`
///
/// ```text
/// \operatorname{im}(u + j \cdot v)
/// ```
///
/// 源：`formulas/complex_ops.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn z_im(u: f64, v: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_im(a: (f64, f64)) -> (f64, f64) {
            (a.1, 0.0)
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        __c_im(__c_add((u, 0.0), __c_mul((0.0, 1.0), (v, 0.0))))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_im__batch(u: &[f64], v: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(u.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = z_im(u[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_im__batch_n<const __HIMARI_LEN: usize>(u: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = z_im(u[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_IM__LATEX: &str = "\\operatorname{im}(u + j \\cdot v)";

/// 结果量纲：`1`
///
/// ```text
/// \exp(u + j \cdot v)
/// ```
///
/// 源：`formulas/complex_ops.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn z_exp(u: f64, v: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_exp(a: (f64, f64)) -> (f64, f64) {
            if a.1 == 0.0 {
                return (a.0.exp(), 0.0);
            }
            let m = a.0.exp();
            (m * a.1.cos(), m * a.1.sin())
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        __c_exp(__c_add((u, 0.0), __c_mul((0.0, 1.0), (v, 0.0))))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_exp__batch(u: &[f64], v: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(u.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = z_exp(u[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_exp__batch_n<const __HIMARI_LEN: usize>(u: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = z_exp(u[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_EXP__LATEX: &str = "\\exp(u + j \\cdot v)";

/// 结果量纲：`1`
///
/// ```text
/// \ln(u + j \cdot v)
/// ```
///
/// 源：`formulas/complex_ops.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn z_ln(u: f64, v: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_ln(a: (f64, f64)) -> (f64, f64) {
            (a.0.hypot(a.1).ln(), a.1.atan2(a.0))
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        __c_ln(__c_add((u, 0.0), __c_mul((0.0, 1.0), (v, 0.0))))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_ln__batch(u: &[f64], v: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(u.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = z_ln(u[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_ln__batch_n<const __HIMARI_LEN: usize>(u: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = z_ln(u[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_LN__LATEX: &str = "\\ln(u + j \\cdot v)";

/// 结果量纲：`1`
///
/// ```text
/// \sin(u + j \cdot v)
/// ```
///
/// 源：`formulas/complex_ops.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn z_sin(u: f64, v: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        #[inline(always)]
        fn __c_sin(a: (f64, f64)) -> (f64, f64) {
            (a.0.sin() * a.1.cosh(), a.0.cos() * a.1.sinh())
        }
        __c_sin(__c_add((u, 0.0), __c_mul((0.0, 1.0), (v, 0.0))))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_sin__batch(u: &[f64], v: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(u.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = z_sin(u[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_sin__batch_n<const __HIMARI_LEN: usize>(u: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = z_sin(u[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_SIN__LATEX: &str = "\\sin(u + j \\cdot v)";

/// 结果量纲：`1`
///
/// ```text
/// \cos(u + j \cdot v)
/// ```
///
/// 源：`formulas/complex_ops.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn z_cos(u: f64, v: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_cos(a: (f64, f64)) -> (f64, f64) {
            (a.0.cos() * a.1.cosh(), -(a.0.sin() * a.1.sinh()))
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        __c_cos(__c_add((u, 0.0), __c_mul((0.0, 1.0), (v, 0.0))))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_cos__batch(u: &[f64], v: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(u.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = z_cos(u[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_cos__batch_n<const __HIMARI_LEN: usize>(u: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = z_cos(u[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_COS__LATEX: &str = "\\cos(u + j \\cdot v)";

/// 结果量纲：`1`
///
/// ```text
/// \tan(u + j \cdot v)
/// ```
///
/// 源：`formulas/complex_ops.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn z_tan(u: f64, v: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        #[inline(always)]
        fn __c_tan(a: (f64, f64)) -> (f64, f64) {
            let s = (a.0.sin() * a.1.cosh(), a.0.cos() * a.1.sinh());
            let c = (a.0.cos() * a.1.cosh(), -(a.0.sin() * a.1.sinh()));
            let d = c.0 * c.0 + c.1 * c.1;
            ((s.0 * c.0 + s.1 * c.1) / d, (s.1 * c.0 - s.0 * c.1) / d)
        }
        __c_tan(__c_add((u, 0.0), __c_mul((0.0, 1.0), (v, 0.0))))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_tan__batch(u: &[f64], v: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(u.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = z_tan(u[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_tan__batch_n<const __HIMARI_LEN: usize>(u: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = z_tan(u[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_TAN__LATEX: &str = "\\tan(u + j \\cdot v)";

/// 结果量纲：`1`
///
/// ```text
/// \sinh(u + j \cdot v)
/// ```
///
/// 源：`formulas/complex_ops.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn z_sinh(u: f64, v: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        #[inline(always)]
        fn __c_sinh(a: (f64, f64)) -> (f64, f64) {
            (a.0.sinh() * a.1.cos(), a.0.cosh() * a.1.sin())
        }
        __c_sinh(__c_add((u, 0.0), __c_mul((0.0, 1.0), (v, 0.0))))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_sinh__batch(u: &[f64], v: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(u.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = z_sinh(u[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_sinh__batch_n<const __HIMARI_LEN: usize>(u: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = z_sinh(u[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_SINH__LATEX: &str = "\\sinh(u + j \\cdot v)";

/// 结果量纲：`1`
///
/// ```text
/// \cosh(u + j \cdot v)
/// ```
///
/// 源：`formulas/complex_ops.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn z_cosh(u: f64, v: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_cosh(a: (f64, f64)) -> (f64, f64) {
            (a.0.cosh() * a.1.cos(), a.0.sinh() * a.1.sin())
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        __c_cosh(__c_add((u, 0.0), __c_mul((0.0, 1.0), (v, 0.0))))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_cosh__batch(u: &[f64], v: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(u.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = z_cosh(u[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_cosh__batch_n<const __HIMARI_LEN: usize>(u: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = z_cosh(u[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_COSH__LATEX: &str = "\\cosh(u + j \\cdot v)";

/// 结果量纲：`1`
///
/// ```text
/// \tanh(u + j \cdot v)
/// ```
///
/// 源：`formulas/complex_ops.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn z_tanh(u: f64, v: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        #[inline(always)]
        fn __c_tanh(a: (f64, f64)) -> (f64, f64) {
            let s = (a.0.sinh() * a.1.cos(), a.0.cosh() * a.1.sin());
            let c = (a.0.cosh() * a.1.cos(), a.0.sinh() * a.1.sin());
            let d = c.0 * c.0 + c.1 * c.1;
            ((s.0 * c.0 + s.1 * c.1) / d, (s.1 * c.0 - s.0 * c.1) / d)
        }
        __c_tanh(__c_add((u, 0.0), __c_mul((0.0, 1.0), (v, 0.0))))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_tanh__batch(u: &[f64], v: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(u.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = z_tanh(u[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_tanh__batch_n<const __HIMARI_LEN: usize>(u: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = z_tanh(u[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_TANH__LATEX: &str = "\\tanh(u + j \\cdot v)";

/// 结果量纲：`1`
///
/// ```text
/// (u - 1)^{3} + j \cdot v
/// ```
///
/// 源：`formulas/complex_ops.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn z_pow3(u: f64, v: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        #[inline(always)]
        fn __c_powf(a: (f64, f64), p: f64) -> (f64, f64) {
            if !p.is_finite() {
                return (f64::NAN, 0.0);
            }
            if a.1 == 0.0 {
                if p == p.trunc() && p.abs() <= i32::MAX as f64 {
                    return (a.0.powi(p as i32), 0.0);
                }
                if a.0 >= 0.0 {
                    return (a.0.powf(p), 0.0);
                }
            }
            let (r, t) = (a.0.hypot(a.1), a.1.atan2(a.0));
            let m = r.powf(p);
            let g = t * p;
            (m * g.cos(), m * g.sin())
        }
        #[inline(always)]
        fn __c_sub(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 - b.0, a.1 - b.1)
        }
        __c_add(__c_powf(__c_sub((u, 0.0), (1.0, 0.0)), 3.0), __c_mul((0.0, 1.0), (v, 0.0)))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_pow3__batch(u: &[f64], v: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(u.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = z_pow3(u[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_pow3__batch_n<const __HIMARI_LEN: usize>(u: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = z_pow3(u[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_POW3__LATEX: &str = "(u - 1)^{3} + j \\cdot v";

/// 结果量纲：`1`
///
/// ```text
/// (u + j \cdot v)^{0.5}
/// ```
///
/// 源：`formulas/complex_ops.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn z_sqrt(u: f64, v: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        #[inline(always)]
        fn __c_sqrt(a: (f64, f64)) -> (f64, f64) {
            if a.1 == 0.0 {
                return if a.0 >= 0.0 { (a.0.sqrt(), 0.0) } else { (0.0, (-a.0).sqrt()) };
            }
            if a.0 == 0.0 {
                let m = (a.1.abs() * 0.5).sqrt();
                return (m, m.copysign(a.1));
            }
            let r = a.0.hypot(a.1);
            let re = (((r + a.0) * 0.5).max(0.0)).sqrt();
            let im = (((r - a.0) * 0.5).max(0.0)).sqrt().copysign(a.1);
            (re, im)
        }
        __c_sqrt(__c_add((u, 0.0), __c_mul((0.0, 1.0), (v, 0.0))))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_sqrt__batch(u: &[f64], v: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(u.len()).min(v.len());
    for __row in 0..__n {
        out[__row] = z_sqrt(u[__row], v[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn z_sqrt__batch_n<const __HIMARI_LEN: usize>(u: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = z_sqrt(u[__row], v[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_SQRT__LATEX: &str = "(u + j \\cdot v)^{0.5}";

/// 结果量纲：`1`
///
/// ```text
/// \operatorname{roots}(a \cdot x^{2} + b \cdot x + c + j, x)
/// ```
///
/// 源：`formulas/complex_roots.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn complex_roots(a: f64, b: f64, c: f64) -> Vec<Vec<(f64, f64)>> {
        #[inline(always)]
        fn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 + b.0, a.1 + b.1)
        }
        #[inline(always)]
        fn __c_conj(a: (f64, f64)) -> (f64, f64) {
            (a.0, -a.1)
        }
        #[inline(always)]
        fn __c_div(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            let m = b.0.abs().max(b.1.abs());
            if m > 1e-150 && m < 1e150 {
                let d = b.0 * b.0 + b.1 * b.1;
                ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
            } else {
                let (br, bi) = (b.0 / m, b.1 / m);
                let s = m * (br * br + bi * bi);
                ((a.0 * br + a.1 * bi) / s, (a.1 * br - a.0 * bi) / s)
            }
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        #[inline(always)]
        fn __c_neg(a: (f64, f64)) -> (f64, f64) {
            (-a.0, -a.1)
        }
        #[inline(always)]
        fn __c_quad_root(a: (f64, f64), b: (f64, f64), c: (f64, f64), first: bool) -> (f64, f64) {
            let d = __c_sub(__c_mul(b, b), __c_mul((4.0, 0.0), __c_mul(a, c)));
            let s = __c_sqrt(d);
            let q_first = __c_mul(__c_conj(b), s).0 >= 0.0;
            let signed_s = if q_first { s } else { __c_neg(s) };
            let q = __c_mul((-0.5, 0.0), __c_add(b, signed_s));
            if q.0 == 0.0 && q.1 == 0.0 {
                return __c_div(__c_neg(b), __c_mul((2.0, 0.0), a));
            }
            let large = __c_div(q, a);
            let small = __c_div(c, q);
            if first == q_first { large } else { small }
        }
        #[inline(always)]
        fn __c_sqrt(a: (f64, f64)) -> (f64, f64) {
            if a.1 == 0.0 {
                return if a.0 >= 0.0 { (a.0.sqrt(), 0.0) } else { (0.0, (-a.0).sqrt()) };
            }
            if a.0 == 0.0 {
                let m = (a.1.abs() * 0.5).sqrt();
                return (m, m.copysign(a.1));
            }
            let r = a.0.hypot(a.1);
            let re = (((r + a.0) * 0.5).max(0.0)).sqrt();
            let im = (((r - a.0) * 0.5).max(0.0)).sqrt().copysign(a.1);
            (re, im)
        }
        #[inline(always)]
        fn __c_sub(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 - b.0, a.1 - b.1)
        }
        let __t0 = __c_add((c, 0.0), (0.0, 1.0));
        vec![vec![__c_quad_root((a, 0.0), (b, 0.0), __t0, true)], vec![__c_quad_root((a, 0.0), (b, 0.0), __t0, false)]]
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn complex_roots__batch(a: &[f64], b: &[f64], c: &[f64], out: &mut [Vec<Vec<(f64, f64)>>]) {
    let __n = out.len().min(a.len()).min(b.len()).min(c.len());
    for __row in 0..__n {
        out[__row] = complex_roots(a[__row], b[__row], c[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn complex_roots__batch_n<const __HIMARI_LEN: usize>(a: &[f64; __HIMARI_LEN], b: &[f64; __HIMARI_LEN], c: &[f64; __HIMARI_LEN], out: &mut [Vec<Vec<(f64, f64)>>; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = complex_roots(a[__row], b[__row], c[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const COMPLEX_ROOTS__LATEX: &str = "\\operatorname{roots}(a \\cdot x^{2} + b \\cdot x + c + j, x)";

/// 结果量纲：`1`
///
/// ```text
/// \begin{bmatrix}\mathrm{h_0} & \mathrm{h_1} & \mathrm{h_2} & \mathrm{h_3} \\ \mathrm{h_3} & \mathrm{h_0} & \mathrm{h_1} & \mathrm{h_2} \\ \mathrm{h_2} & \mathrm{h_3} & \mathrm{h_0} & \mathrm{h_1} \\ \mathrm{h_1} & \mathrm{h_2} & \mathrm{h_3} & \mathrm{h_0}\end{bmatrix}
/// ```
///
/// 源：`formulas/conv_matrix.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn H(h_0: f64, h_1: f64, h_2: f64, h_3: f64) -> Vec<Vec<f64>> {
        vec![vec![h_0, h_1, h_2, h_3], vec![h_3, h_0, h_1, h_2], vec![h_2, h_3, h_0, h_1], vec![h_1, h_2, h_3, h_0]]
    }

// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn H__batch(h_0: &[f64], h_1: &[f64], h_2: &[f64], h_3: &[f64], out: &mut [Vec<Vec<f64>>]) {
    let __n = out.len().min(h_0.len()).min(h_1.len()).min(h_2.len()).min(h_3.len());
    for __row in 0..__n {
        out[__row] = H(h_0[__row], h_1[__row], h_2[__row], h_3[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn H__batch_n<const __HIMARI_LEN: usize>(h_0: &[f64; __HIMARI_LEN], h_1: &[f64; __HIMARI_LEN], h_2: &[f64; __HIMARI_LEN], h_3: &[f64; __HIMARI_LEN], out: &mut [Vec<Vec<f64>>; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = H(h_0[__row], h_1[__row], h_2[__row], h_3[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const H__LATEX: &str = "\\begin{bmatrix}\\mathrm{h_0} & \\mathrm{h_1} & \\mathrm{h_2} & \\mathrm{h_3} \\\\ \\mathrm{h_3} & \\mathrm{h_0} & \\mathrm{h_1} & \\mathrm{h_2} \\\\ \\mathrm{h_2} & \\mathrm{h_3} & \\mathrm{h_0} & \\mathrm{h_1} \\\\ \\mathrm{h_1} & \\mathrm{h_2} & \\mathrm{h_3} & \\mathrm{h_0}\\end{bmatrix}";

/// 结果量纲：`m`
///
/// ```text
/// \begin{bmatrix}\mathrm{x_2} - \mathrm{x_1} & \mathrm{x_3} - \mathrm{x_1} \\ \mathrm{y_2} - \mathrm{y_1} & \mathrm{y_3} - \mathrm{y_1}\end{bmatrix}
/// ```
///
/// 源：`formulas/fem_element.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn J(x_1: f64, x_2: f64, x_3: f64, y_1: f64, y_2: f64, y_3: f64) -> Vec<Vec<f64>> {
        vec![vec![x_2 - x_1, x_3 - x_1], vec![y_2 - y_1, y_3 - y_1]]
    }

// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn J__batch(x_1: &[f64], x_2: &[f64], x_3: &[f64], y_1: &[f64], y_2: &[f64], y_3: &[f64], out: &mut [Vec<Vec<f64>>]) {
    let __n = out.len().min(x_1.len()).min(x_2.len()).min(x_3.len()).min(y_1.len()).min(y_2.len()).min(y_3.len());
    for __row in 0..__n {
        out[__row] = J(x_1[__row], x_2[__row], x_3[__row], y_1[__row], y_2[__row], y_3[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn J__batch_n<const __HIMARI_LEN: usize>(x_1: &[f64; __HIMARI_LEN], x_2: &[f64; __HIMARI_LEN], x_3: &[f64; __HIMARI_LEN], y_1: &[f64; __HIMARI_LEN], y_2: &[f64; __HIMARI_LEN], y_3: &[f64; __HIMARI_LEN], out: &mut [Vec<Vec<f64>>; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = J(x_1[__row], x_2[__row], x_3[__row], y_1[__row], y_2[__row], y_3[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const J__LATEX: &str = "\\begin{bmatrix}\\mathrm{x_2} - \\mathrm{x_1} & \\mathrm{x_3} - \\mathrm{x_1} \\\\ \\mathrm{y_2} - \\mathrm{y_1} & \\mathrm{y_3} - \\mathrm{y_1}\\end{bmatrix}";

/// 结果量纲：`m^2`
///
/// ```text
/// \frac{\det(\begin{bmatrix}\mathrm{x_2} - \mathrm{x_1} & \mathrm{x_3} - \mathrm{x_1} \\ \mathrm{y_2} - \mathrm{y_1} & \mathrm{y_3} - \mathrm{y_1}\end{bmatrix})}{2}
/// ```
///
/// 源：`formulas/fem_element.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn area(x_1: f64, x_2: f64, x_3: f64, y_1: f64, y_2: f64, y_3: f64) -> f64 {
        ((x_2 - x_1) * (y_3 - y_1) + -((x_3 - x_1) * (y_2 - y_1))) / 2.0
    }

/// ∂area/∂x_1，量纲 `m`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn area__d_x_1(x_1: f64, x_2: f64, x_3: f64, y_1: f64, y_2: f64, y_3: f64) -> f64 {
        (-(y_3 - y_1) + (y_2 - y_1)) * 2.0 / 4.0
    }

/// ∂area/∂x_2，量纲 `m`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn area__d_x_2(x_1: f64, x_2: f64, x_3: f64, y_1: f64, y_2: f64, y_3: f64) -> f64 {
        (y_3 - y_1) * 2.0 / 4.0
    }

/// ∂area/∂x_3，量纲 `m`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn area__d_x_3(x_1: f64, x_2: f64, x_3: f64, y_1: f64, y_2: f64, y_3: f64) -> f64 {
        -(y_2 - y_1) * 2.0 / 4.0
    }

/// ∂area/∂y_1，量纲 `m`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn area__d_y_1(x_1: f64, x_2: f64, x_3: f64, y_1: f64, y_2: f64, y_3: f64) -> f64 {
        (-(x_2 - x_1) + -(-(x_3 - x_1))) * 2.0 / 4.0
    }

/// ∂area/∂y_2，量纲 `m`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn area__d_y_2(x_1: f64, x_2: f64, x_3: f64, y_1: f64, y_2: f64, y_3: f64) -> f64 {
        -(x_3 - x_1) * 2.0 / 4.0
    }

/// ∂area/∂y_3，量纲 `m`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn area__d_y_3(x_1: f64, x_2: f64, x_3: f64, y_1: f64, y_2: f64, y_3: f64) -> f64 {
        (x_2 - x_1) * 2.0 / 4.0
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn area__f32(x_1: f32, x_2: f32, x_3: f32, y_1: f32, y_2: f32, y_3: f32) -> f32 {
        ((x_2 - x_1) * (y_3 - y_1) + -((x_3 - x_1) * (y_2 - y_1))) / 2.0f32
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn area__batch(x_1: &[f64], x_2: &[f64], x_3: &[f64], y_1: &[f64], y_2: &[f64], y_3: &[f64], out: &mut [f64]) {
    let __n = out.len().min(x_1.len()).min(x_2.len()).min(x_3.len()).min(y_1.len()).min(y_2.len()).min(y_3.len());
    for __row in 0..__n {
        out[__row] = area(x_1[__row], x_2[__row], x_3[__row], y_1[__row], y_2[__row], y_3[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn area__batch_n<const __HIMARI_LEN: usize>(x_1: &[f64; __HIMARI_LEN], x_2: &[f64; __HIMARI_LEN], x_3: &[f64; __HIMARI_LEN], y_1: &[f64; __HIMARI_LEN], y_2: &[f64; __HIMARI_LEN], y_3: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = area(x_1[__row], x_2[__row], x_3[__row], y_1[__row], y_2[__row], y_3[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const AREA__LATEX: &str = "\\frac{\\det(\\begin{bmatrix}\\mathrm{x_2} - \\mathrm{x_1} & \\mathrm{x_3} - \\mathrm{x_1} \\\\ \\mathrm{y_2} - \\mathrm{y_1} & \\mathrm{y_3} - \\mathrm{y_1}\\end{bmatrix})}{2}";

/// 结果量纲：`m`
///
/// ```text
/// \operatorname{trace}(\begin{bmatrix}\mathrm{x_2} - \mathrm{x_1} & \mathrm{x_3} - \mathrm{x_1} \\ \mathrm{y_2} - \mathrm{y_1} & \mathrm{y_3} - \mathrm{y_1}\end{bmatrix})
/// ```
///
/// 源：`formulas/fem_element.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn jac_trace(x_1: f64, x_2: f64, y_1: f64, y_3: f64) -> f64 {
        x_2 - x_1 + (y_3 - y_1)
    }

/// ∂jac_trace/∂x_1，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn jac_trace__d_x_1(x_1: f64, x_2: f64, y_1: f64, y_3: f64) -> f64 {
        -1.0
    }

/// ∂jac_trace/∂x_2，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn jac_trace__d_x_2(x_1: f64, x_2: f64, y_1: f64, y_3: f64) -> f64 {
        1.0
    }

/// ∂jac_trace/∂y_1，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn jac_trace__d_y_1(x_1: f64, x_2: f64, y_1: f64, y_3: f64) -> f64 {
        -1.0
    }

/// ∂jac_trace/∂y_3，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn jac_trace__d_y_3(x_1: f64, x_2: f64, y_1: f64, y_3: f64) -> f64 {
        1.0
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn jac_trace__f32(x_1: f32, x_2: f32, y_1: f32, y_3: f32) -> f32 {
        x_2 - x_1 + (y_3 - y_1)
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn jac_trace__batch(x_1: &[f64], x_2: &[f64], y_1: &[f64], y_3: &[f64], out: &mut [f64]) {
    let __n = out.len().min(x_1.len()).min(x_2.len()).min(y_1.len()).min(y_3.len());
    for __row in 0..__n {
        out[__row] = jac_trace(x_1[__row], x_2[__row], y_1[__row], y_3[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn jac_trace__batch_n<const __HIMARI_LEN: usize>(x_1: &[f64; __HIMARI_LEN], x_2: &[f64; __HIMARI_LEN], y_1: &[f64; __HIMARI_LEN], y_3: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = jac_trace(x_1[__row], x_2[__row], y_1[__row], y_3[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const JAC_TRACE__LATEX: &str = "\\operatorname{trace}(\\begin{bmatrix}\\mathrm{x_2} - \\mathrm{x_1} & \\mathrm{x_3} - \\mathrm{x_1} \\\\ \\mathrm{y_2} - \\mathrm{y_1} & \\mathrm{y_3} - \\mathrm{y_1}\\end{bmatrix})";

/// 结果量纲：`m`
///
/// ```text
/// \operatorname{adjoint}(\begin{bmatrix}\mathrm{x_2} - \mathrm{x_1} & \mathrm{x_3} - \mathrm{x_1} \\ \mathrm{y_2} - \mathrm{y_1} & \mathrm{y_3} - \mathrm{y_1}\end{bmatrix})
/// ```
///
/// 源：`formulas/fem_element.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn J_adj(x_1: f64, x_2: f64, x_3: f64, y_1: f64, y_2: f64, y_3: f64) -> Vec<Vec<f64>> {
        vec![vec![(x_2 - x_1), (y_2 - y_1)], vec![(x_3 - x_1), (y_3 - y_1)]]
    }

// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn J_adj__batch(x_1: &[f64], x_2: &[f64], x_3: &[f64], y_1: &[f64], y_2: &[f64], y_3: &[f64], out: &mut [Vec<Vec<f64>>]) {
    let __n = out.len().min(x_1.len()).min(x_2.len()).min(x_3.len()).min(y_1.len()).min(y_2.len()).min(y_3.len());
    for __row in 0..__n {
        out[__row] = J_adj(x_1[__row], x_2[__row], x_3[__row], y_1[__row], y_2[__row], y_3[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn J_adj__batch_n<const __HIMARI_LEN: usize>(x_1: &[f64; __HIMARI_LEN], x_2: &[f64; __HIMARI_LEN], x_3: &[f64; __HIMARI_LEN], y_1: &[f64; __HIMARI_LEN], y_2: &[f64; __HIMARI_LEN], y_3: &[f64; __HIMARI_LEN], out: &mut [Vec<Vec<f64>>; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = J_adj(x_1[__row], x_2[__row], x_3[__row], y_1[__row], y_2[__row], y_3[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const J_ADJ__LATEX: &str = "\\operatorname{adjoint}(\\begin{bmatrix}\\mathrm{x_2} - \\mathrm{x_1} & \\mathrm{x_3} - \\mathrm{x_1} \\\\ \\mathrm{y_2} - \\mathrm{y_1} & \\mathrm{y_3} - \\mathrm{y_1}\\end{bmatrix})";

/// 结果量纲：`m^-1`
///
/// ```text
/// \operatorname{inv}(\begin{bmatrix}\mathrm{x_2} - \mathrm{x_1} & \mathrm{x_3} - \mathrm{x_1} \\ \mathrm{y_2} - \mathrm{y_1} & \mathrm{y_3} - \mathrm{y_1}\end{bmatrix})
/// ```
///
/// 源：`formulas/fem_element.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn J_inv(x_1: f64, x_2: f64, x_3: f64, y_1: f64, y_2: f64, y_3: f64) -> Vec<Vec<f64>> {
        let __t0 = y_3 - y_1;
        let __t1 = x_2 - x_1;
        let __t2 = __t1 * __t0;
        let __t3 = x_3 - x_1;
        let __t4 = y_2 - y_1;
        let __t5 = __t3 * __t4;
        let __t6 = -__t5;
        let __t7 = __t2 + __t6;
        vec![vec![__t0 / __t7, -__t3 / __t7], vec![-__t4 / __t7, __t1 / __t7]]
    }

// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn J_inv__batch(x_1: &[f64], x_2: &[f64], x_3: &[f64], y_1: &[f64], y_2: &[f64], y_3: &[f64], out: &mut [Vec<Vec<f64>>]) {
    let __n = out.len().min(x_1.len()).min(x_2.len()).min(x_3.len()).min(y_1.len()).min(y_2.len()).min(y_3.len());
    for __row in 0..__n {
        out[__row] = J_inv(x_1[__row], x_2[__row], x_3[__row], y_1[__row], y_2[__row], y_3[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn J_inv__batch_n<const __HIMARI_LEN: usize>(x_1: &[f64; __HIMARI_LEN], x_2: &[f64; __HIMARI_LEN], x_3: &[f64; __HIMARI_LEN], y_1: &[f64; __HIMARI_LEN], y_2: &[f64; __HIMARI_LEN], y_3: &[f64; __HIMARI_LEN], out: &mut [Vec<Vec<f64>>; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = J_inv(x_1[__row], x_2[__row], x_3[__row], y_1[__row], y_2[__row], y_3[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const J_INV__LATEX: &str = "\\operatorname{inv}(\\begin{bmatrix}\\mathrm{x_2} - \\mathrm{x_1} & \\mathrm{x_3} - \\mathrm{x_1} \\\\ \\mathrm{y_2} - \\mathrm{y_1} & \\mathrm{y_3} - \\mathrm{y_1}\\end{bmatrix})";

/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），列 = 参数（声明顺序）。
///
/// 参数 = `x_1, x_2, x_3, y_1, y_2, y_3`；不依赖某参数的行在该列恒为 0。
/// 源：`formulas/fem_element.formula`
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn fem_element__jacobian(x_1: f64, x_2: f64, x_3: f64, y_1: f64, y_2: f64, y_3: f64) -> Vec<Vec<f64>> {
    vec![
        vec![area__d_x_1(x_1, x_2, x_3, y_1, y_2, y_3), area__d_x_2(x_1, x_2, x_3, y_1, y_2, y_3), area__d_x_3(x_1, x_2, x_3, y_1, y_2, y_3), area__d_y_1(x_1, x_2, x_3, y_1, y_2, y_3), area__d_y_2(x_1, x_2, x_3, y_1, y_2, y_3), area__d_y_3(x_1, x_2, x_3, y_1, y_2, y_3)],
        vec![jac_trace__d_x_1(x_1, x_2, y_1, y_3), jac_trace__d_x_2(x_1, x_2, y_1, y_3), 0.0, jac_trace__d_y_1(x_1, x_2, y_1, y_3), 0.0, jac_trace__d_y_3(x_1, x_2, y_1, y_3)]
    ]
}

/// 源：`formulas/heat.formula`（量纲 `1`）
/// 三对角阵 tridiag(a,b,c)_n（由模式矩阵生成）
#[allow(non_snake_case, clippy::too_many_arguments)]
pub fn heat(k: f64, n: usize) -> Vec<Vec<f64>> {
    let av = {
        -k
    };
    let bv = {
        2.0 * k
    };
    let cv = {
        -k
    };
    let mut m = vec![vec![0.0; n]; n];
    #[allow(clippy::needless_range_loop)]
    for __row in 0..n {
        m[__row][__row] = bv;
        let __next = __row + 1;
        if __next < n {
            m[__row][__next] = cv;
            m[__next][__row] = av;
        }
    }
    m
}

/// 结果量纲：`m^-2·A`
///
/// ```text
/// A \cdot \exp(-(\frac{\mathrm{E_a}}{R \cdot T}))
/// ```
///
/// 源：`formulas/i0_arrhenius.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn i0_arrhenius(A: f64, E_a: f64, R: f64, T: f64) -> f64 {
        A * (-(E_a / (R * T))).exp()
    }

/// ∂i0_arrhenius/∂A，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn i0_arrhenius__d_A(A: f64, E_a: f64, R: f64, T: f64) -> f64 {
        (-(E_a / (R * T))).exp()
    }

/// ∂i0_arrhenius/∂E_a，量纲 `kg^-1·m^-4·s^2·A·mol`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn i0_arrhenius__d_E_a(A: f64, E_a: f64, R: f64, T: f64) -> f64 {
        let __t0 = R * T;
        A * ((-(E_a / __t0)).exp() * -(__t0 / (__t0 * __t0)))
    }

/// ∂i0_arrhenius/∂R，量纲 `kg^-1·m^-4·s^2·A·K·mol`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn i0_arrhenius__d_R(A: f64, E_a: f64, R: f64, T: f64) -> f64 {
        let __t0 = R * T;
        A * ((-(E_a / __t0)).exp() * -(-(E_a * T) / (__t0 * __t0)))
    }

/// ∂i0_arrhenius/∂T，量纲 `m^-2·A·K^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn i0_arrhenius__d_T(A: f64, E_a: f64, R: f64, T: f64) -> f64 {
        let __t0 = R * T;
        A * ((-(E_a / __t0)).exp() * -(-(E_a * R) / (__t0 * __t0)))
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn i0_arrhenius__f32(A: f32, E_a: f32, R: f32, T: f32) -> f32 {
        A * (-(E_a / (R * T))).exp()
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn i0_arrhenius__batch(A: &[f64], E_a: &[f64], R: &[f64], T: &[f64], out: &mut [f64]) {
    let __n = out.len().min(A.len()).min(E_a.len()).min(R.len()).min(T.len());
    for __row in 0..__n {
        out[__row] = i0_arrhenius(A[__row], E_a[__row], R[__row], T[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn i0_arrhenius__batch_n<const __HIMARI_LEN: usize>(A: &[f64; __HIMARI_LEN], E_a: &[f64; __HIMARI_LEN], R: &[f64; __HIMARI_LEN], T: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = i0_arrhenius(A[__row], E_a[__row], R[__row], T[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const I0_ARRHENIUS__LATEX: &str = "A \\cdot \\exp(-(\\frac{\\mathrm{E_a}}{R \\cdot T}))";

/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），列 = 参数（声明顺序）。
///
/// 参数 = `A, E_a, R, T`；不依赖某参数的行在该列恒为 0。
/// 源：`formulas/i0_arrhenius.formula`
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn i0_arrhenius__jacobian(A: f64, E_a: f64, R: f64, T: f64) -> Vec<Vec<f64>> {
    vec![
        vec![i0_arrhenius__d_A(A, E_a, R, T), i0_arrhenius__d_E_a(A, E_a, R, T), i0_arrhenius__d_R(A, E_a, R, T), i0_arrhenius__d_T(A, E_a, R, T)]
    ]
}

/// 结果量纲：`1`
///
/// ```text
/// (\begin{bmatrix}1 & 2 \\ 3 & 4\end{bmatrix} \cdot \begin{bmatrix}5 & 6 \\ 7 & 8\end{bmatrix} + \begin{bmatrix}1 & 1 \\ 1 & 1\end{bmatrix})^{T}
/// ```
///
/// 源：`formulas/matalg.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn M() -> Vec<Vec<f64>> {
        vec![vec![5.0 + 2.0 * 7.0 + 1.0, 3.0 * 5.0 + 4.0 * 7.0 + 1.0], vec![6.0 + 2.0 * 8.0 + 1.0, 3.0 * 6.0 + 4.0 * 8.0 + 1.0]]
    }

// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn M__batch(out: &mut [Vec<Vec<f64>>]) {
    let __n = out.len();
    for __row in 0..__n {
        out[__row] = M();
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn M__batch_n<const __HIMARI_LEN: usize>(out: &mut [Vec<Vec<f64>>; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = M();
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const M__LATEX: &str = "(\\begin{bmatrix}1 & 2 \\\\ 3 & 4\\end{bmatrix} \\cdot \\begin{bmatrix}5 & 6 \\\\ 7 & 8\\end{bmatrix} + \\begin{bmatrix}1 & 1 \\\\ 1 & 1\\end{bmatrix})^{T}";

/// 结果量纲：`1`
///
/// ```text
/// k \cdot \begin{bmatrix}1 & 2 \\ 3 & 4\end{bmatrix}
/// ```
///
/// 源：`formulas/matalg.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn S(k: f64) -> Vec<Vec<f64>> {
        vec![vec![k, k * 2.0], vec![k * 3.0, k * 4.0]]
    }

// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn S__batch(k: &[f64], out: &mut [Vec<Vec<f64>>]) {
    let __n = out.len().min(k.len());
    for __row in 0..__n {
        out[__row] = S(k[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn S__batch_n<const __HIMARI_LEN: usize>(k: &[f64; __HIMARI_LEN], out: &mut [Vec<Vec<f64>>; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = S(k[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const S__LATEX: &str = "k \\cdot \\begin{bmatrix}1 & 2 \\\\ 3 & 4\\end{bmatrix}";

/// 结果量纲：`1`
///
/// ```text
/// \begin{bmatrix}a & b \\ c & d\end{bmatrix}^{2}
/// ```
///
/// 源：`formulas/matrix_power.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn M2(a: f64, b: f64, c: f64, d: f64) -> Vec<Vec<f64>> {
        vec![vec![a * a + b * c, a * b + b * d], vec![c * a + d * c, c * b + d * d]]
    }

// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn M2__batch(a: &[f64], b: &[f64], c: &[f64], d: &[f64], out: &mut [Vec<Vec<f64>>]) {
    let __n = out.len().min(a.len()).min(b.len()).min(c.len()).min(d.len());
    for __row in 0..__n {
        out[__row] = M2(a[__row], b[__row], c[__row], d[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn M2__batch_n<const __HIMARI_LEN: usize>(a: &[f64; __HIMARI_LEN], b: &[f64; __HIMARI_LEN], c: &[f64; __HIMARI_LEN], d: &[f64; __HIMARI_LEN], out: &mut [Vec<Vec<f64>>; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = M2(a[__row], b[__row], c[__row], d[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const M2__LATEX: &str = "\\begin{bmatrix}a & b \\\\ c & d\\end{bmatrix}^{2}";

/// 结果量纲：`m·s^-2`
///
/// ```text
/// \frac{F}{m}
/// ```
///
/// 源：`formulas/mech.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn accel(F: f64, m: f64) -> f64 {
        F / m
    }

/// ∂accel/∂F，量纲 `kg^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn accel__d_F(F: f64, m: f64) -> f64 {
        m / (m * m)
    }

/// ∂accel/∂m，量纲 `kg^-1·m·s^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn accel__d_m(F: f64, m: f64) -> f64 {
        -F / (m * m)
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn accel__f32(F: f32, m: f32) -> f32 {
        F / m
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn accel__batch(F: &[f64], m: &[f64], out: &mut [f64]) {
    let __n = out.len().min(F.len()).min(m.len());
    for __row in 0..__n {
        out[__row] = accel(F[__row], m[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn accel__batch_n<const __HIMARI_LEN: usize>(F: &[f64; __HIMARI_LEN], m: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = accel(F[__row], m[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const ACCEL__LATEX: &str = "\\frac{F}{m}";

/// 结果量纲：`m·s^-1`
///
/// ```text
/// (\frac{F}{m} \cdot r)^{0.5}
/// ```
///
/// 源：`formulas/mech.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn mech(F: f64, m: f64, r: f64) -> f64 {
        (F / m * r).sqrt()
    }

/// ∂mech/∂F，量纲 `kg^-1·s`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn mech__d_F(F: f64, m: f64, r: f64) -> f64 {
        0.5 * (F / m * r).powf(-0.5) * (m / (m * m) * r)
    }

/// ∂mech/∂m，量纲 `kg^-1·m·s^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn mech__d_m(F: f64, m: f64, r: f64) -> f64 {
        0.5 * (F / m * r).powf(-0.5) * (-F / (m * m) * r)
    }

/// ∂mech/∂r，量纲 `s^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn mech__d_r(F: f64, m: f64, r: f64) -> f64 {
        let __t0 = F / m;
        0.5 * (__t0 * r).powf(-0.5) * __t0
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn mech__f32(F: f32, m: f32, r: f32) -> f32 {
        (F / m * r).sqrt()
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn mech__batch(F: &[f64], m: &[f64], r: &[f64], out: &mut [f64]) {
    let __n = out.len().min(F.len()).min(m.len()).min(r.len());
    for __row in 0..__n {
        out[__row] = mech(F[__row], m[__row], r[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn mech__batch_n<const __HIMARI_LEN: usize>(F: &[f64; __HIMARI_LEN], m: &[f64; __HIMARI_LEN], r: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = mech(F[__row], m[__row], r[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const MECH__LATEX: &str = "(\\frac{F}{m} \\cdot r)^{0.5}";

/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），列 = 参数（声明顺序）。
///
/// 参数 = `F, m, r`；不依赖某参数的行在该列恒为 0。
/// 源：`formulas/mech.formula`
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn mech__jacobian(F: f64, m: f64, r: f64) -> Vec<Vec<f64>> {
    vec![
        vec![accel__d_F(F, m), accel__d_m(F, m), 0.0],
        vec![mech__d_F(F, m, r), mech__d_m(F, m, r), mech__d_r(F, m, r)]
    ]
}

/// 结果量纲：`mol`
///
/// ```text
/// \operatorname{solve}(\mathrm{k_on} \cdot E \cdot S - (\mathrm{k_off} + \mathrm{k_cat}) \cdot \mathrm{ES}, \mathrm{ES})
/// ```
///
/// 源：`formulas/michaelis.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn ES_ss(k_on: f64, k_off: f64, k_cat: f64, E: f64, S: f64) -> f64 {
        -(k_on * E * S / -(k_off + k_cat))
    }

/// ∂ES_ss/∂k_on，量纲 `s·mol^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn ES_ss__d_k_on(k_on: f64, k_off: f64, k_cat: f64, E: f64, S: f64) -> f64 {
        let __t0 = k_off + k_cat;
        let __t1 = -__t0;
        -(E * S * __t1 / (__t1 * __t1))
    }

/// ∂ES_ss/∂k_off，量纲 `s·mol`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn ES_ss__d_k_off(k_on: f64, k_off: f64, k_cat: f64, E: f64, S: f64) -> f64 {
        -(-(-(k_on * E * S)) / (-(k_off + k_cat) * -(k_off + k_cat)))
    }

/// ∂ES_ss/∂k_cat，量纲 `s·mol`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn ES_ss__d_k_cat(k_on: f64, k_off: f64, k_cat: f64, E: f64, S: f64) -> f64 {
        -(-(-(k_on * E * S)) / (-(k_off + k_cat) * -(k_off + k_cat)))
    }

/// ∂ES_ss/∂E，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn ES_ss__d_E(k_on: f64, k_off: f64, k_cat: f64, E: f64, S: f64) -> f64 {
        let __t0 = k_off + k_cat;
        let __t1 = -__t0;
        -(k_on * S * __t1 / (__t1 * __t1))
    }

/// ∂ES_ss/∂S，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn ES_ss__d_S(k_on: f64, k_off: f64, k_cat: f64, E: f64, S: f64) -> f64 {
        let __t0 = k_off + k_cat;
        let __t1 = -__t0;
        -(k_on * E * __t1 / (__t1 * __t1))
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn ES_ss__f32(k_on: f32, k_off: f32, k_cat: f32, E: f32, S: f32) -> f32 {
        -(k_on * E * S / -(k_off + k_cat))
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn ES_ss__batch(k_on: &[f64], k_off: &[f64], k_cat: &[f64], E: &[f64], S: &[f64], out: &mut [f64]) {
    let __n = out.len().min(k_on.len()).min(k_off.len()).min(k_cat.len()).min(E.len()).min(S.len());
    for __row in 0..__n {
        out[__row] = ES_ss(k_on[__row], k_off[__row], k_cat[__row], E[__row], S[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn ES_ss__batch_n<const __HIMARI_LEN: usize>(k_on: &[f64; __HIMARI_LEN], k_off: &[f64; __HIMARI_LEN], k_cat: &[f64; __HIMARI_LEN], E: &[f64; __HIMARI_LEN], S: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = ES_ss(k_on[__row], k_off[__row], k_cat[__row], E[__row], S[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const ES_SS__LATEX: &str = "\\operatorname{solve}(\\mathrm{k_on} \\cdot E \\cdot S - (\\mathrm{k_off} + \\mathrm{k_cat}) \\cdot \\mathrm{ES}, \\mathrm{ES})";

/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），列 = 参数（声明顺序）。
///
/// 参数 = `k_on, k_off, k_cat, E, S`；不依赖某参数的行在该列恒为 0。
/// 源：`formulas/michaelis.formula`
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn michaelis__jacobian(k_on: f64, k_off: f64, k_cat: f64, E: f64, S: f64) -> Vec<Vec<f64>> {
    vec![
        vec![ES_ss__d_k_on(k_on, k_off, k_cat, E, S), ES_ss__d_k_off(k_on, k_off, k_cat, E, S), ES_ss__d_k_cat(k_on, k_off, k_cat, E, S), ES_ss__d_E(k_on, k_off, k_cat, E, S), ES_ss__d_S(k_on, k_off, k_cat, E, S)]
    ]
}

/// 结果量纲：`kg·m^2·s^-3·A^-1`
///
/// ```text
/// \mathrm{E0} - \frac{R \cdot T}{n \cdot F} \cdot \ln(\frac{\mathrm{a_ox}}{\mathrm{a_red}})
/// ```
///
/// 源：`formulas/nernst.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn nernst(E0: f64, R: f64, T: f64, n: f64, F: f64, a_ox: f64, a_red: f64) -> f64 {
        E0 - R * T / (n * F) * (a_ox / a_red).ln()
    }

/// ∂nernst/∂E0，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn nernst__d_E0(E0: f64, R: f64, T: f64, n: f64, F: f64, a_ox: f64, a_red: f64) -> f64 {
        1.0
    }

/// ∂nernst/∂R，量纲 `s^-1·A^-1·K·mol`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn nernst__d_R(E0: f64, R: f64, T: f64, n: f64, F: f64, a_ox: f64, a_red: f64) -> f64 {
        let __t0 = n * F;
        -(T * __t0 / (__t0 * __t0) * (a_ox / a_red).ln())
    }

/// ∂nernst/∂T，量纲 `kg·m^2·s^-3·A^-1·K^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn nernst__d_T(E0: f64, R: f64, T: f64, n: f64, F: f64, a_ox: f64, a_red: f64) -> f64 {
        let __t0 = n * F;
        -(R * __t0 / (__t0 * __t0) * (a_ox / a_red).ln())
    }

/// ∂nernst/∂n，量纲 `kg·m^2·s^-3·A^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn nernst__d_n(E0: f64, R: f64, T: f64, n: f64, F: f64, a_ox: f64, a_red: f64) -> f64 {
        -(-(R * T * F) / ((n * F) * (n * F)) * (a_ox / a_red).ln())
    }

/// ∂nernst/∂F，量纲 `kg·m^2·s^-4·A^-2·mol`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn nernst__d_F(E0: f64, R: f64, T: f64, n: f64, F: f64, a_ox: f64, a_red: f64) -> f64 {
        -(-(R * T * n) / ((n * F) * (n * F)) * (a_ox / a_red).ln())
    }

/// ∂nernst/∂a_ox，量纲 `kg·m^2·s^-3·A^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn nernst__d_a_ox(E0: f64, R: f64, T: f64, n: f64, F: f64, a_ox: f64, a_red: f64) -> f64 {
        -(R * T / (n * F) * (a_red / (a_red * a_red) / (a_ox / a_red)))
    }

/// ∂nernst/∂a_red，量纲 `kg·m^2·s^-3·A^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn nernst__d_a_red(E0: f64, R: f64, T: f64, n: f64, F: f64, a_ox: f64, a_red: f64) -> f64 {
        -(R * T / (n * F) * (-a_ox / (a_red * a_red) / (a_ox / a_red)))
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn nernst__f32(E0: f32, R: f32, T: f32, n: f32, F: f32, a_ox: f32, a_red: f32) -> f32 {
        E0 - R * T / (n * F) * (a_ox / a_red).ln()
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn nernst__batch(E0: &[f64], R: &[f64], T: &[f64], n: &[f64], F: &[f64], a_ox: &[f64], a_red: &[f64], out: &mut [f64]) {
    let __n = out.len().min(E0.len()).min(R.len()).min(T.len()).min(n.len()).min(F.len()).min(a_ox.len()).min(a_red.len());
    for __row in 0..__n {
        out[__row] = nernst(E0[__row], R[__row], T[__row], n[__row], F[__row], a_ox[__row], a_red[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn nernst__batch_n<const __HIMARI_LEN: usize>(E0: &[f64; __HIMARI_LEN], R: &[f64; __HIMARI_LEN], T: &[f64; __HIMARI_LEN], n: &[f64; __HIMARI_LEN], F: &[f64; __HIMARI_LEN], a_ox: &[f64; __HIMARI_LEN], a_red: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = nernst(E0[__row], R[__row], T[__row], n[__row], F[__row], a_ox[__row], a_red[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const NERNST__LATEX: &str = "\\mathrm{E0} - \\frac{R \\cdot T}{n \\cdot F} \\cdot \\ln(\\frac{\\mathrm{a_ox}}{\\mathrm{a_red}})";

/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），列 = 参数（声明顺序）。
///
/// 参数 = `E0, R, T, n, F, a_ox, a_red`；不依赖某参数的行在该列恒为 0。
/// 源：`formulas/nernst.formula`
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn nernst__jacobian(E0: f64, R: f64, T: f64, n: f64, F: f64, a_ox: f64, a_red: f64) -> Vec<Vec<f64>> {
    vec![
        vec![nernst__d_E0(E0, R, T, n, F, a_ox, a_red), nernst__d_R(E0, R, T, n, F, a_ox, a_red), nernst__d_T(E0, R, T, n, F, a_ox, a_red), nernst__d_n(E0, R, T, n, F, a_ox, a_red), nernst__d_F(E0, R, T, n, F, a_ox, a_red), nernst__d_a_ox(E0, R, T, n, F, a_ox, a_red), nernst__d_a_red(E0, R, T, n, F, a_ox, a_red)]
    ]
}

/// 结果量纲：`1`
///
/// ```text
/// a \cdot x^{3} + b \cdot x
/// ```
///
/// 源：`formulas/polynomial.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn p3(x: f64, a: f64, b: f64) -> f64 {
        (a * x * x + b) * x
    }

/// ∂p3/∂x，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn p3__d_x(x: f64, a: f64, b: f64) -> f64 {
        a * (3.0 * (x * x)) + b
    }

/// ∂p3/∂a，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn p3__d_a(x: f64, a: f64, b: f64) -> f64 {
        (x).powi(3)
    }

/// ∂p3/∂b，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn p3__d_b(x: f64, a: f64, b: f64) -> f64 {
        x
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn p3__f32(x: f32, a: f32, b: f32) -> f32 {
        (a * x * x + b) * x
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn p3__batch(x: &[f64], a: &[f64], b: &[f64], out: &mut [f64]) {
    let __n = out.len().min(x.len()).min(a.len()).min(b.len());
    for __row in 0..__n {
        out[__row] = p3(x[__row], a[__row], b[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn p3__batch_n<const __HIMARI_LEN: usize>(x: &[f64; __HIMARI_LEN], a: &[f64; __HIMARI_LEN], b: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = p3(x[__row], a[__row], b[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const P3__LATEX: &str = "a \\cdot x^{3} + b \\cdot x";

/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），列 = 参数（声明顺序）。
///
/// 参数 = `x, a, b`；不依赖某参数的行在该列恒为 0。
/// 源：`formulas/polynomial.formula`
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn polynomial__jacobian(x: f64, a: f64, b: f64) -> Vec<Vec<f64>> {
    vec![
        vec![p3__d_x(x, a, b), p3__d_a(x, a, b), p3__d_b(x, a, b)]
    ]
}

/// 结果量纲：`m`
///
/// ```text
/// \operatorname{roots}(a \cdot x^{2} + b \cdot x + c, x)
/// ```
///
/// 源：`formulas/polyroots.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn roots_quad(a: f64, b: f64, c: f64) -> Result<Vec<Vec<f64>>, himari::runtime::NumericError> {
        (|| { himari::runtime::into_matrix(Ok::<_, himari::Error>(himari::Value::Matrix(vec![vec![himari::C::real(himari::runtime::real_quadratic_root_values(true, himari::runtime::scalar_result({
        a
    })?, himari::runtime::scalar_result({
        b
    })?, himari::runtime::scalar_result({
        c
    })?)?)],vec![himari::C::real(himari::runtime::real_quadratic_root_values(false, himari::runtime::scalar_result({
        a
    })?, himari::runtime::scalar_result({
        b
    })?, himari::runtime::scalar_result({
        c
    })?)?)]]))?) })()
    }

// Runtime matrix/vector formulas do not emit scalar batch or f32 variants.

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const ROOTS_QUAD__LATEX: &str = "\\operatorname{roots}(a \\cdot x^{2} + b \\cdot x + c, x)";

/// 结果量纲：`m`
///
/// ```text
/// \operatorname{roots}(b \cdot x + c, x)
/// ```
///
/// 源：`formulas/polyroots.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn roots_lin(b: f64, c: f64) -> Vec<Vec<f64>> {
        vec![vec![-(c / b)]]
    }

// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn roots_lin__batch(b: &[f64], c: &[f64], out: &mut [Vec<Vec<f64>>]) {
    let __n = out.len().min(b.len()).min(c.len());
    for __row in 0..__n {
        out[__row] = roots_lin(b[__row], c[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn roots_lin__batch_n<const __HIMARI_LEN: usize>(b: &[f64; __HIMARI_LEN], c: &[f64; __HIMARI_LEN], out: &mut [Vec<Vec<f64>>; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = roots_lin(b[__row], c[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const ROOTS_LIN__LATEX: &str = "\\operatorname{roots}(b \\cdot x + c, x)";

/// 结果量纲：`1`
///
/// ```text
/// \operatorname{poly_coeffs}(2 \cdot s^{2} - 3 \cdot s + 1, s)
/// ```
///
/// 源：`formulas/polyroots.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn coeffs_quad() -> Vec<Vec<f64>> {
        vec![vec![1.0, -3.0, 2.0]]
    }

// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn coeffs_quad__batch(out: &mut [Vec<Vec<f64>>]) {
    let __n = out.len();
    for __row in 0..__n {
        out[__row] = coeffs_quad();
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn coeffs_quad__batch_n<const __HIMARI_LEN: usize>(out: &mut [Vec<Vec<f64>>; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = coeffs_quad();
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const COEFFS_QUAD__LATEX: &str = "\\operatorname{poly_coeffs}(2 \\cdot s^{2} - 3 \\cdot s + 1, s)";

/// 结果量纲：`1`
///
/// ```text
/// \frac{\rho \cdot v \cdot L}{\mu}
/// ```
///
/// 源：`formulas/reynolds.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn Re(rho: f64, v: f64, L: f64, mu: f64) -> f64 {
        rho * v * L / mu
    }

/// ∂Re/∂rho，量纲 `kg^-1·m^3`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn Re__d_rho(rho: f64, v: f64, L: f64, mu: f64) -> f64 {
        v * L * mu / (mu * mu)
    }

/// ∂Re/∂v，量纲 `m^-1·s`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn Re__d_v(rho: f64, v: f64, L: f64, mu: f64) -> f64 {
        rho * L * mu / (mu * mu)
    }

/// ∂Re/∂L，量纲 `m^-1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn Re__d_L(rho: f64, v: f64, L: f64, mu: f64) -> f64 {
        rho * v * mu / (mu * mu)
    }

/// ∂Re/∂mu，量纲 `kg^-1·m·s`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn Re__d_mu(rho: f64, v: f64, L: f64, mu: f64) -> f64 {
        -(rho * v * L) / (mu * mu)
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn Re__f32(rho: f32, v: f32, L: f32, mu: f32) -> f32 {
        rho * v * L / mu
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn Re__batch(rho: &[f64], v: &[f64], L: &[f64], mu: &[f64], out: &mut [f64]) {
    let __n = out.len().min(rho.len()).min(v.len()).min(L.len()).min(mu.len());
    for __row in 0..__n {
        out[__row] = Re(rho[__row], v[__row], L[__row], mu[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn Re__batch_n<const __HIMARI_LEN: usize>(rho: &[f64; __HIMARI_LEN], v: &[f64; __HIMARI_LEN], L: &[f64; __HIMARI_LEN], mu: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = Re(rho[__row], v[__row], L[__row], mu[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const RE__LATEX: &str = "\\frac{\\rho \\cdot v \\cdot L}{\\mu}";

/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），列 = 参数（声明顺序）。
///
/// 参数 = `rho, v, L, mu`；不依赖某参数的行在该列恒为 0。
/// 源：`formulas/reynolds.formula`
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn reynolds__jacobian(rho: f64, v: f64, L: f64, mu: f64) -> Vec<Vec<f64>> {
    vec![
        vec![Re__d_rho(rho, v, L, mu), Re__d_v(rho, v, L, mu), Re__d_L(rho, v, L, mu), Re__d_mu(rho, v, L, mu)]
    ]
}

/// 结果量纲：`1`
///
/// ```text
/// \operatorname{clamp}(\mathrm{drive}, 0, \mathrm{ceiling})
/// ```
///
/// 源：`formulas/saturate.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn sat(drive: f64, ceiling: f64) -> f64 {
        (drive).max(0.0).min(ceiling)
    }

// 不生成解析导数：表达式含不可符号化的算子（clamp）。

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn sat__f32(drive: f32, ceiling: f32) -> f32 {
        (drive).max(0.0f32).min(ceiling)
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn sat__batch(drive: &[f64], ceiling: &[f64], out: &mut [f64]) {
    let __n = out.len().min(drive.len()).min(ceiling.len());
    for __row in 0..__n {
        out[__row] = sat(drive[__row], ceiling[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn sat__batch_n<const __HIMARI_LEN: usize>(drive: &[f64; __HIMARI_LEN], ceiling: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = sat(drive[__row], ceiling[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const SAT__LATEX: &str = "\\operatorname{clamp}(\\mathrm{drive}, 0, \\mathrm{ceiling})";

/// 结果量纲：`1`
///
/// ```text
/// \max(\mathrm{drive}, \mathrm{ceiling})
/// ```
///
/// 源：`formulas/saturate.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn peak(drive: f64, ceiling: f64) -> f64 {
        (drive).max(ceiling)
    }

// 不生成解析导数：表达式含不可符号化的算子（max）。

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn peak__f32(drive: f32, ceiling: f32) -> f32 {
        (drive).max(ceiling)
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn peak__batch(drive: &[f64], ceiling: &[f64], out: &mut [f64]) {
    let __n = out.len().min(drive.len()).min(ceiling.len());
    for __row in 0..__n {
        out[__row] = peak(drive[__row], ceiling[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn peak__batch_n<const __HIMARI_LEN: usize>(drive: &[f64; __HIMARI_LEN], ceiling: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = peak(drive[__row], ceiling[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const PEAK__LATEX: &str = "\\max(\\mathrm{drive}, \\mathrm{ceiling})";

/// 结果量纲：`1`
///
/// ```text
/// \min(\mathrm{drive}, \mathrm{ceiling})
/// ```
///
/// 源：`formulas/saturate.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn floor_(drive: f64, ceiling: f64) -> f64 {
        (drive).min(ceiling)
    }

// 不生成解析导数：表达式含不可符号化的算子（min）。

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn floor___f32(drive: f32, ceiling: f32) -> f32 {
        (drive).min(ceiling)
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn floor___batch(drive: &[f64], ceiling: &[f64], out: &mut [f64]) {
    let __n = out.len().min(drive.len()).min(ceiling.len());
    for __row in 0..__n {
        out[__row] = floor_(drive[__row], ceiling[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn floor___batch_n<const __HIMARI_LEN: usize>(drive: &[f64; __HIMARI_LEN], ceiling: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = floor_(drive[__row], ceiling[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const FLOOR___LATEX: &str = "\\min(\\mathrm{drive}, \\mathrm{ceiling})";

/// 结果量纲：`m`
///
/// ```text
/// \operatorname{solve}(\begin{bmatrix}(\mathrm{k1} + \mathrm{k2}) \cdot \mathrm{x1} - \mathrm{k2} \cdot \mathrm{x2} - \mathrm{F1} & -\mathrm{k2} \cdot \mathrm{x1} + (\mathrm{k2} + \mathrm{k3}) \cdot \mathrm{x2} - \mathrm{F2}\end{bmatrix}, \begin{bmatrix}\mathrm{x1} & \mathrm{x2}\end{bmatrix})
/// ```
///
/// 源：`formulas/springs.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn x(k1: f64, k2: f64, k3: f64, F1: f64, F2: f64) -> Vec<Vec<f64>> {
        let __t0 = k1 + k2;
        let __t1 = F1 / __t0;
        let __t2 = -k2;
        let __t3 = __t2 / __t0;
        let __t4 = __t2 * __t1;
        let __t5 = F2 - __t4;
        let __t6 = k2 + k3;
        let __t7 = __t2 * __t3;
        let __t8 = __t6 - __t7;
        let __t9 = __t5 / __t8;
        vec![vec![__t1 - __t3 * __t9], vec![__t9]]
    }

// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn x__batch(k1: &[f64], k2: &[f64], k3: &[f64], F1: &[f64], F2: &[f64], out: &mut [Vec<Vec<f64>>]) {
    let __n = out.len().min(k1.len()).min(k2.len()).min(k3.len()).min(F1.len()).min(F2.len());
    for __row in 0..__n {
        out[__row] = x(k1[__row], k2[__row], k3[__row], F1[__row], F2[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn x__batch_n<const __HIMARI_LEN: usize>(k1: &[f64; __HIMARI_LEN], k2: &[f64; __HIMARI_LEN], k3: &[f64; __HIMARI_LEN], F1: &[f64; __HIMARI_LEN], F2: &[f64; __HIMARI_LEN], out: &mut [Vec<Vec<f64>>; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = x(k1[__row], k2[__row], k3[__row], F1[__row], F2[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const X__LATEX: &str = "\\operatorname{solve}(\\begin{bmatrix}(\\mathrm{k1} + \\mathrm{k2}) \\cdot \\mathrm{x1} - \\mathrm{k2} \\cdot \\mathrm{x2} - \\mathrm{F1} & -\\mathrm{k2} \\cdot \\mathrm{x1} + (\\mathrm{k2} + \\mathrm{k3}) \\cdot \\mathrm{x2} - \\mathrm{F2}\\end{bmatrix}, \\begin{bmatrix}\\mathrm{x1} & \\mathrm{x2}\\end{bmatrix})";

/// 结果量纲：`m`
///
/// ```text
/// \operatorname{solve}(\begin{bmatrix}\mathrm{k1} \cdot (u - v) - \mathrm{F1} & u - L\end{bmatrix}, \begin{bmatrix}u & v\end{bmatrix})
/// ```
///
/// 源：`formulas/springs.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn mixed(k1: f64, F1: f64, L: f64) -> Vec<Vec<f64>> {
        let __t0 = F1 / k1;
        let __t1 = -k1;
        let __t2 = __t1 / k1;
        let __t3 = L - __t0;
        let __t4 = -__t2;
        let __t5 = __t3 / __t4;
        vec![vec![__t0 - __t2 * __t5], vec![__t5]]
    }

// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn mixed__batch(k1: &[f64], F1: &[f64], L: &[f64], out: &mut [Vec<Vec<f64>>]) {
    let __n = out.len().min(k1.len()).min(F1.len()).min(L.len());
    for __row in 0..__n {
        out[__row] = mixed(k1[__row], F1[__row], L[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn mixed__batch_n<const __HIMARI_LEN: usize>(k1: &[f64; __HIMARI_LEN], F1: &[f64; __HIMARI_LEN], L: &[f64; __HIMARI_LEN], out: &mut [Vec<Vec<f64>>; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = mixed(k1[__row], F1[__row], L[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const MIXED__LATEX: &str = "\\operatorname{solve}(\\begin{bmatrix}\\mathrm{k1} \\cdot (u - v) - \\mathrm{F1} & u - L\\end{bmatrix}, \\begin{bmatrix}u & v\\end{bmatrix})";

/// 结果量纲：`m`
///
/// ```text
/// 2 \cdot \operatorname{solve}(\mathrm{k1} \cdot \mathrm{x1} - \mathrm{F1}, \mathrm{x1})
/// ```
///
/// 源：`formulas/springs.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn zz(k1: f64, F1: f64) -> f64 {
        2.0 * -(-F1 / k1)
    }

/// ∂zz/∂k1，量纲 `kg^-1·m·s^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn zz__d_k1(k1: f64, F1: f64) -> f64 {
        2.0 * -(-(-F1) / (k1 * k1))
    }

/// ∂zz/∂F1，量纲 `kg^-1·s^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn zz__d_F1(k1: f64, F1: f64) -> f64 {
        2.0 * -(-k1 / (k1 * k1))
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn zz__f32(k1: f32, F1: f32) -> f32 {
        2.0f32 * -(-F1 / k1)
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn zz__batch(k1: &[f64], F1: &[f64], out: &mut [f64]) {
    let __n = out.len().min(k1.len()).min(F1.len());
    for __row in 0..__n {
        out[__row] = zz(k1[__row], F1[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn zz__batch_n<const __HIMARI_LEN: usize>(k1: &[f64; __HIMARI_LEN], F1: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = zz(k1[__row], F1[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const ZZ__LATEX: &str = "2 \\cdot \\operatorname{solve}(\\mathrm{k1} \\cdot \\mathrm{x1} - \\mathrm{F1}, \\mathrm{x1})";

/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），列 = 参数（声明顺序）。
///
/// 参数 = `k1, F1`；不依赖某参数的行在该列恒为 0。
/// 源：`formulas/springs.formula`
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn springs__jacobian(k1: f64, F1: f64) -> Vec<Vec<f64>> {
    vec![
        vec![zz__d_k1(k1, F1), zz__d_F1(k1, F1)]
    ]
}

/// 结果量纲：`kg·m^2·s^-3·A^-1`
///
/// ```text
/// a + b \cdot \ln(\frac{i}{\mathrm{i_0}})
/// ```
///
/// 源：`formulas/tafel.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn tafel(a: f64, b: f64, i: f64, i_0: f64) -> f64 {
        a + b * (i / i_0).ln()
    }

/// ∂tafel/∂a，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn tafel__d_a(a: f64, b: f64, i: f64, i_0: f64) -> f64 {
        1.0
    }

/// ∂tafel/∂b，量纲 `1`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn tafel__d_b(a: f64, b: f64, i: f64, i_0: f64) -> f64 {
        (i / i_0).ln()
    }

/// ∂tafel/∂i，量纲 `kg·m^4·s^-3·A^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn tafel__d_i(a: f64, b: f64, i: f64, i_0: f64) -> f64 {
        b * (i_0 / (i_0 * i_0) / (i / i_0))
    }

/// ∂tafel/∂i_0，量纲 `kg·m^4·s^-3·A^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn tafel__d_i_0(a: f64, b: f64, i: f64, i_0: f64) -> f64 {
        b * (-i / (i_0 * i_0) / (i / i_0))
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn tafel__f32(a: f32, b: f32, i: f32, i_0: f32) -> f32 {
        a + b * (i / i_0).ln()
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn tafel__batch(a: &[f64], b: &[f64], i: &[f64], i_0: &[f64], out: &mut [f64]) {
    let __n = out.len().min(a.len()).min(b.len()).min(i.len()).min(i_0.len());
    for __row in 0..__n {
        out[__row] = tafel(a[__row], b[__row], i[__row], i_0[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn tafel__batch_n<const __HIMARI_LEN: usize>(a: &[f64; __HIMARI_LEN], b: &[f64; __HIMARI_LEN], i: &[f64; __HIMARI_LEN], i_0: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = tafel(a[__row], b[__row], i[__row], i_0[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const TAFEL__LATEX: &str = "a + b \\cdot \\ln(\\frac{i}{\\mathrm{i_0}})";

/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），列 = 参数（声明顺序）。
///
/// 参数 = `a, b, i, i_0`；不依赖某参数的行在该列恒为 0。
/// 源：`formulas/tafel.formula`
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn tafel__jacobian(a: f64, b: f64, i: f64, i_0: f64) -> Vec<Vec<f64>> {
    vec![
        vec![tafel__d_a(a, b, i, i_0), tafel__d_b(a, b, i, i_0), tafel__d_i(a, b, i, i_0), tafel__d_i_0(a, b, i, i_0)]
    ]
}

/// 结果量纲：`m`
///
/// ```text
/// \begin{bmatrix}\mathrm{x_2} - \mathrm{x_1} & \mathrm{x_3} - \mathrm{x_1} & \mathrm{x_4} - \mathrm{x_1} \\ \mathrm{y_2} - \mathrm{y_1} & \mathrm{y_3} - \mathrm{y_1} & \mathrm{y_4} - \mathrm{y_1} \\ \mathrm{z_2} - \mathrm{z_1} & \mathrm{z_3} - \mathrm{z_1} & \mathrm{z_4} - \mathrm{z_1}\end{bmatrix}
/// ```
///
/// 源：`formulas/tet_element.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn J3(x_1: f64, x_2: f64, x_3: f64, x_4: f64, y_1: f64, y_2: f64, y_3: f64, y_4: f64, z_1: f64, z_2: f64, z_3: f64, z_4: f64) -> Vec<Vec<f64>> {
        vec![vec![x_2 - x_1, x_3 - x_1, x_4 - x_1], vec![y_2 - y_1, y_3 - y_1, y_4 - y_1], vec![z_2 - z_1, z_3 - z_1, z_4 - z_1]]
    }

// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn J3__batch(x_1: &[f64], x_2: &[f64], x_3: &[f64], x_4: &[f64], y_1: &[f64], y_2: &[f64], y_3: &[f64], y_4: &[f64], z_1: &[f64], z_2: &[f64], z_3: &[f64], z_4: &[f64], out: &mut [Vec<Vec<f64>>]) {
    let __n = out.len().min(x_1.len()).min(x_2.len()).min(x_3.len()).min(x_4.len()).min(y_1.len()).min(y_2.len()).min(y_3.len()).min(y_4.len()).min(z_1.len()).min(z_2.len()).min(z_3.len()).min(z_4.len());
    for __row in 0..__n {
        out[__row] = J3(x_1[__row], x_2[__row], x_3[__row], x_4[__row], y_1[__row], y_2[__row], y_3[__row], y_4[__row], z_1[__row], z_2[__row], z_3[__row], z_4[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn J3__batch_n<const __HIMARI_LEN: usize>(x_1: &[f64; __HIMARI_LEN], x_2: &[f64; __HIMARI_LEN], x_3: &[f64; __HIMARI_LEN], x_4: &[f64; __HIMARI_LEN], y_1: &[f64; __HIMARI_LEN], y_2: &[f64; __HIMARI_LEN], y_3: &[f64; __HIMARI_LEN], y_4: &[f64; __HIMARI_LEN], z_1: &[f64; __HIMARI_LEN], z_2: &[f64; __HIMARI_LEN], z_3: &[f64; __HIMARI_LEN], z_4: &[f64; __HIMARI_LEN], out: &mut [Vec<Vec<f64>>; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = J3(x_1[__row], x_2[__row], x_3[__row], x_4[__row], y_1[__row], y_2[__row], y_3[__row], y_4[__row], z_1[__row], z_2[__row], z_3[__row], z_4[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const J3__LATEX: &str = "\\begin{bmatrix}\\mathrm{x_2} - \\mathrm{x_1} & \\mathrm{x_3} - \\mathrm{x_1} & \\mathrm{x_4} - \\mathrm{x_1} \\\\ \\mathrm{y_2} - \\mathrm{y_1} & \\mathrm{y_3} - \\mathrm{y_1} & \\mathrm{y_4} - \\mathrm{y_1} \\\\ \\mathrm{z_2} - \\mathrm{z_1} & \\mathrm{z_3} - \\mathrm{z_1} & \\mathrm{z_4} - \\mathrm{z_1}\\end{bmatrix}";

/// 结果量纲：`m^3`
///
/// ```text
/// \frac{\det(\begin{bmatrix}\mathrm{x_2} - \mathrm{x_1} & \mathrm{x_3} - \mathrm{x_1} & \mathrm{x_4} - \mathrm{x_1} \\ \mathrm{y_2} - \mathrm{y_1} & \mathrm{y_3} - \mathrm{y_1} & \mathrm{y_4} - \mathrm{y_1} \\ \mathrm{z_2} - \mathrm{z_1} & \mathrm{z_3} - \mathrm{z_1} & \mathrm{z_4} - \mathrm{z_1}\end{bmatrix})}{6}
/// ```
///
/// 源：`formulas/tet_element.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn vol(x_1: f64, x_2: f64, x_3: f64, x_4: f64, y_1: f64, y_2: f64, y_3: f64, y_4: f64, z_1: f64, z_2: f64, z_3: f64, z_4: f64) -> f64 {
        let __t0 = y_3 - y_1;
        let __t1 = z_4 - z_1;
        let __t2 = y_4 - y_1;
        let __t3 = z_3 - z_1;
        let __t4 = y_2 - y_1;
        let __t5 = z_2 - z_1;
        ((x_2 - x_1) * (__t0 * __t1 + -(__t2 * __t3)) + -((x_3 - x_1) * (__t4 * __t1 + -(__t2 * __t5))) + (x_4 - x_1) * (__t4 * __t3 + -(__t0 * __t5))) / 6.0
    }

/// ∂vol/∂x_1，量纲 `m^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn vol__d_x_1(x_1: f64, x_2: f64, x_3: f64, x_4: f64, y_1: f64, y_2: f64, y_3: f64, y_4: f64, z_1: f64, z_2: f64, z_3: f64, z_4: f64) -> f64 {
        let __t0 = y_3 - y_1;
        let __t1 = z_4 - z_1;
        let __t2 = y_4 - y_1;
        let __t3 = z_3 - z_1;
        let __t4 = y_2 - y_1;
        let __t5 = z_2 - z_1;
        (-(__t0 * __t1 + -(__t2 * __t3)) + (__t4 * __t1 + -(__t2 * __t5)) + -(__t4 * __t3 + -(__t0 * __t5))) * 6.0 / 36.0
    }

/// ∂vol/∂x_2，量纲 `m^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn vol__d_x_2(x_1: f64, x_2: f64, x_3: f64, x_4: f64, y_1: f64, y_2: f64, y_3: f64, y_4: f64, z_1: f64, z_2: f64, z_3: f64, z_4: f64) -> f64 {
        ((y_3 - y_1) * (z_4 - z_1) + -((y_4 - y_1) * (z_3 - z_1))) * 6.0 / 36.0
    }

/// ∂vol/∂x_3，量纲 `m^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn vol__d_x_3(x_1: f64, x_2: f64, x_3: f64, x_4: f64, y_1: f64, y_2: f64, y_3: f64, y_4: f64, z_1: f64, z_2: f64, z_3: f64, z_4: f64) -> f64 {
        -((y_2 - y_1) * (z_4 - z_1) + -((y_4 - y_1) * (z_2 - z_1))) * 6.0 / 36.0
    }

/// ∂vol/∂x_4，量纲 `m^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn vol__d_x_4(x_1: f64, x_2: f64, x_3: f64, x_4: f64, y_1: f64, y_2: f64, y_3: f64, y_4: f64, z_1: f64, z_2: f64, z_3: f64, z_4: f64) -> f64 {
        ((y_2 - y_1) * (z_3 - z_1) + -((y_3 - y_1) * (z_2 - z_1))) * 6.0 / 36.0
    }

/// ∂vol/∂y_1，量纲 `m^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn vol__d_y_1(x_1: f64, x_2: f64, x_3: f64, x_4: f64, y_1: f64, y_2: f64, y_3: f64, y_4: f64, z_1: f64, z_2: f64, z_3: f64, z_4: f64) -> f64 {
        let __t0 = z_4 - z_1;
        let __t1 = -__t0;
        let __t2 = z_3 - z_1;
        let __t3 = z_2 - z_1;
        ((x_2 - x_1) * (__t1 + __t2) + -((x_3 - x_1) * (__t1 + __t3)) + (x_4 - x_1) * (-__t2 + __t3)) * 6.0 / 36.0
    }

/// ∂vol/∂y_2，量纲 `m^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn vol__d_y_2(x_1: f64, x_2: f64, x_3: f64, x_4: f64, y_1: f64, y_2: f64, y_3: f64, y_4: f64, z_1: f64, z_2: f64, z_3: f64, z_4: f64) -> f64 {
        (-((x_3 - x_1) * (z_4 - z_1)) + (x_4 - x_1) * (z_3 - z_1)) * 6.0 / 36.0
    }

/// ∂vol/∂y_3，量纲 `m^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn vol__d_y_3(x_1: f64, x_2: f64, x_3: f64, x_4: f64, y_1: f64, y_2: f64, y_3: f64, y_4: f64, z_1: f64, z_2: f64, z_3: f64, z_4: f64) -> f64 {
        ((x_2 - x_1) * (z_4 - z_1) + (x_4 - x_1) * -(z_2 - z_1)) * 6.0 / 36.0
    }

/// ∂vol/∂y_4，量纲 `m^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn vol__d_y_4(x_1: f64, x_2: f64, x_3: f64, x_4: f64, y_1: f64, y_2: f64, y_3: f64, y_4: f64, z_1: f64, z_2: f64, z_3: f64, z_4: f64) -> f64 {
        ((x_2 - x_1) * -(z_3 - z_1) + -((x_3 - x_1) * -(z_2 - z_1))) * 6.0 / 36.0
    }

/// ∂vol/∂z_1，量纲 `m^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn vol__d_z_1(x_1: f64, x_2: f64, x_3: f64, x_4: f64, y_1: f64, y_2: f64, y_3: f64, y_4: f64, z_1: f64, z_2: f64, z_3: f64, z_4: f64) -> f64 {
        let __t0 = y_3 - y_1;
        let __t1 = -__t0;
        let __t2 = y_4 - y_1;
        let __t3 = -__t2;
        let __t4 = -__t3;
        let __t5 = y_2 - y_1;
        let __t6 = -__t5;
        ((x_2 - x_1) * (__t1 + __t4) + -((x_3 - x_1) * (__t6 + __t4)) + (x_4 - x_1) * (__t6 + -__t1)) * 6.0 / 36.0
    }

/// ∂vol/∂z_2，量纲 `m^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn vol__d_z_2(x_1: f64, x_2: f64, x_3: f64, x_4: f64, y_1: f64, y_2: f64, y_3: f64, y_4: f64, z_1: f64, z_2: f64, z_3: f64, z_4: f64) -> f64 {
        (-((x_3 - x_1) * -(y_4 - y_1)) + (x_4 - x_1) * -(y_3 - y_1)) * 6.0 / 36.0
    }

/// ∂vol/∂z_3，量纲 `m^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn vol__d_z_3(x_1: f64, x_2: f64, x_3: f64, x_4: f64, y_1: f64, y_2: f64, y_3: f64, y_4: f64, z_1: f64, z_2: f64, z_3: f64, z_4: f64) -> f64 {
        ((x_2 - x_1) * -(y_4 - y_1) + (x_4 - x_1) * (y_2 - y_1)) * 6.0 / 36.0
    }

/// ∂vol/∂z_4，量纲 `m^2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn vol__d_z_4(x_1: f64, x_2: f64, x_3: f64, x_4: f64, y_1: f64, y_2: f64, y_3: f64, y_4: f64, z_1: f64, z_2: f64, z_3: f64, z_4: f64) -> f64 {
        ((x_2 - x_1) * (y_3 - y_1) + -((x_3 - x_1) * (y_2 - y_1))) * 6.0 / 36.0
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn vol__f32(x_1: f32, x_2: f32, x_3: f32, x_4: f32, y_1: f32, y_2: f32, y_3: f32, y_4: f32, z_1: f32, z_2: f32, z_3: f32, z_4: f32) -> f32 {
        let __t0 = y_3 - y_1;
        let __t1 = z_4 - z_1;
        let __t2 = y_4 - y_1;
        let __t3 = z_3 - z_1;
        let __t4 = y_2 - y_1;
        let __t5 = z_2 - z_1;
        ((x_2 - x_1) * (__t0 * __t1 + -(__t2 * __t3)) + -((x_3 - x_1) * (__t4 * __t1 + -(__t2 * __t5))) + (x_4 - x_1) * (__t4 * __t3 + -(__t0 * __t5))) / 6.0f32
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn vol__batch(x_1: &[f64], x_2: &[f64], x_3: &[f64], x_4: &[f64], y_1: &[f64], y_2: &[f64], y_3: &[f64], y_4: &[f64], z_1: &[f64], z_2: &[f64], z_3: &[f64], z_4: &[f64], out: &mut [f64]) {
    let __n = out.len().min(x_1.len()).min(x_2.len()).min(x_3.len()).min(x_4.len()).min(y_1.len()).min(y_2.len()).min(y_3.len()).min(y_4.len()).min(z_1.len()).min(z_2.len()).min(z_3.len()).min(z_4.len());
    for __row in 0..__n {
        out[__row] = vol(x_1[__row], x_2[__row], x_3[__row], x_4[__row], y_1[__row], y_2[__row], y_3[__row], y_4[__row], z_1[__row], z_2[__row], z_3[__row], z_4[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn vol__batch_n<const __HIMARI_LEN: usize>(x_1: &[f64; __HIMARI_LEN], x_2: &[f64; __HIMARI_LEN], x_3: &[f64; __HIMARI_LEN], x_4: &[f64; __HIMARI_LEN], y_1: &[f64; __HIMARI_LEN], y_2: &[f64; __HIMARI_LEN], y_3: &[f64; __HIMARI_LEN], y_4: &[f64; __HIMARI_LEN], z_1: &[f64; __HIMARI_LEN], z_2: &[f64; __HIMARI_LEN], z_3: &[f64; __HIMARI_LEN], z_4: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = vol(x_1[__row], x_2[__row], x_3[__row], x_4[__row], y_1[__row], y_2[__row], y_3[__row], y_4[__row], z_1[__row], z_2[__row], z_3[__row], z_4[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const VOL__LATEX: &str = "\\frac{\\det(\\begin{bmatrix}\\mathrm{x_2} - \\mathrm{x_1} & \\mathrm{x_3} - \\mathrm{x_1} & \\mathrm{x_4} - \\mathrm{x_1} \\\\ \\mathrm{y_2} - \\mathrm{y_1} & \\mathrm{y_3} - \\mathrm{y_1} & \\mathrm{y_4} - \\mathrm{y_1} \\\\ \\mathrm{z_2} - \\mathrm{z_1} & \\mathrm{z_3} - \\mathrm{z_1} & \\mathrm{z_4} - \\mathrm{z_1}\\end{bmatrix})}{6}";

/// 结果量纲：`m^-1`
///
/// ```text
/// \operatorname{inv}(\begin{bmatrix}\mathrm{x_2} - \mathrm{x_1} & \mathrm{x_3} - \mathrm{x_1} & \mathrm{x_4} - \mathrm{x_1} \\ \mathrm{y_2} - \mathrm{y_1} & \mathrm{y_3} - \mathrm{y_1} & \mathrm{y_4} - \mathrm{y_1} \\ \mathrm{z_2} - \mathrm{z_1} & \mathrm{z_3} - \mathrm{z_1} & \mathrm{z_4} - \mathrm{z_1}\end{bmatrix})
/// ```
///
/// 源：`formulas/tet_element.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn J3_inv(x_1: f64, x_2: f64, x_3: f64, x_4: f64, y_1: f64, y_2: f64, y_3: f64, y_4: f64, z_1: f64, z_2: f64, z_3: f64, z_4: f64) -> Vec<Vec<f64>> {
        let __t0 = y_3 - y_1;
        let __t1 = z_4 - z_1;
        let __t2 = __t0 * __t1;
        let __t3 = y_4 - y_1;
        let __t4 = z_3 - z_1;
        let __t5 = __t3 * __t4;
        let __t6 = -__t5;
        let __t7 = __t2 + __t6;
        let __t8 = x_2 - x_1;
        let __t9 = __t8 * __t7;
        let __t10 = x_3 - x_1;
        let __t11 = y_2 - y_1;
        let __t12 = __t11 * __t1;
        let __t13 = z_2 - z_1;
        let __t14 = __t3 * __t13;
        let __t15 = -__t14;
        let __t16 = __t12 + __t15;
        let __t17 = __t10 * __t16;
        let __t18 = -__t17;
        let __t19 = __t9 + __t18;
        let __t20 = x_4 - x_1;
        let __t21 = __t11 * __t4;
        let __t22 = __t0 * __t13;
        let __t23 = -__t22;
        let __t24 = __t21 + __t23;
        let __t25 = __t20 * __t24;
        let __t26 = __t19 + __t25;
        vec![vec![__t7 / __t26, -(__t10 * __t1 + -(__t20 * __t4)) / __t26, (__t10 * __t3 + -(__t20 * __t0)) / __t26], vec![-__t16 / __t26, (__t8 * __t1 + -(__t20 * __t13)) / __t26, -(__t8 * __t3 + -(__t20 * __t11)) / __t26], vec![__t24 / __t26, -(__t8 * __t4 + -(__t10 * __t13)) / __t26, (__t8 * __t0 + -(__t10 * __t11)) / __t26]]
    }

// 矩阵值公式不生成解析导数：元素级偏导不是矩阵（雅可比才是），该车道待做。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn J3_inv__batch(x_1: &[f64], x_2: &[f64], x_3: &[f64], x_4: &[f64], y_1: &[f64], y_2: &[f64], y_3: &[f64], y_4: &[f64], z_1: &[f64], z_2: &[f64], z_3: &[f64], z_4: &[f64], out: &mut [Vec<Vec<f64>>]) {
    let __n = out.len().min(x_1.len()).min(x_2.len()).min(x_3.len()).min(x_4.len()).min(y_1.len()).min(y_2.len()).min(y_3.len()).min(y_4.len()).min(z_1.len()).min(z_2.len()).min(z_3.len()).min(z_4.len());
    for __row in 0..__n {
        out[__row] = J3_inv(x_1[__row], x_2[__row], x_3[__row], x_4[__row], y_1[__row], y_2[__row], y_3[__row], y_4[__row], z_1[__row], z_2[__row], z_3[__row], z_4[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn J3_inv__batch_n<const __HIMARI_LEN: usize>(x_1: &[f64; __HIMARI_LEN], x_2: &[f64; __HIMARI_LEN], x_3: &[f64; __HIMARI_LEN], x_4: &[f64; __HIMARI_LEN], y_1: &[f64; __HIMARI_LEN], y_2: &[f64; __HIMARI_LEN], y_3: &[f64; __HIMARI_LEN], y_4: &[f64; __HIMARI_LEN], z_1: &[f64; __HIMARI_LEN], z_2: &[f64; __HIMARI_LEN], z_3: &[f64; __HIMARI_LEN], z_4: &[f64; __HIMARI_LEN], out: &mut [Vec<Vec<f64>>; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = J3_inv(x_1[__row], x_2[__row], x_3[__row], x_4[__row], y_1[__row], y_2[__row], y_3[__row], y_4[__row], z_1[__row], z_2[__row], z_3[__row], z_4[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const J3_INV__LATEX: &str = "\\operatorname{inv}(\\begin{bmatrix}\\mathrm{x_2} - \\mathrm{x_1} & \\mathrm{x_3} - \\mathrm{x_1} & \\mathrm{x_4} - \\mathrm{x_1} \\\\ \\mathrm{y_2} - \\mathrm{y_1} & \\mathrm{y_3} - \\mathrm{y_1} & \\mathrm{y_4} - \\mathrm{y_1} \\\\ \\mathrm{z_2} - \\mathrm{z_1} & \\mathrm{z_3} - \\mathrm{z_1} & \\mathrm{z_4} - \\mathrm{z_1}\\end{bmatrix})";

/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），列 = 参数（声明顺序）。
///
/// 参数 = `x_1, x_2, x_3, x_4, y_1, y_2, y_3, y_4, z_1, z_2, z_3, z_4`；不依赖某参数的行在该列恒为 0。
/// 源：`formulas/tet_element.formula`
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn tet_element__jacobian(x_1: f64, x_2: f64, x_3: f64, x_4: f64, y_1: f64, y_2: f64, y_3: f64, y_4: f64, z_1: f64, z_2: f64, z_3: f64, z_4: f64) -> Vec<Vec<f64>> {
    vec![
        vec![vol__d_x_1(x_1, x_2, x_3, x_4, y_1, y_2, y_3, y_4, z_1, z_2, z_3, z_4), vol__d_x_2(x_1, x_2, x_3, x_4, y_1, y_2, y_3, y_4, z_1, z_2, z_3, z_4), vol__d_x_3(x_1, x_2, x_3, x_4, y_1, y_2, y_3, y_4, z_1, z_2, z_3, z_4), vol__d_x_4(x_1, x_2, x_3, x_4, y_1, y_2, y_3, y_4, z_1, z_2, z_3, z_4), vol__d_y_1(x_1, x_2, x_3, x_4, y_1, y_2, y_3, y_4, z_1, z_2, z_3, z_4), vol__d_y_2(x_1, x_2, x_3, x_4, y_1, y_2, y_3, y_4, z_1, z_2, z_3, z_4), vol__d_y_3(x_1, x_2, x_3, x_4, y_1, y_2, y_3, y_4, z_1, z_2, z_3, z_4), vol__d_y_4(x_1, x_2, x_3, x_4, y_1, y_2, y_3, y_4, z_1, z_2, z_3, z_4), vol__d_z_1(x_1, x_2, x_3, x_4, y_1, y_2, y_3, y_4, z_1, z_2, z_3, z_4), vol__d_z_2(x_1, x_2, x_3, x_4, y_1, y_2, y_3, y_4, z_1, z_2, z_3, z_4), vol__d_z_3(x_1, x_2, x_3, x_4, y_1, y_2, y_3, y_4, z_1, z_2, z_3, z_4), vol__d_z_4(x_1, x_2, x_3, x_4, y_1, y_2, y_3, y_4, z_1, z_2, z_3, z_4)]
    ]
}

/// 结果量纲：`kg·m^2·s^(-5/2)·A^-2`
///
/// ```text
/// \sigma \cdot f^{-0.5}
/// ```
///
/// 源：`formulas/warburg.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn warburg_re(sigma: f64, f: f64) -> f64 {
        sigma * (f).powf(-0.5)
    }

/// ∂warburg_re/∂sigma，量纲 `s^(1/2)`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn warburg_re__d_sigma(sigma: f64, f: f64) -> f64 {
        (f).powf(-0.5)
    }

/// ∂warburg_re/∂f，量纲 `kg·m^2·s^(-3/2)·A^-2`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn warburg_re__d_f(sigma: f64, f: f64) -> f64 {
        sigma * (-0.5 * (f).powf(-1.5))
    }

/// f32 变体：同一条公式的单精度版本（字面量带 `f32` 后缀）。
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn warburg_re__f32(sigma: f32, f: f32) -> f32 {
        sigma * (f).powf(-0.5f32)
    }

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn warburg_re__batch(sigma: &[f64], f: &[f64], out: &mut [f64]) {
    let __n = out.len().min(sigma.len()).min(f.len());
    for __row in 0..__n {
        out[__row] = warburg_re(sigma[__row], f[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn warburg_re__batch_n<const __HIMARI_LEN: usize>(sigma: &[f64; __HIMARI_LEN], f: &[f64; __HIMARI_LEN], out: &mut [f64; __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = warburg_re(sigma[__row], f[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const WARBURG_RE__LATEX: &str = "\\sigma \\cdot f^{-0.5}";

/// 系统雅可比：行 = **参与求导的**公式（本文件顺序，矩阵值/复数公式被跳过），列 = 参数（声明顺序）。
///
/// 参数 = `sigma, f`；不依赖某参数的行在该列恒为 0。
/// 源：`formulas/warburg.formula`
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn warburg__jacobian(sigma: f64, f: f64) -> Vec<Vec<f64>> {
    vec![
        vec![warburg_re__d_sigma(sigma, f), warburg_re__d_f(sigma, f)]
    ]
}

/// 结果量纲：`kg·m^2·s^-3·A^-2`
///
/// ```text
/// \frac{\sigma}{(j \cdot \omega)^{0.5}}
/// ```
///
/// 源：`formulas/warburg_complex.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn Z_c(sigma: f64, omega: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_div(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            let m = b.0.abs().max(b.1.abs());
            if m > 1e-150 && m < 1e150 {
                let d = b.0 * b.0 + b.1 * b.1;
                ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
            } else {
                let (br, bi) = (b.0 / m, b.1 / m);
                let s = m * (br * br + bi * bi);
                ((a.0 * br + a.1 * bi) / s, (a.1 * br - a.0 * bi) / s)
            }
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        #[inline(always)]
        fn __c_sqrt(a: (f64, f64)) -> (f64, f64) {
            if a.1 == 0.0 {
                return if a.0 >= 0.0 { (a.0.sqrt(), 0.0) } else { (0.0, (-a.0).sqrt()) };
            }
            if a.0 == 0.0 {
                let m = (a.1.abs() * 0.5).sqrt();
                return (m, m.copysign(a.1));
            }
            let r = a.0.hypot(a.1);
            let re = (((r + a.0) * 0.5).max(0.0)).sqrt();
            let im = (((r - a.0) * 0.5).max(0.0)).sqrt().copysign(a.1);
            (re, im)
        }
        __c_div((sigma, 0.0), __c_sqrt(__c_mul((0.0, 1.0), (omega, 0.0))))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn Z_c__batch(sigma: &[f64], omega: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(sigma.len()).min(omega.len());
    for __row in 0..__n {
        out[__row] = Z_c(sigma[__row], omega[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn Z_c__batch_n<const __HIMARI_LEN: usize>(sigma: &[f64; __HIMARI_LEN], omega: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = Z_c(sigma[__row], omega[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_C__LATEX: &str = "\\frac{\\sigma}{(j \\cdot \\omega)^{0.5}}";

/// 结果量纲：`kg·m^2·s^-3·A^-2`
///
/// ```text
/// \operatorname{abs}(\frac{\sigma}{(j \cdot \omega)^{0.5}})
/// ```
///
/// 源：`formulas/warburg_complex.formula`
#[inline]
#[allow(non_snake_case, unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn Z_mag(sigma: f64, omega: f64) -> (f64, f64) {
        #[inline(always)]
        fn __c_abs(a: (f64, f64)) -> (f64, f64) {
            (a.0.hypot(a.1), 0.0)
        }
        #[inline(always)]
        fn __c_div(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            let m = b.0.abs().max(b.1.abs());
            if m > 1e-150 && m < 1e150 {
                let d = b.0 * b.0 + b.1 * b.1;
                ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
            } else {
                let (br, bi) = (b.0 / m, b.1 / m);
                let s = m * (br * br + bi * bi);
                ((a.0 * br + a.1 * bi) / s, (a.1 * br - a.0 * bi) / s)
            }
        }
        #[inline(always)]
        fn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
            (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
        }
        #[inline(always)]
        fn __c_sqrt(a: (f64, f64)) -> (f64, f64) {
            if a.1 == 0.0 {
                return if a.0 >= 0.0 { (a.0.sqrt(), 0.0) } else { (0.0, (-a.0).sqrt()) };
            }
            if a.0 == 0.0 {
                let m = (a.1.abs() * 0.5).sqrt();
                return (m, m.copysign(a.1));
            }
            let r = a.0.hypot(a.1);
            let re = (((r + a.0) * 0.5).max(0.0)).sqrt();
            let im = (((r - a.0) * 0.5).max(0.0)).sqrt().copysign(a.1);
            (re, im)
        }
        __c_abs(__c_div((sigma, 0.0), __c_sqrt(__c_mul((0.0, 1.0), (omega, 0.0)))))
    }

// 复数公式不生成解析导数：非解析算子（abs/arg/conj/re/im）的链式法则无定义。

/// 批量入口：公共长度一次算定，供 LLVM 向量化。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn Z_mag__batch(sigma: &[f64], omega: &[f64], out: &mut [(f64, f64)]) {
    let __n = out.len().min(sigma.len()).min(omega.len());
    for __row in 0..__n {
        out[__row] = Z_mag(sigma[__row], omega[__row]);
    }
}
/// 常量长度批量入口：长度进类型参数，边界检查可全部消除。
#[allow(non_snake_case, unused_variables)]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn Z_mag__batch_n<const __HIMARI_LEN: usize>(sigma: &[f64; __HIMARI_LEN], omega: &[f64; __HIMARI_LEN], out: &mut [(f64, f64); __HIMARI_LEN]) {
    for __row in 0..__HIMARI_LEN {
        out[__row] = Z_mag(sigma[__row], omega[__row]);
    }
}

/// 源公式（回写），供文档/UI 复用同一份真源。
pub const Z_MAG__LATEX: &str = "\\operatorname{abs}(\\frac{\\sigma}{(j \\cdot \\omega)^{0.5}})";

