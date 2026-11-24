// EXPECT: FORMULA_FAIL
// NOTE: 模式矩阵在**内联车道**被拒并引导走文件车道（分诊规则）
// 自动生成，勿手改 —— 见 make_probes.py
        #![allow(warnings)]
        use himari_macros::*;

        fn main() {
            let _v = formula_matlab!{ decl { b : d; c : d; a : d; }
[2*b  -b   ... 0
 -b  2*b  -b  ...
 ...  -b  2*b  -b
 0  ...  -b  2*b] };
            println!("{_v:?}");
        }
