// EXPECT: FORMULA_FAIL
// NOTE: 矩阵元素量纲不一致必须被拒（m vs s）
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_matlab!{ r#"decl { a : l [m]; t : t [s]; } [a; t]"# };
    println!("{_v:?}");
}
