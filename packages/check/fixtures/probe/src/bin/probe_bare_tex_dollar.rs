// EXPECT: FORMULA_FAIL
// NOTE: 实测纠正：`$` 是合法 token（宏元语法字符），能过词法；被拒的是我们的子集
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_latex!{ $ x $ };
    println!("{_v:?}");
}
