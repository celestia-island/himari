//! `himari::Error` must be usable with `anyhow::Result` without a wrapper.
//!
//! This is a compile-and-behaviour gate: `anyhow` requires the source error to be
//! `std::error::Error + Send + Sync + 'static`, and every `Result` the crate returns should
//! be able to flow into an `anyhow::Context`-based application with a single `?`.

use anyhow::Context as _;
use himari::i18n::{Error, Locale};
use std::collections::HashMap;

/// A plain `anyhow::Result` function that only uses `?` on himari results.
fn pipeline() -> anyhow::Result<f64> {
    let dim = himari::parse_unit("N/m")?;
    let set = himari_matlab_free_parse("decl { k : stiffness [N/m]; F : force [N]; } F / k")?;
    let dims = himari::dims_of_set(&set)?;
    let expr = set.into_single().1;
    let result_dim = himari::dim_check(&expr, &dims)?;
    assert_eq!(result_dim, himari::parse_unit("m")?);
    let env = HashMap::new();
    Ok(himari::eval(&expr, &env).unwrap_or(dim.display().len() as f64))
}

/// Without the dialect crate as a dependency, build the same shape through the public IR.
fn himari_matlab_free_parse(_src: &str) -> anyhow::Result<himari::FormulaSet> {
    let set = himari::FormulaSet {
        decls: vec![
            himari::Decl {
                name: "k".into(),
                quantity: "stiffness".into(),
                unit: Some("N/m".into()),
            },
            himari::Decl {
                name: "F".into(),
                quantity: "force".into(),
                unit: Some("N".into()),
            },
        ],
        items: vec![himari::FormulaItem {
            name: Some("x".into()),
            expr: himari::div(himari::var("F"), himari::var("k")),
            line: 1,
        }],
        uses: vec![],
    };
    Ok(set)
}

#[test]
fn himari_errors_flow_into_anyhow_results() {
    let value = pipeline().expect("no error expected");
    assert!(value.is_finite());
}

#[test]
fn a_failing_himari_call_becomes_an_anyhow_error_with_our_code() {
    let err: anyhow::Error = himari::parse_unit("definitely-not-a-unit")
        .unwrap_err()
        .into();
    // The error keeps its stable code, so `anyhow`'s chain still identifies it.
    let down = err
        .downcast_ref::<Error>()
        .expect("downcast to himari::Error");
    assert_eq!(down.code(), "unknown-unit");
    assert_eq!(
        down.render(Locale::ZhHans),
        "未知单位 `definitely-not-a-unit`"
    );
}

#[test]
fn errors_are_send_sync_and_static_so_anyhow_accepts_them() {
    fn assert_bounds<T: std::error::Error + Send + Sync + 'static>() {}
    assert_bounds::<Error>();
}

/// The point of the catalogue: one enum, one stable code, nine renderings. A test that asserts
/// the code never breaks when a translation changes; a test that asserts a sentence does.
#[test]
fn every_locale_renders_the_same_error_with_the_same_code() {
    let err = Error::UndeclaredSymbol("v".into());
    assert_eq!(err.code(), "undeclared-symbol");
    let mut seen = std::collections::BTreeSet::new();
    for locale in Locale::ALL {
        let text = err.render(locale);
        assert!(!text.trim().is_empty(), "{locale:?} is empty");
        assert!(text.contains('v'), "{locale:?} dropped the payload: {text}");
        assert!(
            !text.contains("{0}"),
            "{locale:?} left a placeholder: {text}"
        );
        assert!(
            seen.insert(text),
            "{locale:?} is not translated (duplicate text)"
        );
    }
    assert_eq!(seen.len(), Locale::ALL.len(), "each locale must differ");
}

#[test]
fn a_program_can_rethrow_a_himari_error_with_context_and_keep_the_code() {
    fn inner() -> anyhow::Result<()> {
        Err(Error::DimensionMismatch {
            left: "kg".into(),
            right: "m".into(),
        })?
    }
    let err = inner().context("adding force to mass").unwrap_err();
    assert!(err.to_string().contains("adding force to mass"));
    let source = err
        .downcast_ref::<Error>()
        .expect("root cause is himari::Error");
    assert_eq!(source.code(), "dimension-mismatch");
}
