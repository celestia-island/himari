// EXPECT: TOKEN_FAIL
// NOTE: LaTeX 反斜杠不是 Rust token —— 这是整个方案的硬边界
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_latex!{ \frac{a}{b} };
    println!("{_v:?}");
}
