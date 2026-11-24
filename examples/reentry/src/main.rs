//! A minimal reentry example: a **cross-file formula library** + generated code + physical
//! invariant self-checks.
//!
//! Three things hold at once here:
//!   1. the formulas live in `formulas/*.formula` and humans can read them directly (LaTeX/MATLAB
//!      writing habits);
//!   2. `use "lib/atmosphere.formula"` reuses the formula library across files (three lines in
//!      build.rs close over the whole lane);
//!   3. the physical invariant (at terminal velocity drag = weight) is verified by the generated
//!      code itself.
//!
//! Note that **federation is inlining**: `rho` comes from the library file, and once expanded into
//! the caller `rho_0 / h / H` appear in the parameter list — a library is not a "function call" but
//! a "formula substitution". That is both what makes it predictable (no hidden state) and what it
//! costs (the parameter list gets longer).
//!
//! How to run: `cargo run -p himari-example-reentry`

include!("generated.rs");

const MASS: f64 = 1200.0; // kg
const GRAVITY: f64 = 9.80665;
const AREA: f64 = 4.5; // m²
const CD: f64 = 1.2; // blunt body
const RHO0: f64 = 1.225; // sea level, kg/m³
const H_SCALE: f64 = 8500.0; // isothermal atmosphere scale height, m

fn main() {
    // ---- 1) The library formula: atmospheric density against altitude ----
    println!("{:>10}  {:>12}", "h (m)", "rho (kg/m³)");
    for h in [0.0, 2_000.0, 5_000.0, 10_000.0, 20_000.0] {
        println!("{h:>10.0}  {:>12.4}", rho(RHO0, h, H_SCALE));
    }

    // ---- 2) Physical invariant: at v_terminal the drag must equal the weight ----
    // Argument order = declaration order (the library's rho_0/h/H come first because they are
    // defined earlier)
    let vt = v_terminal(RHO0, 0.0, H_SCALE, MASS, GRAVITY, CD, AREA);
    let drag_at_vt = drag(RHO0, 0.0, H_SCALE, CD, AREA, vt);
    let weight = MASS * GRAVITY;
    let rel = (drag_at_vt - weight).abs() / weight;
    println!(
        "\n海平面终端速度 v_t = {vt:.3} m/s（阻力 {drag_at_vt:.1} N vs 重力 {weight:.1} N，相对差 {rel:.2e}）"
    );
    assert!(rel < 1e-12, "终端速度的定义就是阻力 = 重力");

    // The terminal velocity is higher up high (thinner air) — why a reentry gets harder to slow down
    // the lower it goes
    let vt20k = v_terminal(RHO0, 20_000.0, H_SCALE, MASS, GRAVITY, CD, AREA);
    println!(
        "20 km 处终端速度 = {vt20k:.1} m/s（是海平面的 {:.1}×）",
        vt20k / vt
    );
    assert!(vt20k > vt);

    // ---- 3) One table: velocity → dynamic pressure → drag ----
    println!(
        "\n{:>8}  {:>12}  {:>12}",
        "v (m/s)", "q_dyn (Pa)", "drag (N)"
    );
    for speed in [50.0, 100.0, 200.0, 400.0, 800.0, 1600.0] {
        println!(
            "{speed:>8.0}  {:>12.1}  {:>12.1}",
            q_dyn(RHO0, 0.0, H_SCALE, speed),
            drag(RHO0, 0.0, H_SCALE, CD, AREA, speed)
        );
    }

    // ---- 4) The analytic derivative: ∂drag/∂v = ρ·Cd·A·v (compared against a difference quotient)
    // ----
    let v = 321.0;
    let analytic = drag__d_v(RHO0, 0.0, H_SCALE, CD, AREA, v);
    let h = 1e-6;
    let numeric = (drag(RHO0, 0.0, H_SCALE, CD, AREA, v + h)
        - drag(RHO0, 0.0, H_SCALE, CD, AREA, v - h))
        / (2.0 * h);
    println!(
        "\n∂drag/∂v：解析 {analytic:.6} ／ 差商 {numeric:.6}（相对差 {:.2e}）",
        (analytic - numeric).abs() / numeric
    );

    // ---- 5) Symbolic linear solving: static deflection = F/k (the result is still a formula, so the
    // derivative holds automatically) ----
    let (k, f) = (2.0e6, 1.5e4);
    println!(
        "静位移 x = {:.6} m（∂x/∂F = {:.3e}，应为 1/k = {:.3e}）",
        x_static(k, f),
        x_static__d_F(k, f),
        1.0 / k
    );

    // ---- 6) The f32 variant and the system Jacobian ----
    let vt32 = v_terminal__f32(
        RHO0 as f32,
        0.0,
        H_SCALE as f32,
        MASS as f32,
        GRAVITY as f32,
        CD as f32,
        AREA as f32,
    );
    println!(
        "f32 终端速度 = {vt32:.3} m/s（与 f64 相对差 {:.2e}）",
        (vt32 as f64 - vt).abs() / vt
    );
    let jac = entry__jacobian(RHO0, 0.0, H_SCALE, MASS, GRAVITY, CD, AREA, vt);
    println!(
        "entry 系统雅可比：{} 行 × {} 列（行 = 四条公式（含库内联 rho），列 = 八个参数）",
        jac.len(),
        jac.first().map(|r| r.len()).unwrap_or(0)
    );
}
#[cfg(test)]
mod tests {
    // The example itself is a regression-testable piece of evidence: `cargo test` runs these
    // invariants too.

    #[test]
    fn terminal_velocity_balances_drag_and_weight() {
        let (mass, g, rho0, h, hs, cd, area) = (1200.0, 9.80665, 1.225, 0.0, 8500.0, 1.2, 4.5);
        let vt = super::v_terminal(rho0, h, hs, mass, g, cd, area);
        let drag = super::drag(rho0, h, hs, cd, area, vt);
        let weight = mass * g;
        assert!((drag - weight).abs() / weight < 1e-12);
    }

    #[test]
    fn library_density_decays_with_altitude() {
        // The library formula has been inlined into this file → usable directly
        let sea = super::rho(1.225, 0.0, 8500.0);
        let high = super::rho(1.225, 10_000.0, 8500.0);
        assert!((sea - 1.225).abs() < 1e-12);
        assert!(high < sea * 0.45, "10 km 处密度应降到 ~0.38：{high}");
    }

    #[test]
    fn solved_static_deflection_has_unit_stiffness_slope() {
        let (k, f) = (2.0e6, 1.5e4);
        assert!((super::x_static(k, f) - f / k).abs() < 1e-15);
        assert!((super::x_static__d_F(k, f) - 1.0 / k).abs() < 1e-18);
    }
}
