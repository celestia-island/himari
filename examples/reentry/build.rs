// The whole build.rs lane = three lines: the library function `generate_dir` closes over parsing /
// federation / dimensions / shapes / derivatives / f32 / batch entry points / the Jacobian / source
// LaTeX.

fn main() {
    println!("cargo:rerun-if-changed=formulas");
    if let Err(e) = himari_buildhelper::generate_dir("formulas", "src/generated.rs") {
        panic!("{e}");
    }
}
