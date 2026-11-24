// EXPECT: OK
// NOTE: `%` 是合法 Rust token，所以 MATLAB 注释在宏层可存活
// 自动生成，勿手改 —— 见 make_probes.py
    #![allow(warnings)]
    use himari_macros::*;

    fn main() {
        let a = 1.5f64;
let _v = formula_matlab!{ decl { a : l [m]; } a % 注释 };
        println!("{_v:?}");
    }
