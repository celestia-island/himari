//! The **round trip** of a cross-file library: canonicalization must not lose `use`, and the
//! dimensions still hold after the loader merges.

use std::io::Write as _;

fn tmp(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("himari-lib-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    let p = dir.join(name);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let mut f = std::fs::File::create(&p).unwrap();
    f.write_all(body.as_bytes()).unwrap();
    p
}

#[test]
fn canonical_round_trip_preserves_use_directive() {
    let dir = tmp("roundtrip");
    write(
        &dir,
        "lib/gas.formula",
        "decl {\n    R : gas_constant [J/mol/K];\n    T : temperature [K];\n}\nrt = R * T\n",
    );
    let main = write(
        &dir,
        "main.formula",
        "use \"lib/gas.formula\"\ndecl {\n    n : amount [mol];\n}\np = n * rt\n",
    );
    let set = himari_loader::load(&main).unwrap();

    // After merging: the library's decls come first and the dimension is computable
    let dims = himari::dims_of_set(&set).unwrap();
    assert_eq!(dims["p"], himari::parse_unit("J").unwrap());

    // Note: the result of `load` is an **already inlined** set whose uses are empty (the
    // dependencies have been resolved). Formatting targets **a single file**, so the file's own
    // parse result must be used.
    assert!(
        set.uses.is_empty(),
        "merged 集合的依赖已经内联，不该再带 use"
    );
    let src = std::fs::read_to_string(&main).unwrap();
    let own = himari_matlab::parse(&src).unwrap();
    assert_eq!(own.uses, vec!["lib/gas.formula".to_string()]);

    // After canonicalization the **relative order** of `use` and the declarations is preserved
    // (use comes before decl)
    let text = himari::format_set(&own);
    let u = text.find("use \"lib/gas.formula\"").expect("use 丢了");
    let d = text.find("decl {").expect("decl 丢了");
    assert!(u < d, "canonical 形态里 use 必须在 decl 之前：\n{text}");

    // Re-parse the canonical text: both the use and the items are still there
    let back = himari_matlab::parse(&text).unwrap();
    assert_eq!(back.uses, vec!["lib/gas.formula".to_string()]);
    assert_eq!(back.items.len(), own.items.len());
}

#[test]
fn library_dims_flow_into_the_caller() {
    let dir = tmp("flow");
    // The library provides a "velocity" that the caller uses to compute kinetic energy
    write(
        &dir,
        "lib/kinematics.formula",
        "decl {\n    a : acceleration [m/s^2];\n    t : time [s];\n}\nv = a * t\n",
    );
    let main = write(
        &dir,
        "main.formula",
        "use \"lib/kinematics.formula\"\ndecl {\n    m : mass [kg];\n}\nke = 0.5 * m * v ^ 2\n",
    );
    let set = himari_loader::load(&main).unwrap();
    let dims = himari::dims_of_set(&set).unwrap();
    assert_eq!(
        dims["ke"],
        himari::parse_unit("J").unwrap(),
        "动能必须是焦耳"
    );
}
