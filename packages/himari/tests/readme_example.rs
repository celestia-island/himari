//! The README's anyhow example must compile and hold as written (the README block is prose;
//! this test is its executable twin).
#[allow(unused_imports)] // the README shows this import; keep the twin identical
use anyhow::Context as _;

fn run() -> anyhow::Result<f64> {
    let dim = himari::parse_unit("N/m")?;
    let e = himari::Error::NoRealRoots { discriminant: -4.0 };
    let err = anyhow::Error::from(e).context("solving the loop equation");
    let root = err
        .downcast_ref::<himari::Error>()
        .expect("root cause kept");
    assert_eq!(root.code(), "no-real-roots");
    assert_eq!(dim, himari::parse_unit("kg/s^2")?);
    Ok(1.0)
}

#[test]
fn the_readme_anyhow_example_holds() {
    assert_eq!(run().unwrap(), 1.0);
}
