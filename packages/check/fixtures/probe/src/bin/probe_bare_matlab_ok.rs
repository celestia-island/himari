// EXPECT: OK
// NOTE: MATLAB 表达式子集可以整段当 token 流喂进宏
// 自动生成，勿手改 —— 见 make_probes.py
    #![allow(warnings)]
    use himari_macros::*;

    fn main() {
        let a = 1.5f64;
let b = 1.5f64;
let _v = formula_matlab!{ decl { a : l [m]; b : l [m]; } a / b };
        println!("{_v:?}");
    }
