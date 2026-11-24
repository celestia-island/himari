// EXPECT: TOKEN_FAIL
// NOTE: ∂(U+2202, Sm) 同上
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_matlab!{ ∂u };
    println!("{_v:?}");
}
