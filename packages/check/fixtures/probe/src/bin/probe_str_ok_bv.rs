// EXPECT: OK
// NOTE: Butler–Volmer：LaTeX 表面 + 量纲全通 + 并置乘法 + 嵌套分式
// 自动生成，勿手改 —— 见 make_probes.py
    #![allow(warnings)]
    use himari_macros::*;

    fn main() {
        let i_0 = 1.5f64;
let eta = 1.5f64;
let alpha = 1.5f64;
let n = 1.5f64;
let F = 1.5f64;
let R = 1.5f64;
let T = 1.5f64;
let _v = formula_latex!{ r#"decl { i_0 : j [A/m^2]; eta : v [V]; alpha : d; n : d; F : f [C/mol]; R : g [J/mol/K]; T : t [K]; } i = i_0 ( \exp( \frac{\alpha n F \eta}{R T} ) - \exp( -\frac{(1 - \alpha) n F \eta}{R T} ) )"# };
        println!("{_v:?}");
    }
