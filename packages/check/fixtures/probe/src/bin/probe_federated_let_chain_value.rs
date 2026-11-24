// EXPECT: OK
// NOTE: 三条联邦（换行分隔）+ 尾条无名：宏产物是 let 链 + 终值
// 自动生成，勿手改 —— 见 make_probes.py
        #![allow(warnings)]
        use himari_macros::*;

        fn main() {
            let x = 1.5f64;
    let _v = formula_matlab!{ decl { x : d; }
a = x + 1
b = a * 2
b - a };
            println!("{_v:?}");
        }
