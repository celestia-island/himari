// EXPECT: FORMULA_FAIL
// NOTE: \sin^2 x 有歧义（(sin x)^2 还是 sin(x^2)），必须拒绝
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_latex!{ r#"decl { x : a; } \sin^2 x"# };
    println!("{_v:?}");
}
