// EXPECT: OK
// NOTE: token 车道**换行分隔**联邦公式（span 行号在 stable 可用，行号已对齐源文件）
// 自动生成，勿手改 —— 见 make_probes.py
        #![allow(warnings)]
        use himari_macros::*;

        fn main() {
            let F = 1.5f64;
    let m = 1.5f64;
    let r = 1.5f64;
    let _v = formula_matlab!{ decl { F : f [N]; m : m [kg]; r : rad [m]; }
accel = F / m
mech = ( accel * r )^(1/2) };
            println!("{_v:?}");
        }
