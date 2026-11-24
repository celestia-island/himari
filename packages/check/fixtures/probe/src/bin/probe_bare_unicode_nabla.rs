// EXPECT: TOKEN_FAIL
// NOTE: ∇(U+2207, Sm) 不是 XID_Start，Rust 不认
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_matlab!{ ∇ u };
    println!("{_v:?}");
}
