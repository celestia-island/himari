// EXPECT: FORMULA_FAIL
// NOTE: α 是合法 Rust 标识符（XID_Start）；本子集刻意只收 ASCII 标识符，故在语义层拒绝
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_matlab!{ decl { α : l [m]; } α + 1.0 };
    println!("{_v:?}");
}
