//! Kill gate item 3: **generated code vs hand-written code** (the scalar path is required to be on
//! par).
//!
//! How to run: `cargo run -p three --bin bench --release`

use std::hint::black_box;
use std::time::Instant;

const F: f64 = 96485.33212;
const R: f64 = 8.314462618;

/// The human hand-written version: a naive implementation corresponding term by term to the
/// `.formula` source.
#[inline]
fn hand(i_0: f64, eta: f64, alpha: f64, n: f64, f: f64, r: f64, t: f64) -> f64 {
    let x = alpha * n * f * eta / (r * t);
    let y = (1.0 - alpha) * n * f * eta / (r * t);
    i_0 * (x.exp() - (-y).exp())
}

fn main() {
    let iters = 3_000_000usize;
    let mut acc = 0.0f64;

    // Warm-up (the first call triggers branch-prediction/cache cold starts)
    for i in 0..10_000 {
        acc += three::bv(1.0, i as f64 * 1e-9, 0.5, 1.0, F, R, 298.15);
    }

    let t0 = Instant::now();
    for i in 0..iters {
        let eta = black_box(i as f64 * 1e-9);
        acc += three::bv(
            black_box(1.0),
            eta,
            black_box(0.5),
            black_box(1.0),
            F,
            R,
            black_box(298.15),
        );
    }
    let gen_dt = t0.elapsed();

    let t1 = Instant::now();
    for i in 0..iters {
        let eta = black_box(i as f64 * 1e-9);
        acc += hand(
            black_box(1.0),
            eta,
            black_box(0.5),
            black_box(1.0),
            F,
            R,
            black_box(298.15),
        );
    }
    let hand_dt = t1.elapsed();

    let g = gen_dt.as_secs_f64() * 1e9 / iters as f64;
    let h = hand_dt.as_secs_f64() * 1e9 / iters as f64;
    println!(
        "生成 {g:.3} ns/call ｜ 手写 {h:.3} ns/call ｜ 比值 {:.3} ｜ acc={acc:.6e}",
        g / h
    );
    bench_batch();
    bench_complex();
    bench_jacobian();
    bench_f32();
}

const N_BENCH: usize = 100_000;

fn bench_batch() {
    let n = 100_000usize;
    // All seven inputs **really vary** and are built outside the timed region — batch and the
    // scalar path consume the same data; any constant vector would let the scalar path be
    // constant-folded (this trap was actually hit once).
    let eta: Vec<f64> = (0..n).map(|i| i as f64 * 1e-4).collect();
    let vi0: Vec<f64> = (0..n).map(|i| 1.0 + i as f64 * 1e-12).collect();
    let valpha: Vec<f64> = (0..n).map(|i| 0.5 + i as f64 * 1e-12).collect();
    let vn: Vec<f64> = (0..n).map(|i| 1.0 + i as f64 * 1e-12).collect();
    let vf: Vec<f64> = (0..n).map(|i| F * (1.0 + i as f64 * 1e-15)).collect();
    let vr: Vec<f64> = (0..n).map(|i| R * (1.0 + i as f64 * 1e-15)).collect();
    let vt: Vec<f64> = (0..n).map(|i| 298.15 + i as f64 * 1e-9).collect();
    let mut out = vec![0.0f64; n];

    // batch (the generated lane, zip-iterator form)
    let t0 = std::time::Instant::now();
    three::bv__batch(&vi0, &eta, &valpha, &vn, &vf, &vr, &vt, &mut out);
    let batch_dt = t0.elapsed();
    let _s0 = black_box(out.iter().sum::<f64>());

    // Scalar loop (same data, same layout)
    let t1 = std::time::Instant::now();
    for k in 0..n {
        out[k] = three::bv(vi0[k], eta[k], valpha[k], vn[k], vf[k], vr[k], vt[k]);
    }
    let scalar_dt = t1.elapsed();

    // The const-generic variant (the length goes into the type → no bounds checks)
    let vi0a: [f64; N_BENCH] = std::array::from_fn(|i| 1.0 + i as f64 * 1e-12);
    let etaa: [f64; N_BENCH] = std::array::from_fn(|i| i as f64 * 1e-4);
    let alphaa: [f64; N_BENCH] = std::array::from_fn(|i| 0.5 + i as f64 * 1e-12);
    let na: [f64; N_BENCH] = std::array::from_fn(|i| 1.0 + i as f64 * 1e-12);
    let fa: [f64; N_BENCH] = std::array::from_fn(|i| F * (1.0 + i as f64 * 1e-15));
    let ra: [f64; N_BENCH] = std::array::from_fn(|i| R * (1.0 + i as f64 * 1e-15));
    let ta: [f64; N_BENCH] = std::array::from_fn(|i| 298.15 + i as f64 * 1e-9);
    let mut outc = [0.0f64; N_BENCH];
    let t2 = std::time::Instant::now();
    three::bv__batch_n(&vi0a, &etaa, &alphaa, &na, &fa, &ra, &ta, &mut outc);
    let cg_dt = t2.elapsed();
    let _s2 = black_box(outc.iter().sum::<f64>());
    let _ = _s2;
    let _s1 = black_box(out.iter().sum::<f64>());
    let _ = (_s0, _s1);

    let b = batch_dt.as_secs_f64() * 1e9 / n as f64;
    let sc = scalar_dt.as_secs_f64() * 1e9 / n as f64;
    let cg = cg_dt.as_secs_f64() * 1e9 / N_BENCH as f64;
    println!(
        "batch(切片) {b:.3} ns/点 ｜ batch(const N) {cg:.3} ns/点 ｜ 标量 {sc:.3} ns/点 ｜ 切片/标量 {:.3} ｜ 定长/标量 {:.3}",
        sc / b,
        sc / cg
    );
}

