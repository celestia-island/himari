// EXPECT: TOKEN_FAIL
// NOTE: \left/\right 同因
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_latex!{ \left( x \right) };
    println!("{_v:?}");
}
