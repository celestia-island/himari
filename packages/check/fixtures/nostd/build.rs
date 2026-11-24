// The compile gate of the no_std lane: formulas → libm-flavored generated code → compiled together
// with the `#![no_std]` library.

fn main() {
    println!("cargo:rerun-if-changed=formulas");
    if let Err(e) = himari_buildhelper::generate_dir_libm("formulas", "src/generated.rs") {
        panic!("{e}");
    }
}
