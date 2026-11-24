// The body of the build.rs lane lives in himari-buildhelper::generate_dir (consumers should not
// hand-write 300 lines themselves).
//
// Of the three lanes this is the only one that achieves real file:line diagnostics, cargo-tracked
// rebuilds and committed-artifact reviewability at the same time.

fn main() {
    println!("cargo:rerun-if-changed=formulas");
    // A change to the artifact itself does not trigger a rebuild (otherwise writing it back would
    // create a cycle)
    if let Err(e) = himari_buildhelper::generate_dir("formulas", "src/generated.rs") {
        panic!("{e}");
    }
}
