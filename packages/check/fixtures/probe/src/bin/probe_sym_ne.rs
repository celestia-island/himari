// EXPECT: MEASURE
// GLYPH: ≠
// CLASS: op
// NOTE: ne U+2260
// 自动生成，勿手改 —— 见 make_probes.py
#![allow(warnings)]
fn main() {
    // 只测词法：这个字形能不能作为 Rust 源码活下来
    println!("{}", stringify!(≠));
}
