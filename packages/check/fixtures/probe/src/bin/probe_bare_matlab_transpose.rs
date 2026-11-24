// EXPECT: TOKEN_FAIL
// NOTE: `'` 在 Rust 里是生命周期/字符字面量起点，转置运算符无法进宏
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_matlab!{ decl { a : l [m]; } a' };
    println!("{_v:?}");
}
