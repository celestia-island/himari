//! `no_std` compile gate.
//!
//! This crate must build with `core` + `alloc` + `libm` only. `[lib] test = false`
//! because the test harness requires std, which this fixture deliberately omits.
//! Passing `cargo build` proves the generated code and the parser chain do not
//! reach for std behind our back.

#![no_std]

extern crate alloc;
extern crate libm;

include!("generated.rs");

/// Exercise the parser and dimension checker in a no_std environment.
///
/// The function is never called at runtime; it only has to compile, which is what
/// proves the front-end stack is usable without std.
pub fn parse_and_check(src: &str) -> Option<bool> {
    let set = himari_matlab::parse(src).ok()?;
    let dims = himari::dims_of_set(&set).ok()?;
    let shapes = himari::runtime::shapes_of_set(&set).ok()?;
    for item in &set.items {
        let folded = himari::simplify(&item.expr);
        himari::runtime::dim_check_runtime(&folded, &dims, &shapes).ok()?;
    }
    Some(true)
}

/// Exercise canonical rendering without std.
pub fn canonical(src: &str) -> Option<alloc::string::String> {
    let set = himari_matlab::parse(src).ok()?;
    Some(himari::format_set(&set))
}
