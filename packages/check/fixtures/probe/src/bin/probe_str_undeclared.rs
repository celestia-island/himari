// EXPECT: FORMULA_FAIL
// NOTE: 未声明符号一律拒绝，不做字形推断
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_latex!{ r#"decl { a : l [m]; } a + b"# };
    println!("{_v:?}");
}
