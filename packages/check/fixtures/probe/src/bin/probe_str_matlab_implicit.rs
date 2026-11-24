// EXPECT: FORMULA_FAIL
// NOTE: MATLAB 语义里 `x y` 不是乘法；并置乘法只属于 LaTeX 的单字母体系
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_matlab!{ r#"decl { x : a; y : a; } x y"# };
    println!("{_v:?}");
}
