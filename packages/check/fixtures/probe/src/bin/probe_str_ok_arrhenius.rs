// EXPECT: OK
// NOTE: Arrhenius：多字母名字只能写 E_a（LaTeX 里 Ea 会被读成 E·a —— 这正是单字母体系的约束）
// 自动生成，勿手改 —— 见 make_probes.py
    #![allow(warnings)]
    use himari_macros::*;

    fn main() {
        let A = 1.5f64;
let E_a = 1.5f64;
let R = 1.5f64;
let T = 1.5f64;
let _v = formula_latex!{ r#"decl { A : f [Hz]; E_a : e [J/mol]; R : g [J/mol/K]; T : t [K]; } k = A \exp( -\frac{E_a}{R T} )"# };
        println!("{_v:?}");
    }
