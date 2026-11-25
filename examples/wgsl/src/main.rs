//! Emit a WGSL compute kernel from a formula and validate it with a real shader compiler.
//!
//! The WGSL lane produces an `f32` scalar function for GPU use. This example emits two kernels
//! (one physics-flavoured, one with a transcendental), runs the emitter's own structural gate and
//! a real `naga` parse, and prints the sources.
//!
//! Run: `cargo run -p himari-example-wgsl`
//!
//! For executing the kernel and comparing the numbers against the CPU lane, see the
//! `himari-gpu-verify` crate at the workspace root (`just gpu-verify`).

use himari::{add, div, mul, neg, num, pow, var, Expr};

fn main() {
    // Kinetic energy: ½·m·v².
    let kinetic = mul(mul(num(0.5), var("m")), pow(var("v"), num(2.0)));
    let kernel = himari::wgsl::emit_wgsl_with_params(
        &kinetic,
        "kinetic",
        &["m".to_string(), "v".to_string()],
    )
    .expect("this shape must emit");
    println!("── kinetic ──\n{kernel}");
    himari::wgsl::validate_wgsl(&kernel).expect("the emitted kernel must pass the structural gate");

    // A transcendental: t·exp(−t/τ) + sin(ω·t) — exercises `exp` and `sin` in WGSL.
    let tau = var("tau");
    let omega = var("omega");
    let t = var("t");
    let wave = add(
        mul(
            t.clone(),
            Expr::Call("exp".into(), vec![neg(div(t.clone(), tau))]),
        ),
        Expr::Call("sin".into(), vec![mul(omega, t)]),
    );
    let kernel = himari::wgsl::emit_wgsl_with_params(
        &wave,
        "wave",
        &["t".to_string(), "omega".to_string(), "tau".to_string()],
    )
    .expect("this shape must emit");
    println!("── wave ──\n{kernel}");
    himari::wgsl::validate_wgsl(&kernel).expect("the emitted kernel must pass the structural gate");

    // The emitter's structural gate runs before a kernel ever leaves the emitter; a float
    // literal without the `f` suffix is refused (the return-line rule is per-line, so the
    // kernel must be written in statement form).
    let refused = himari::wgsl::validate_wgsl("fn f() -> f32 {\n    return 2.5;\n}");
    println!(
        "structure gate on `2.5`  : {:?}",
        refused.err().map(|e| e.code())
    );
    himari::wgsl::validate_wgsl("fn f() -> f32 {\n    return 2.5f;\n}")
        .expect("the suffixed literal must pass");
    println!("structure gate on `2.5f` : ok");
}