/// The complex lane (EIS frequency sweep): generated code vs a hand-written complex Warburg.
///
/// The timed region contains only the computation itself: the input vectors are built outside it
/// (the same discipline as above).
fn bench_complex() {
    let n = 200_000usize;
    let sigma: Vec<f64> = (0..n).map(|i| 0.5 + i as f64 * 1e-9).collect();
    let omega: Vec<f64> = (0..n).map(|i| 1.0 + i as f64 * 1e-3).collect();
    let mut out = vec![(0.0f64, 0.0f64); n];

    let t0 = Instant::now();
    three::Z_c__batch(&sigma, &omega, &mut out);
    let gen_dt = t0.elapsed();
    let s0 = black_box(out.iter().map(|(a, b)| a + b).sum::<f64>());

    // Hand-written: σ / √(jω) — the same mathematics as the formula, written directly in cos/sin
    // form
    let t1 = Instant::now();
    for k in 0..n {
        let (sg, w) = (sigma[k], omega[k]);
        let r = w.sqrt();
        // 1/√(jω) = (1/√ω)·(cos(-π/4) + j·sin(-π/4))
        // Watch the sign: sin(-π/4) = **-**√2/2 (a "reference implementation" that misses this step
        // gives a wrong value)
        let m = sg / r * std::f64::consts::FRAC_1_SQRT_2;
        out[k] = (m, -m);
    }
    let hand_dt = t1.elapsed();
    let s1 = black_box(out.iter().map(|(a, b)| a + b).sum::<f64>());

    // For diagnosis: **copy verbatim** the operator sequence of the generated code (including
    // the fast-path branch), to tell "the algorithm itself is expensive" apart from "the emitted
    // code has extra overhead"
    let t2 = Instant::now();
    for k in 0..n {
        let (sg, w) = (sigma[k], omega[k]);
        // __c_mul((0,1),(w,0)) → (0, w)
        let z = (0.0f64, w);
        // The __c_sqrt fast path: a.0 == 0 → one sqrt
        let m = (z.1.abs() * 0.5).sqrt();
        let sq = (m, m.copysign(z.1));
        // The naive __c_div form
        let b = sq;
        let d = b.0 * b.0 + b.1 * b.1;
        let a = (sg, 0.0f64);
        out[k] = ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d);
    }
    let same_dt = t2.elapsed();
    let s2 = black_box(out.iter().map(|(a, b)| a + b).sum::<f64>());

    // A direct scalar loop (without the batch wrapper): separates the overhead of the batch
    // wrapper from that of Z_c itself
    let t3 = Instant::now();
    let mut acc = 0.0f64;
    for k in 0..n {
        let z = three::Z_c(sigma[k], omega[k]);
        acc += z.0 + z.1;
    }
    let direct_dt = t3.elapsed();
    let s3 = black_box(acc);

    let g = gen_dt.as_secs_f64() * 1e9 / n as f64;
    let h = hand_dt.as_secs_f64() * 1e9 / n as f64;
    let sd = same_dt.as_secs_f64() * 1e9 / n as f64;
    let dd = direct_dt.as_secs_f64() * 1e9 / n as f64;
    println!(
        "复数 Z_c：batch {g:.3} ｜ 标量直调 {dd:.3} ｜ 手写 {h:.3} ｜ 同形手抄 {sd:.3} ns/点 ｜ batch/手写 {:.3} ｜ 直调/同形 {:.3} ｜ acc={:.3e}/{:.3e}/{:.3e}/{:.3e}",
        g / h,
        dd / sd,
        s0,
        s1,
        s2,
        s3
    );
}

