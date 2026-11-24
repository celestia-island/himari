fn main() {
    println!("cargo:rerun-if-changed=formulas");
    if let Err(e) = himari_buildhelper::generate_dir("formulas", "src/generated.rs") {
        panic!("{e}");
    }
}
