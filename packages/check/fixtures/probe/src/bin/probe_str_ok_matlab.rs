// EXPECT: OK
// NOTE: MATLAB 表面：多字母标识符、显式乘法、无并置
// 自动生成，勿手改 —— 见 make_probes.py
    #![allow(warnings)]
    use himari_macros::*;

    fn main() {
        let F = 1.5f64;
let m = 1.5f64;
let _v = formula_matlab!{ r#"decl { F : f [N]; m : m [kg]; a : acc [m/s^2]; r : rad [m]; } a = F / m"# };
        println!("{_v:?}");
    }
