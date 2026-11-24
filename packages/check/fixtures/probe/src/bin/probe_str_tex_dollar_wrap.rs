// EXPECT: FORMULA_FAIL
// NOTE: 行内数学定界符不属于公式本体，收进源里就是隐式约定
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_latex!{ r#"decl { x : a; } $x$"# };
    println!("{_v:?}");
}
