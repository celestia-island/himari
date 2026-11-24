// EXPECT: FORMULA_FAIL
// NOTE: MathML **能过词法**（< > / 都是 token），但作为源语言被我们的子集拒绝
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
use himari_macros::*;

fn main() {
    let _v = formula_matlab!{ <math><mi>x</mi></math> };
    println!("{_v:?}");
}
