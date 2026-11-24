// EXPECT: OK
// NOTE: 希腊命令 → ASCII 标识符：LaTeX 表面与 Rust 标识符之间的映射
// 自动生成，勿手改 —— 见 make_probes.py
    #![allow(warnings)]
    use himari_macros::*;

    fn main() {
        let alpha = 1.5f64;
let eta = 1.5f64;
let _v = formula_latex!{ r#"decl { alpha : d; eta : d; } \alpha \eta"# };
        println!("{_v:?}");
    }
