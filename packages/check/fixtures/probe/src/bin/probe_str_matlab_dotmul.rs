// EXPECT: FORMULA_FAIL
// NOTE: `.*` 是逐元素语义，静默当 `*` 会改变含义 —— 必须拒绝
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_matlab!{ r#"decl { x : a; y : a; } x .* y"# };
    println!("{_v:?}");
}
