// EXPECT: MEASURE
// GLYPH: θ
// CLASS: letter
// NOTE: theta U+03B8
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
fn main() {
    // 只测词法：这个字形能不能作为 Rust 源码活下来
    println!("{}", stringify!(θ));
}
