// EXPECT: TOKEN_FAIL
// NOTE: 数学命令与上一条同因
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_latex!{ \alpha };
    println!("{_v:?}");
}
