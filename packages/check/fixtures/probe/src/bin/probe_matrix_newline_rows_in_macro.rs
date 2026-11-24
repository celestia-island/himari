// EXPECT: FORMULA_FAIL
// NOTE: token 车道换行分行矩阵能被解析，但模式矩阵仍需文件车道（分诊规则）
// 自动生成，勿手改 —— 见 make_probes.py
        #![allow(warnings)]
        use himari_macros::*;

        fn main() {
            let _v = formula_matlab!{ decl { k : d; }
[2*k, -k, ..., 0
 -k, 2*k, -k, ...
 ..., -k, 2*k, -k
 0, ..., -k, 2*k] };
            println!("{_v:?}");
        }
