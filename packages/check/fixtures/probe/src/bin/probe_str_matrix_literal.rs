// EXPECT: OK
// NOTE: 具体矩阵已接入量纲检查（元素同量纲即可）——原 FORMULA_FAIL 期望随矩阵支持落地而更新
// 自动生成，勿手改 —— 见 make_probes.py
    #![allow(warnings)]
    use himari_macros::*;

    fn main() {
        let a = 1.5f64;
let b = 1.5f64;
let _v = formula_matlab!{ r#"decl { a : a; b : a; } [1, a; b, 2]"# };
        println!("{_v:?}");
    }
