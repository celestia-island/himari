// EXPECT: OK
// NOTE: 字符串车道让 LaTeX 可用，代价是内层 span 全丢
// 自动生成，勿手改 —— 见 make_probes.py
    #![allow(warnings)]
    use himari_macros::*;

    fn main() {
        let a = 1.5f64;
let b = 1.5f64;
let _v = formula_latex!{ r#"decl { a : l [m]; b : l [m]; } \frac{a}{b}"# };
        println!("{_v:?}");
    }
