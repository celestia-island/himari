// EXPECT: OK
// NOTE: 半整数量纲后 √V = V^(1/2) 合法（原 FORMULA_FAIL 期望随分数指数特性落地而更新）
// 自动生成，勿手改 —— 见 make_probes.py
    #![allow(warnings)]
    use himari_macros::*;

    fn main() {
        let v = 1.5f64;
let _v = formula_latex!{ r#"decl { v : v [V]; } \sqrt{v}"# };
        println!("{_v:?}");
    }
