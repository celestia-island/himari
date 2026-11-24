// EXPECT: FORMULA_FAIL
// NOTE: `:` 与 `end` 都是合法 token（语法层能过），但不在本子集语义内
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_matlab!{ decl { a : l [m]; } a(1:end) };
    println!("{_v:?}");
}
