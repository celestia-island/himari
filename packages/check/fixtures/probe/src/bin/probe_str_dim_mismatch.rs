// EXPECT: FORMULA_FAIL
// NOTE: 量纲不一致：这是本方案最赚钱的一类拦截
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_latex!{ r#"decl { a : l [m]; t : t [s]; } a + t"# };
    println!("{_v:?}");
}
