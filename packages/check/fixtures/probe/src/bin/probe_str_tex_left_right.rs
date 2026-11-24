// EXPECT: FORMULA_FAIL
// NOTE: \left/\right 只影响排版高度，没有数学语义 —— 拒绝
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_latex!{ r#"decl { x : a; } \left( x \right)"# };
    println!("{_v:?}");
}