/// The system Jacobian (the inner loop of an implicit solver): all partials in one call.
fn bench_jacobian() {
    let n = 100_000usize;
    let i0: Vec<f64> = (0..n).map(|i| 1.0 + i as f64 * 1e-12).collect();
    let eta: Vec<f64> = (0..n).map(|i| i as f64 * 1e-4).collect();
    let alpha: Vec<f64> = (0..n).map(|i| 0.5 + i as f64 * 1e-12).collect();
    let nn: Vec<f64> = (0..n).map(|i| 1.0 + i as f64 * 1e-12).collect();
    let fv: Vec<f64> = (0..n).map(|_| F).collect();
    let rv: Vec<f64> = (0..n).map(|_| R).collect();
    let tv: Vec<f64> = (0..n).map(|i| 298.15 + i as f64 * 1e-9).collect();

    // The per-partial derivative function (the reference path)
    let t0 = Instant::now();
    let mut acc = 0.0f64;
    for k in 0..n {
        acc += three::bv__d_eta(i0[k], eta[k], alpha[k], nn[k], fv[k], rv[k], tv[k]);
    }
    let one_dt = t0.elapsed();

    // The Jacobian (all 7 partials in one go)
    let t1 = Instant::now();
    let mut acc2 = 0.0f64;
    for k in 0..n {
        let j: Vec<Vec<f64>> = jac_row(i0[k], eta[k], alpha[k], nn[k], fv[k], rv[k], tv[k]);
        acc2 += j[0][1];
    }
    let jac_dt = t1.elapsed();
    let one = one_dt.as_secs_f64() * 1e9 / n as f64;
    let jac = jac_dt.as_secs_f64() * 1e9 / n as f64;
    println!(
        "雅可比：单条偏导 {one:.3} ns/点 ｜ 整行雅可比 {jac:.3} ns/点 ｜ 比值 {:.3} ｜ acc={acc:.3e}/{acc2:.3e}",
        jac / one
    );
    let _ = black_box((acc, acc2));
}

/// Single precision (f32) vs double precision (f64): the two lanes of the same formula.
///
/// The f32 gain comes from wider SIMD (8 lanes vs 4) and half the memory bandwidth — the default
/// choice for games / audio DSP.
fn bench_f32() {
    let n = 200_000usize;
    let eta64: Vec<f64> = (0..n).map(|i| i as f64 * 1e-5).collect();
    let eta32: Vec<f32> = eta64.iter().map(|x| *x as f32).collect();

    let t0 = Instant::now();
    let mut acc = 0.0f64;
    for eta in &eta64 {
        acc += three::bv(1.0, *eta, 0.5, 1.0, F, R, 298.15);
    }
    let f64_dt = t0.elapsed();
    let s0 = black_box(acc);

    let t1 = Instant::now();
    let mut acc32 = 0.0f32;
    for eta in &eta32 {
        acc32 += three::bv__f32(1.0, *eta, 0.5, 1.0, F as f32, R as f32, 298.15);
    }
    let f32_dt = t1.elapsed();
    let s1 = black_box(acc32);
    let _ = black_box((s0, s1));

    let a = f64_dt.as_secs_f64() * 1e9 / n as f64;
    let b = f32_dt.as_secs_f64() * 1e9 / n as f64;
    println!(
        "f32 vs f64：f64 {a:.3} ns/点 ｜ f32 {b:.3} ns/点 ｜ f64/f32 {:.3}",
        a / b
    );
}

/// Row-wise Jacobian access (avoiding any extra work beyond the Vec allocation inside the timed
/// region).
#[inline]
fn jac_row(i_0: f64, eta: f64, alpha: f64, n: f64, f: f64, r: f64, t: f64) -> Vec<Vec<f64>> {
    three::bv__jacobian(i_0, eta, alpha, n, f, r, t)
}
