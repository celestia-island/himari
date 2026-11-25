#![cfg_attr(not(feature = "std"), no_std)]

//! himari core: dimensioned IR / symbolic differentiation / matrix and linear solving / CSE /
//! Rust emitter.
//!
//! LaTeX, matex and paper glyphs are each lowered by a front end into the same IR; this crate
//! does not guess dialects, it only does algebra, checking, evaluation and code generation.

extern crate alloc;
#[cfg(test)]
extern crate std;

#[cfg(test)]
use std::prelude::v1::*;

pub mod i18n;

pub use i18n::Error;
pub use i18n::ReservedReason;
mod math;
pub mod runtime;
pub mod wgsl;

/// Fractional part, available in both std and no_std builds.
pub fn math_fract(value: f64) -> f64 {
    math::fract(value)
}

use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::Write as _;
#[cfg(feature = "std")]
pub use std::collections::HashMap;

#[cfg(not(feature = "std"))]
pub type HashMap<K, V> = BTreeMap<K, V>;

// ---------------------------------------------------------------- errors

/// Unified front-end error. `line == 0` means "no position" (e.g. a purely semantic error).
///
/// The error is identified by a stable code ([`Error::code`]), not by a sentence: `msg` is the
/// rendering for the locale that was requested (English by default), and `code` is what tests
/// and callers match on. Messages that have not been catalogued yet carry
/// `code == Error::Message(..)` with their original text — an explicit staging marker, not a
/// silent fallback.
#[derive(Clone, Debug, PartialEq)]
pub struct FrontError {
    pub line: usize,
    pub col: usize,
    /// Rendering in [`i18n::Locale::En`] unless [`FrontError::in_locale`] produced a copy.
    pub msg: String,
    pub code: Box<i18n::Error>,
    /// Substituted arguments, kept so the message can be re-rendered in another language.
    args: Vec<String>,
}

impl FrontError {
    pub fn new(line: usize, col: usize, msg: impl Into<Error>) -> Self {
        // Same rule as `plain`: a catalogued error keeps its variant across the front-end, so
        // `code()` still identifies the failure at the CLI/LSP edges; only a raw string lands as
        // `Message`.
        let error: Error = msg.into();
        let rendered = error.render(i18n::Locale::En);
        Self {
            line,
            col,
            msg: rendered,
            code: Box::new(error),
            args: Vec::new(),
        }
    }

    pub fn plain(msg: impl Into<Error>) -> Self {
        // Keep the variant: a front-end that wraps a catalogued error must not downgrade it to
        // `Message`, or `code()` would stop identifying the failure once it crosses the parser.
        let error: Error = msg.into();
        let msg = error.render(i18n::Locale::En);
        Self {
            line: 0,
            col: 0,
            msg,
            code: Box::new(error),
            args: Vec::new(),
        }
    }

    /// A catalogued error: the message is rendered from the code and its arguments.
    ///
    /// The arguments are kept, so [`FrontError::in_locale`] can re-render the same error in
    /// any of the nine languages without the caller having to hold them again.
    pub fn coded(code: i18n::Error, args: &[&str], line: usize, col: usize) -> Self {
        Self {
            line,
            col,
            msg: i18n::render(&code, args, i18n::Locale::En),
            code: Box::new(code),
            args: args.iter().map(|a| a.to_string()).collect(),
        }
    }

    /// Renders this error in `language`. Uncatalogued (`Raw`) errors keep their text.
    pub fn render_in(&self, language: i18n::Locale) -> String {
        // Uncatalogued (`Message`) errors pass through verbatim inside `render`.
        let borrowed: Vec<&str> = self.args.iter().map(|a| a.as_str()).collect();
        i18n::render(&self.code, &borrowed, language)
    }

    /// Convenience: a copy whose `msg` is rendered in `language`.
    pub fn in_locale(&self, language: i18n::Locale) -> Self {
        let mut out = self.clone();
        out.msg = self.render_in(language);
        out
    }

    /// Convenience: a copy rendered in `HIMARI_LANG`/`LC_ALL`/`LANG`, or English under `no_std`.
    pub fn localized(&self) -> Self {
        #[cfg(feature = "std")]
        {
            self.in_locale(i18n::Locale::from_env())
        }
        #[cfg(not(feature = "std"))]
        {
            self.clone()
        }
    }

    /// Contextual rendering: source line + caret.
    ///
    /// Given only `line:col`, the user has to count by hand which character is in column 17;
    /// quoting that line and drawing a `^` under the position is an editor-grade experience,
    /// and it costs only a few lines of code.
    pub fn render_with_source(&self, origin: &str, src: &str) -> String {
        let head = self.render(origin);
        if self.line == 0 {
            return head;
        }
        let Some(text) = src.lines().nth(self.line - 1) else {
            return head;
        };
        let gutter = format!("{}", self.line);
        let pad = " ".repeat(gutter.len());
        // A tab counts as one character (column numbers are counted that way too); the caret's
        // indentation matches the display width
        let mut caret = String::new();
        for ch in text.chars().take(self.col.saturating_sub(1)) {
            caret.push(if ch == '\t' { '\t' } else { ' ' });
        }
        format!("{head}\n{pad} |\n{gutter} | {text}\n{pad} | {caret}^")
    }

    pub fn render(&self, origin: &str) -> String {
        if self.line == 0 {
            format!("{origin}: {}", self.msg)
        } else {
            format!("{origin}:{}:{}: {}", self.line, self.col, self.msg)
        }
    }
}

// ---------------------------------------------------------------- declarations

/// Formula symbol declaration. **The dimension must be given explicitly** — this is where this
/// scheme pins down LaTeX's semantic ambiguity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decl {
    pub name: String,
    pub quantity: String,
    pub unit: Option<String>,
}

// ---------------------------------------------------------------- dimensions

/// The 7 base quantities: kg, m, s, A, K, mol, cd.
///
/// **Stored internally in units of 1/2** (half-units): each number in `[i16; 7]` is twice the
/// real exponent (`s^-1/2` is stored as `-1`, `s^-1` as `-2`). This way fractional powers
/// (√, ^(1/2), ^(3/2)) and integer powers go through the same integer arithmetic, with no need
/// to introduce a rational type; divide by 2 again when displaying.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dim(pub [i16; 7]);

impl Dim {
    pub const ZERO: Dim = Dim([0; 7]);

    /// Construct from integer exponents (external API semantics: real exponents).
    pub fn from_exponents(exps: [i16; 7]) -> Dim {
        Dim(exps.map(|e| e * 2))
    }

    /// Real exponents (for display/diagnostics); fractional exponents are truncated to i16 pairs,
    /// so the caller must handle the half itself.
    pub fn exponents(&self) -> [i16; 7] {
        self.0.map(|e| e / 2)
    }

    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|v| *v == 0)
    }

    fn mul_dim(self, o: Dim) -> Dim {
        let mut r = self.0;
        for (a, b) in r.iter_mut().zip(o.0) {
            *a += b;
        }
        Dim(r)
    }

    fn div_dim(self, o: Dim) -> Dim {
        let mut r = self.0;
        for (a, b) in r.iter_mut().zip(o.0) {
            *a -= b;
        }
        Dim(r)
    }

    /// Rational scaling by `p/q` (the `s^(-1/2)` inside a unit string).
    ///
    /// If it does not divide evenly, report an error: dimension exponents are **half-lattice
    /// integers**, `m^(1/3)` does not exist in this algebra — better to reject than to round off a
    /// fake dimension.
    pub fn scale_ratio(self, p: i32, q: i32) -> Result<Dim, Error> {
        if q == 0 {
            return Err(Error::ExponentZeroDenominator {
                exponent: format!("{p}/{q}"),
            });
        }
        let mut r = [0i16; 7];
        for (out, v) in r.iter_mut().zip(self.0) {
            // i64 intermediate + range check: i32 multiplication overflows (`m^(2000000000)` used
            // to panic outright in debug and silently wrap into a **wrong dimension** in release).
            let prod = v as i64 * p as i64;
            if prod % q as i64 != 0 {
                return Err(Error::ExponentNotHalfInteger(format!("{p}/{q}")));
            }
            let scaled = prod / q as i64;
            *out = i16::try_from(scaled).map_err(|_| {
                format!("单位指数 {p}/{q} 超出量纲格范围（半格指数必须落在 i16 内）")
            })?;
        }
        Ok(Dim(r))
    }

    pub fn display(&self) -> String {
        const NAMES: [&str; 7] = ["kg", "m", "s", "A", "K", "mol", "cd"];
        if self.is_zero() {
            return "1".to_string();
        }
        let mut parts = Vec::new();
        for (i, n) in NAMES.iter().enumerate() {
            let half = self.0[i];
            if half == 0 {
                continue;
            }
            if half % 2 == 0 {
                let e = half / 2;
                if e == 1 {
                    parts.push((*n).to_string());
                } else {
                    parts.push(format!("{n}^{e}"));
                }
            } else {
                // Half-lattice: write it as `s^(-7/2)` / `s^(1/2)` (with parentheses, so it is
                // not read as s^-7 then divided by 2)
                parts.push(format!("{n}^({half}/2)"));
            }
        }
        parts.join("·")
    }
}

/// A unit (dimension + conversion factor to SI base units).
///
/// The dimension is a **promise**, the conversion is a **convenience**: the two are stored
/// separately because "`nF` and `F` have the same dimension" and "`1 nF = 1e-9 F`" are two
/// different things. `dim_check` uses only `dim`; `convert` uses both.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitSpec {
    pub dim: Dim,
    /// Multiplier from this unit to SI base units (`nF` → 1e-9; `kg` → 1).
    pub scale: f64,
}

/// SI prefixes (including their symbols), with longer prefixes matched first
/// (`daN` = daN, not d·aN).
const SI_PREFIXES: &[(&str, f64)] = &[
    ("da", 1e1),
    ("Q", 1e30),
    ("R", 1e27),
    ("r", 1e-27),
    ("q", 1e-30),
    ("Y", 1e24),
    ("Z", 1e21),
    ("E", 1e18),
    ("P", 1e15),
    ("T", 1e12),
    ("G", 1e9),
    ("M", 1e6),
    ("k", 1e3),
    ("h", 1e2),
    ("d", 1e-1),
    ("c", 1e-2),
    ("m", 1e-3),
    ("\u{00b5}", 1e-6),
    ("\u{03bc}", 1e-6),
    ("n", 1e-9),
    ("p", 1e-12),
    ("f", 1e-15),
    ("a", 1e-18),
    ("z", 1e-21),
    ("y", 1e-24),
];

/// Prefixed unit name → (dimension, conversion factor).
///
/// The lookup order is **first the full name, then prefix splitting**: `m` is metre and not
/// "milli-nothing", `mol` is mole and not "milli-ol", `cd` is candela and not "centi-d". When
/// splitting prefixes, longer prefixes win, so `daN` is ten newtons and `daPa` is ten pascals.
/// Known but **deliberately rejected** units: offsets (°C/°F) and logarithmic ones (the dB family).
///
/// This must be blocked before prefix splitting: otherwise `degC` gets split into prefix + unknown
/// remainder and reports "unknown unit", pointing people the wrong way — the real cause is "this
/// unit has no multiplicative dimension".
fn rejected_unit(name: &str) -> Option<Error> {
    match name {
        "degC" | "Celsius" | "degF" | "Fahrenheit" | "degR" => {
            Some(Error::OffsetUnit(name.to_string()))
        }
        "dB" | "dBm" | "dBW" | "dBV" | "neper" => Some(Error::LogUnit(name.to_string())),
        // Ambiguous symbols are rejected instead of silently picking one reading:
        // thermochemical (4.184 J) and IT (4.1868 J) calories differ, and `rpm`
        // would collapse the angle axis that `rad` deliberately keeps dimensionless.
        "cal" | "Cal" => Some(Error::AmbiguousUnit {
            name: name.to_string(),
            why: "cal_th = 4.184 J and cal_IT = 4.1868 J differ".to_string(),
            remedy: "write the J value explicitly".to_string(),
        }),
        "rpm" => Some(Error::AmbiguousUnit {
            name: name.to_string(),
            why: "1 rpm = 2*pi/60 rad/s, and rad is dimensionless here".to_string(),
            remedy: "write `2 * pi * f` with f declared in Hz".to_string(),
        }),
        _ => None,
    }
}

fn prefixed_unit(name: &str) -> Result<(Dim, f64), Error> {
    if let Some(error) = rejected_unit(name) {
        return Err(error);
    }
    if let Ok(hit) = unit_base(name) {
        return Ok(hit);
    }
    for (pfx, k) in SI_PREFIXES {
        if let Some(rest) = name.strip_prefix(pfx) {
            if rest.is_empty() {
                continue;
            }
            if let Ok((d, base_scale)) = unit_base(rest) {
                return Ok((d, *k * base_scale));
            }
        }
    }
    Err(Error::UnknownUnit(name.to_string()))
}

fn unit_base(name: &str) -> Result<(Dim, f64), Error> {
    // The table holds **real exponents**; Dim stores half-lattice internally, hence from_exponents
    let d = |a: [i16; 7]| Dim::from_exponents(a);
    let one = |a: [i16; 7]| (d(a), 1.0f64);
    Ok(match name {
        // Base and derived SI units. `1` at the end of the arm is the factor to
        // the SI base unit; grams are the one exception (1 g = 1e-3 kg).
        "kg" => one([1, 0, 0, 0, 0, 0, 0]),
        "g" => (d([1, 0, 0, 0, 0, 0, 0]), 1e-3),
        "m" => one([0, 1, 0, 0, 0, 0, 0]),
        "s" => one([0, 0, 1, 0, 0, 0, 0]),
        "A" => one([0, 0, 0, 1, 0, 0, 0]),
        "K" => one([0, 0, 0, 0, 1, 0, 0]),
        "mol" => one([0, 0, 0, 0, 0, 1, 0]),
        "cd" => one([0, 0, 0, 0, 0, 0, 1]),
        "V" => one([1, 2, -3, -1, 0, 0, 0]),
        "N" => one([1, 1, -2, 0, 0, 0, 0]),
        "J" => one([1, 2, -2, 0, 0, 0, 0]),
        "W" => one([1, 2, -3, 0, 0, 0, 0]),
        "C" => one([0, 0, 1, 1, 0, 0, 0]),
        "Pa" => one([1, -1, -2, 0, 0, 0, 0]),
        "Hz" => one([0, 0, -1, 0, 0, 0, 0]),
        // Derived units electronics cannot avoid: without them EIS is limited to V/A.
        "F" => one([-1, -2, 4, 2, 0, 0, 0]),
        "\u{03a9}" | "ohm" => one([1, 2, -3, -2, 0, 0, 0]),
        "S" => one([-1, -2, 3, 2, 0, 0, 0]),
        "H" => one([1, 2, -2, -2, 0, 0, 0]),
        "Wb" => one([1, 2, -2, -1, 0, 0, 0]),
        "T" => one([1, 0, -2, -1, 0, 0, 0]),
        "rad" | "1" | "sr" => (Dim::ZERO, 1.0),
        // Non-SI units with exact definitions (BIPM SI Brochure 9th edition).
        // Each entry carries its own factor so `convert` stays dimensionally honest.
        "min" => (d([0, 0, 1, 0, 0, 0, 0]), 60.0),
        "h" => (d([0, 0, 1, 0, 0, 0, 0]), 3600.0),
        "d" => (d([0, 0, 1, 0, 0, 0, 0]), 86400.0),
        "L" | "l" => (d([0, 3, 0, 0, 0, 0, 0]), 1e-3),
        "deg" => (Dim::ZERO, core::f64::consts::PI / 180.0),
        "arcmin" => (Dim::ZERO, core::f64::consts::PI / 10_800.0),
        "arcsec" => (Dim::ZERO, core::f64::consts::PI / 648_000.0),
        "bar" => (d([1, -1, -2, 0, 0, 0, 0]), 1e5),
        "atm" => (d([1, -1, -2, 0, 0, 0, 0]), 101_325.0),
        "eV" => (d([1, 2, -2, 0, 0, 0, 0]), 1.602_176_634e-19),
        "Wh" => (d([1, 2, -2, 0, 0, 0, 0]), 3600.0),
        "tonne" => (d([1, 0, 0, 0, 0, 0, 0]), 1000.0),
        "Da" | "u" => (d([1, 0, 0, 0, 0, 0, 0]), 1.660_539_068_92e-27),
        "nmi" => (d([0, 1, 0, 0, 0, 0, 0]), 1852.0),
        "knot" => (d([0, 1, -1, 0, 0, 0, 0]), 1852.0 / 3600.0),
        "ha" => (d([0, 2, 0, 0, 0, 0, 0]), 1e4),
        "acre" => (d([0, 2, 0, 0, 0, 0, 0]), 4_046.856_422_4),
        other => return Err(Error::UnknownUnit(other.to_string())),
    })
}

/// Fractional exponent `-1/2` / `1/2` / `3` → (numerator, denominator).
fn parse_ratio(s: &str) -> Result<(i32, i32), Error> {
    let (n, d) = match s.trim().split_once('/') {
        Some((n, d)) => (n, d),
        None => (s.trim(), "1"),
    };
    let n: i32 = n
        .trim()
        .parse()
        .map_err(|_| format!("坏的单位指数 `{}`", s.trim()))?;
    let d: i32 = d.trim().parse().map_err(|_| {
        Error::MalformedUnit(
            s.trim().to_string(),
            "exponent denominator is not an integer".into(),
        )
    })?;
    if d == 0 {
        return Err(Error::ExponentZeroDenominator {
            exponent: s.trim().to_string(),
        });
    }
    Ok((n, d))
}

/// Parse unit strings such as `A/m^2`, `kg*m/s^2`, `V/A*s^(-1/2)`.
pub fn parse_unit(src: &str) -> Result<Dim, Error> {
    Ok(parse_unit_spec(src)?.dim)
}

/// Parse a unit string, yielding both the dimension and the conversion factor
/// (`nF` → (C/V, 1e-9)).
pub fn parse_unit_spec(src: &str) -> Result<UnitSpec, Error> {
    let src = src.trim();
    if src.is_empty() {
        return Err(Error::MalformedUnit(
            String::new(),
            "empty unit string".into(),
        ));
    }
    // Only ASCII plus two micro signs are allowed (U+00B5 / U+03BC)
    if let Some(c) = src
        .chars()
        .find(|c| !c.is_ascii() && !matches!(*c, '\u{00b5}' | '\u{03bc}' | '\u{03a9}' | '\u{00b7}'))
    {
        return Err(Error::MalformedUnit(
            src.to_string(),
            format!("invalid character `{c}`"),
        ));
    }
    let mut acc = Dim::ZERO;
    let mut scale = 1.0f64;
    let mut op = '*';
    let mut atom = String::new();
    let flush = |acc: &mut Dim, scale: &mut f64, op: char, atom: &mut String| {
        if atom.is_empty() {
            return Err(Error::MalformedUnit(
                src.to_string(),
                "empty component".into(),
            ));
        }
        // Exponents accept rationals p/q: `s^(-1/2)` (the textbook unit of the Warburg
        // coefficient).
        // The token-macro lane renders punctuation with surrounding spaces, so
        // `[m^2]` arrives as `m ^ 2 `. Trim before lookup, otherwise every
        // multi-atom unit becomes "unknown unit `m `" and the formula cannot build.
        let atom_trimmed = atom.trim().to_string();
        let (name, p, q) = match atom_trimmed.split_once('^') {
            Some((n, e)) => {
                // `s^(-1/2)`: the parentheses are required, otherwise the `/` inside the exponent
                // cannot be told apart from the unit string's
                // division sign (the tokenizer only recognizes operators outside parentheses)
                let e = e.trim();
                let e = e
                    .strip_prefix('(')
                    .and_then(|x| x.strip_suffix(')'))
                    .unwrap_or(e);
                let (p, q) = parse_ratio(e)?;
                (n.trim(), p, q)
            }
            None => (atom_trimmed.as_str(), 1, 1),
        };
        let (d, k) = prefixed_unit(name)?;
        let d = d.scale_ratio(p, q)?;
        // The conversion factor is scaled by the exponent p/q: `(mm)^2` = (1e-3)² = 1e-6
        let k = math::powf(k, p as f64 / q as f64);
        *acc = if op == '/' {
            acc.div_dim(d)
        } else {
            acc.mul_dim(d)
        };
        *scale = if op == '/' { *scale / k } else { *scale * k };
        atom.clear();
        Ok(())
    };
    let mut depth = 0i32;
    for c in src.chars() {
        match c {
            '(' => {
                depth += 1;
                atom.push(c);
            }
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return Err(Error::MalformedUnit(
                        src.to_string(),
                        "unbalanced `(`".to_string(),
                    ));
                }
                atom.push(c);
            }
            // `·` (U+00B7) is the separator `Dim::display` emits: what is written back must be
            // readable again,
            // otherwise "write the display result into a decl" is a broken path
            '/' | '*' | '\u{00b7}' if depth == 0 => {
                flush(&mut acc, &mut scale, op, &mut atom)?;
                op = if c == '/' { '/' } else { '*' };
            }
            _ => atom.push(c),
        }
    }
    if depth != 0 {
        return Err(Error::MalformedUnit(
            src.to_string(),
            "unbalanced `(`".to_string(),
        ));
    }
    flush(&mut acc, &mut scale, op, &mut atom)?;
    Ok(UnitSpec { dim: acc, scale })
}

/// Unit-aware conversion: `convert(1.0, "nF", "F") == 1e-9`.
///
/// Different dimensions → error (no "guessing what you meant to convert"). The inputs of
/// generated code are always **SI base units**,
/// conversion happens only at the boundary (reading data / writing reports); hiding the
/// conversion inside the formula is exactly where dimension accidents come from.
pub fn convert(value: f64, from: &str, to: &str) -> Result<f64, Error> {
    let a = parse_unit_spec(from)?;
    let b = parse_unit_spec(to)?;
    if a.dim != b.dim {
        return Err(Error::IncompatibleUnits {
            from: from.to_string(),
            to: to.to_string(),
            from_dim: a.dim.display().to_string(),
            to_dim: b.dim.display().to_string(),
        });
    }
    Ok(value * a.scale / b.scale)
}

// ---------------------------------------------------------------- dimensional analysis (Buckingham Π)

/// Exact rationals (used by Gaussian elimination).
///
/// Why not `f64`: dimension exponents are half-lattice integers, and `f64` elimination leaves
/// residue at the 1e-17 level,
/// so judgments like "this combination is dimensionless" start drifting at random. The dimension
/// is a promise,
/// so the judgment must be **exact**.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rat(i128, i128);

impl Rat {
    fn new(n: i128, d: i128) -> Rat {
        assert!(d != 0, "有理数分母为零");
        let g = gcd(n, d);
        let (n, d) = (n / g, d / g);
        if d < 0 {
            Rat(-n, -d)
        } else {
            Rat(n, d)
        }
    }
    fn zero() -> Rat {
        Rat(0, 1)
    }
    fn is_zero(self) -> bool {
        self.0 == 0
    }
    fn sub(self, o: Rat) -> Rat {
        Rat::new(self.0 * o.1 - o.0 * self.1, self.1 * o.1)
    }
    fn mul(self, o: Rat) -> Rat {
        Rat::new(self.0 * o.0, self.1 * o.1)
    }
    fn div(self, o: Rat) -> Rat {
        Rat::new(self.0 * o.1, self.1 * o.0)
    }
    fn to_f64(self) -> f64 {
        self.0 as f64 / self.1 as f64
    }
}

#[cfg(test)]
fn contains_pow_of(e: &Expr, name: &str, k: f64) -> bool {
    match e {
        Expr::Pow(a, b) => {
            (matches!(a.as_ref(), Expr::Var(n) if n == name)
                && matches!(b.as_ref(), Expr::Num(v) if *v == k))
                || contains_pow_of(a, name, k)
                || contains_pow_of(b, name, k)
        }
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => {
            contains_pow_of(a, name, k) || contains_pow_of(b, name, k)
        }
        Expr::Neg(a)
        | Expr::Transpose(a)
        | Expr::Grad(a)
        | Expr::Laplacian(a)
        | Expr::Deriv(a, _) => contains_pow_of(a, name, k),
        Expr::Call(_, args) => args.iter().any(|a| contains_pow_of(a, name, k)),
        Expr::Mat(rows) => rows.iter().flatten().any(|c| contains_pow_of(c, name, k)),
        Expr::MatPattern(p) => p
            .elements()
            .into_iter()
            .any(|c| contains_pow_of(c, name, k)),
        Expr::Sum {
            lower, upper, body, ..
        }
        | Expr::Integral {
            lower, upper, body, ..
        } => {
            contains_pow_of(lower, name, k)
                || contains_pow_of(upper, name, k)
                || contains_pow_of(body, name, k)
        }
        _ => false,
    }
}

fn gcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    if a == 0 {
        1
    } else {
        a
    }
}

/// Dimension → exponents of the 7 base quantities (rationals; half-lattice means `/2`).
fn dim_rat(d: Dim) -> Vec<Rat> {
    d.0.iter().map(|h| Rat::new(*h as i128, 2)).collect()
}

/// Row-reduced echelon form of a 7×(n+1) augmented matrix. Returns
/// (matrix, pivot columns, consistent).
// The column index `j` is the **semantic index** of the elimination (row swaps are done by column
// within one row), not a single-set iteration.
#[allow(clippy::needless_range_loop)]
fn rref_aug(mut a: Vec<Vec<Rat>>) -> (Vec<Vec<Rat>>, Vec<usize>, bool) {
    let rows = a.len();
    let cols = a.first().map(|r| r.len()).unwrap_or(0);
    let mut pivots: Vec<usize> = Vec::new();
    let mut r = 0usize;
    for c in 0..cols.saturating_sub(1) {
        if r == rows {
            break;
        }
        let Some(p) = (r..rows).find(|&i| !a[i][c].is_zero()) else {
            continue;
        };
        a.swap(r, p);
        let inv = a[r][c];
        for j in c..cols {
            a[r][j] = a[r][j].div(inv);
        }
        for i in 0..rows {
            if i != r && !a[i][c].is_zero() {
                let f = a[i][c];
                for j in c..cols {
                    let sub = f.mul(a[r][j]);
                    a[i][j] = a[i][j].sub(sub);
                }
            }
        }
        pivots.push(c);
        r += 1;
    }
    let consistent = !(0..rows).any(|i| {
        (0..cols.saturating_sub(1)).all(|j| a[i][j].is_zero()) && !a[i][cols - 1].is_zero()
    });
    (a, pivots, consistent)
}

/// Dimensionless groups (Buckingham Π theorem).
///
/// Given a set of "variable name + dimension", returns a **basis** of dimensionless
/// combinations: each element is the exponent vector of the variables (rationals; this facility
/// goes only down to half-integers); multiplying/dividing the variables by those exponents yields
/// a pure number.
/// The number of groups = number of variables − rank of the exponent matrix (the conclusion of the
/// Π theorem itself, used as an assertion in tests).
pub fn dimensionless_groups(vars: &[(String, Dim)]) -> Result<Vec<Vec<f64>>, Error> {
    if vars.is_empty() {
        return Err(Error::InvalidInput(
            "dimensionless groups need at least one variable".into(),
        ));
    }
    let n = vars.len();
    // 7×n exponent matrix + a redundant all-zero augmented column (to reuse the same rref)
    let mut a: Vec<Vec<Rat>> = (0..7)
        .map(|row| {
            let mut r: Vec<Rat> = vars.iter().map(|(_, d)| dim_rat(*d)[row]).collect();
            r.push(Rat::zero());
            r
        })
        .collect();
    let (red, pivots, _) = rref_aug(core::mem::take(&mut a));
    let mut basis: Vec<Vec<f64>> = Vec::new();
    for f in 0..n {
        if pivots.contains(&f) {
            continue;
        }
        // Set the free variable f to 1 and back-substitute the pivot variables
        let mut x = vec![0.0f64; n];
        x[f] = 1.0;
        for (i, &pc) in pivots.iter().enumerate() {
            x[pc] = -red[i][f].to_f64();
        }
        // Sign normalization: the sign of a basis vector is representational redundancy (both Π
        // and its reciprocal are legal groups),
        // so pinning "the first non-zero exponent is positive" makes the output deterministic and
        // comparable.
        if x.iter().find(|e| **e != 0.0).is_some_and(|e| *e < 0.0) {
            for e in x.iter_mut() {
                *e = -*e;
            }
        }
        basis.push(x);
    }
    Ok(basis)
}

/// Decide "can the target dimension be formed by multiplying/dividing powers of these
/// variables?", and if so give one set of exponents.
///
/// This and `dimensionless_groups` are two uses of the same system of equations: one solves the
/// homogeneous case
/// (forming a dimensionless group), the other the inhomogeneous case (forming the target
/// dimension).
pub fn dim_solve(vars: &[(String, Dim)], target: Dim) -> Result<Vec<f64>, Error> {
    if vars.is_empty() {
        return if target.is_zero() {
            Ok(vec![])
        } else {
            Err(Error::InvalidInput(format!(
                "no variables to form the dimension {}",
                target.display()
            )))
        };
    }
    let n = vars.len();
    let t = dim_rat(target);
    let a: Vec<Vec<Rat>> = (0..7)
        .map(|row| {
            let mut r: Vec<Rat> = vars.iter().map(|(_, d)| dim_rat(*d)[row]).collect();
            r.push(t[row]);
            r
        })
        .collect();
    let (red, pivots, consistent) = rref_aug(a);
    if !consistent {
        return Err(Error::TargetDimensionUnreachable {
            target: target.display().to_string(),
            needed: t.iter().filter(|r| !r.is_zero()).count(),
            spanned: pivots.len(),
        });
    }
    let mut x = vec![0.0f64; n];
    for (i, &pc) in pivots.iter().enumerate() {
        x[pc] = red[i][n].to_f64();
    }
    Ok(x)
}

/// Textual form of a dimensionless group: `rho^1·v^1·L^1·mu^-1`.
pub fn pi_text(names: &[String], exps: &[f64]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (n, e) in names.iter().zip(exps) {
        if *e == 0.0 {
            continue;
        }
        if *e == 1.0 {
            parts.push(n.clone());
        } else {
            parts.push(format!("{n}^{}", fmt_num(*e)));
        }
    }
    if parts.is_empty() {
        "1".to_string()
    } else {
        parts.join("·")
    }
}
// ---------------------------------------------------------------- boundary guards

/// Maximum nesting depth the IR allows.
///
/// After parsing succeeds, **every** downstream stage (`dims` / `diff` / `simplify` / `key` /
/// `to_latex` / `emit` / `eval`) is implemented recursively — a deliberately deep input
/// (`((((…x…))))`) can overflow their whole stack, which is a SIGSEGV, not a readable error.
/// So the depth is **checked once right after lowering**, turning "stack overflow" into "a
/// rejection carrying a number".
/// The limit is 2048: a long chain `a + b + c + …` is **left-nested** in the IR, so the depth
/// equals the number of terms, and long series/long polynomials routinely have thousands of
/// terms. Recursion with small frames at 2048 levels takes about 1MB on an 8MB stack, which is
/// safe;
/// any deeper is rejected explicitly (instead of letting some downstream stage overflow).
pub const MAX_DEPTH: usize = 2048;

/// Maximum bracket nesting accepted by the front-ends before parsing.
///
/// The IR depth limit above is checked *after* a tree exists; parser recursion
/// happens before that. `pest`'s optional `stack-guard` feature detects the host
/// stack, but it is unavailable in `no_std` builds, so this byte-level guard is
/// the portable line of defence. 256 nested groups is far beyond handwritten
/// formulas yet well inside a default thread stack.
pub const MAX_PARSE_NESTING: i32 = 256;

/// Upper bound on the explicit loop steps for sums/integrals (a teaching lane, not a numerics
/// library: do not let it become a DoS).
pub const MAX_STEPS: u64 = 10_000_000;

/// Source text scan: reject **invisible** and **control** characters.
///
/// They do not make the grammar fail (in pest they are mostly rejected outright as unknown
/// characters, but inside comments/strings they pass silently), yet they can make a formula "look
/// different from what it actually is" during review:
/// zero-width spaces, bidirectional overrides and zero-width joiners are real social-engineering
/// surfaces.
pub fn scan_source(src: &str) -> Result<(), FrontError> {
    // Deterministic nesting guard that does not depend on the host stack or on
    // pest's optional `stack-guard` feature. Both std and no_std builds need it:
    // without it, deeply nested input aborts the process instead of reporting.
    let mut depth = 0i32;
    for (i, c) in src.char_indices() {
        match c {
            '(' | '[' | '{' => {
                depth += 1;
                if depth > MAX_PARSE_NESTING {
                    let (line, col) = line_col_at(src, i);
                    return Err(FrontError::coded(
                        i18n::Error::NestingTooDeep {
                            got: depth.to_string(),
                            limit: MAX_PARSE_NESTING.to_string(),
                        },
                        &[],
                        line,
                        col,
                    ));
                }
            }
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
    }
    for (i, c) in src.char_indices() {
        let forbidden = match c {
            '\u{0000}' => Some("NUL"),
            '\u{0009}' | '\u{000a}' | '\u{000d}' => None, // tab/newline are layout
            c if (c as u32) < 0x20 => Some("control character"),
            '\u{200b}'..='\u{200f}' => Some("zero-width character"),
            '\u{2028}' | '\u{2029}' => Some("line/paragraph separator"),
            '\u{202a}'..='\u{202e}' => Some("bidirectional override"),
            '\u{2060}'..='\u{2064}' => Some("invisible joiner"),
            '\u{feff}' => Some("byte order mark"),
            _ => None,
        };
        if let Some(kind) = forbidden {
            let (line, col) = line_col_at(src, i);
            let hex = format!("{:04X}", c as u32);
            return Err(FrontError::coded(
                i18n::Error::InvisibleCharacter(format!("{hex} ({kind})")),
                &[],
                line,
                col,
            ));
        }
    }
    Ok(())
}

fn line_col_at(src: &str, byte: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, c) in src.char_indices() {
        if i >= byte {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Expression depth (a leaf = 1). Used as a guard before persisting/before emitting.
pub fn depth_of(e: &Expr) -> usize {
    let kids: Vec<&Expr> = match e {
        Expr::Num(_) | Expr::Var(_) | Expr::Indexed { .. } => Vec::new(),
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
            vec![a, b]
        }
        Expr::Neg(a)
        | Expr::Transpose(a)
        | Expr::Grad(a)
        | Expr::Laplacian(a)
        | Expr::Deriv(a, _) => {
            vec![a]
        }
        Expr::Call(_, args) => args.iter().collect(),
        Expr::Mat(rows) => rows.iter().flatten().collect(),
        Expr::MatPattern(p) => p.elements(),
        Expr::Sum {
            lower, upper, body, ..
        }
        | Expr::Integral {
            lower, upper, body, ..
        } => vec![lower, upper, body],
    };
    1 + kids.into_iter().map(depth_of).max().unwrap_or(0)
}

/// Guard over the whole formula set (called once at the end of the front-end parse):
/// depth + function arity.
pub fn check_set_depth(set: &FormulaSet) -> Result<(), FrontError> {
    // A set with declarations but no formula is a valid AST and an invalid asset: guard it here
    // so hand-built sets (federation, loaders, tests) get the catalogued error, not a silent
    // empty success downstream.
    if set.items.is_empty() {
        return Err(FrontError::plain(Error::EmptyFormula));
    }
    let runtime_shapes = runtime::shapes_of_set(set).map_err(FrontError::plain)?;
    for item in &set.items {
        check_depth(&item.expr).map_err(FrontError::plain)?;
        check_calls_with_shapes(&item.expr, &runtime_shapes).map_err(FrontError::plain)?;
    }
    Ok(())
}

/// Reserved-word collision check for declaration names (`decl { exp : ... }` would shadow the
/// function name).
pub fn check_decl_names(set: &FormulaSet) -> Result<(), FrontError> {
    const RESERVED: &[&str] = &["j"];
    for d in &set.decls {
        if func_arity(&d.name).is_some() {
            // Catalogued variant; the payload is the colliding name and the advice rides in
            // the second slot so each locale can phrase it.
            return Err(FrontError::plain(Error::ReservedName {
                name: d.name.clone(),
                reason: i18n::ReservedReason::BuiltinFunction,
            }));
        }
        if RESERVED.contains(&d.name.as_str()) {
            return Err(FrontError::plain(Error::ReservedName {
                name: d.name.clone(),
                reason: i18n::ReservedReason::ImaginaryUnit,
            }));
        }
    }
    Ok(())
}

/// Depth guard: reject outright once `MAX_DEPTH` is exceeded (to avoid downstream recursive stack
/// overflow).
pub fn check_depth(e: &Expr) -> Result<(), Error> {
    let d = depth_of(e);
    if d > MAX_DEPTH {
        return Err(Error::NestingTooDeep {
            got: d.to_string(),
            limit: MAX_DEPTH.to_string(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------- IR

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Num(f64),
    Var(String),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
    Call(String, Vec<Expr>),
    Mat(Vec<Vec<Expr>>),
    /// A **pattern matrix** with ellipses: a "hand-written sketch" of a dynamic number of rows
    /// and columns.
    /// Before evaluation it must first be `recognize_pattern`-ed into a builtin shape
    /// (identity / diag / tridiag).
    MatPattern(MatPattern),
    /// Transpose `A^T` / `A'` (file lane only; `'` is a lexical error in the macro token lane).
    Transpose(Box<Expr>),
    /// ∂u/∂x — symbolic differentiation (not evaluated; dimension = dim(u)/dim(x)).
    Deriv(Box<Expr>, String),
    /// ∇u — gradient (dimension = dim(u)/m, Cartesian SI metric assumption).
    Grad(Box<Expr>),
    /// ∇²u / Δu — Laplacian (dimension = dim(u)/m²).
    Laplacian(Box<Expr>),
    /// Σ_{var=lower}^{upper} body — finite sum (**evaluable**; var is a bound variable).
    Sum {
        var: String,
        lower: Box<Expr>,
        upper: Box<Expr>,
        body: Box<Expr>,
    },
    /// ∫_lower^upper body d var — definite integral (evaluated by Simpson's rule;
    /// dimension = dim(body)·dim(var)).
    Integral {
        var: String,
        lower: Box<Expr>,
        upper: Box<Expr>,
        body: Box<Expr>,
    },
    /// Indexed symbol a_{i,j} — name + index list (the atom of tensor expressions).
    Indexed {
        name: String,
        indices: Vec<String>,
    },
}

/// One cell of a pattern matrix: a concrete element, or an ellipsis (its direction is inferred
/// from the neighbours when writing back).
#[derive(Clone, Debug, PartialEq)]
pub enum PatternCell {
    El(Expr),
    Dots,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatPattern {
    pub rows: Vec<Vec<PatternCell>>,
}

impl MatPattern {
    /// The set of concrete cells' dimensions (ellipses do not participate).
    pub fn elements(&self) -> Vec<&Expr> {
        self.rows
            .iter()
            .flatten()
            .filter_map(|c| match c {
                PatternCell::El(e) => Some(e),
                PatternCell::Dots => None,
            })
            .collect()
    }

    pub fn grid(&self) -> Option<(usize, usize)> {
        let cols = self.rows.first()?.len();
        if self.rows.iter().all(|r| r.len() == cols) {
            Some((self.rows.len(), cols))
        } else {
            None
        }
    }
}

/// Builtin matrix shapes that can be generated. A pattern matrix **must** first be recognized as
/// one of these;
/// if it cannot be recognized, report an error and list the supported shapes — rejection is a
/// feature.
#[derive(Clone, Debug, PartialEq)]
pub enum MatrixKind {
    /// `[1 ⋯ 0; ⋯ 1 ⋯; 0 ⋯ 1]` → `identity(n)`
    Identity,
    /// `[d ⋯ 0; ⋯ d ⋯; 0 ⋯ d]` (same expression on the diagonal) → `diag(d, n)`
    Diag(Expr),
    /// The diagonal is an index sequence `d_{1} d_{2} d_{3}` → vector parameter `d: &[f64]`
    DiagVec(String),
    /// `[b c ⋯ 0; a b c ⋯; ⋯ a b c; 0 ⋯ a b]` → `tridiag(a, b, c, n)`
    Tridiag { a: Expr, b: Expr, c: Expr },
    /// Tridiagonal, but the diagonal is an index sequence → `tridiag_v(a, b: &[f64], c, n)`
    TridiagVec { a: Expr, name: String, c: Expr },
    /// Upper bidiagonal `[b c; · b c; ⋯ · b]` → `bidiag_u(b, c, n)`
    BidiagUpper { b: Expr, c: Expr },
    /// Lower bidiagonal `[b · ⋯ 0; a b ·; 0 ⋯ a b]` → `bidiag_l(a, b, n)`
    BidiagLower { a: Expr, b: Expr },
    /// Block diagonal `[P 0; 0 Q]` → `blockdiag2(P, Q, n, m)` (the two blocks may be equal or
    /// different)
    BlockDiag2 { p: Expr, q: Expr },
    /// Symmetric banded: half-bandwidth k (main diagonal + k sub-diagonals with the same value)
    /// → `banded(v, n)`
    /// e.g. k=1 is the equal-value tridiagonal special case, k=2 is pentadiagonal.
    Banded { v: Expr, half_band: usize },
}

/// Extract a single index from an Indexed expression.
fn as_indexed(e: &Expr) -> Option<(&str, &str)> {
    match e {
        Expr::Indexed { name, indices } if indices.len() == 1 => {
            Some((name.as_str(), indices[0].as_str()))
        }
        _ => None,
    }
}

/// Whether the diagonal cells form an **index sequence**: same name + indices that are all the
/// same letter (general term) or all distinct pure numbers (a sequence).
fn indexed_diag(cells: &[&Expr]) -> Option<String> {
    let mut name: Option<&str> = None;
    let mut letters: Option<&str> = None;
    let mut nums: Vec<&str> = Vec::new();
    for c in cells {
        let (n, idx) = as_indexed(c)?;
        match name {
            None => name = Some(n),
            Some(prev) if prev == n => {}
            _ => return None,
        }
        if idx.chars().all(|ch| ch.is_ascii_digit()) {
            if nums.contains(&idx) {
                return None;
            }
            nums.push(idx);
        } else if idx.chars().all(|ch| ch.is_ascii_alphabetic()) {
            match letters {
                None => letters = Some(idx),
                Some(prev) if prev == idx => {}
                _ => return None,
            }
        } else {
            return None;
        }
    }
    if letters.is_some() && !nums.is_empty() {
        return None;
    }
    name.map(|n| n.to_string())
}

pub fn num(v: f64) -> Expr {
    Expr::Num(v)
}
pub fn var(s: impl Into<String>) -> Expr {
    Expr::Var(s.into())
}
pub fn add(a: Expr, b: Expr) -> Expr {
    Expr::Add(Box::new(a), Box::new(b))
}
pub fn sub(a: Expr, b: Expr) -> Expr {
    Expr::Sub(Box::new(a), Box::new(b))
}
pub fn mul(a: Expr, b: Expr) -> Expr {
    Expr::Mul(Box::new(a), Box::new(b))
}
pub fn div(a: Expr, b: Expr) -> Expr {
    Expr::Div(Box::new(a), Box::new(b))
}
pub fn pow(a: Expr, b: Expr) -> Expr {
    Expr::Pow(Box::new(a), Box::new(b))
}
pub fn neg(a: Expr) -> Expr {
    Expr::Neg(Box::new(a))
}
pub fn transpose(a: Expr) -> Expr {
    // The transpose of a scalar is the identity (treat a scalar as 1×1): normalize here so the IR
    // never holds
    // `Transpose(Num)` — its write-back `-4'`/`4''` makes the semantics of each side drift
    // (the matex grammar allows only one transpose suffix).
    match a {
        Expr::Num(_) => a,
        other => Expr::Transpose(Box::new(other)),
    }
}
pub fn deriv(u: Expr, by: impl Into<String>) -> Expr {
    Expr::Deriv(Box::new(u), by.into())
}
pub fn grad(u: Expr) -> Expr {
    Expr::Grad(Box::new(u))
}
pub fn laplacian(u: Expr) -> Expr {
    Expr::Laplacian(Box::new(u))
}
pub fn sum(var: impl Into<String>, lower: Expr, upper: Expr, body: Expr) -> Expr {
    Expr::Sum {
        var: var.into(),
        lower: Box::new(lower),
        upper: Box::new(upper),
        body: Box::new(body),
    }
}
pub fn integral(var: impl Into<String>, lower: Expr, upper: Expr, body: Expr) -> Expr {
    Expr::Integral {
        var: var.into(),
        lower: Box::new(lower),
        upper: Box::new(upper),
        body: Box::new(body),
    }
}
pub fn indexed(name: impl Into<String>, indices: Vec<String>) -> Expr {
    Expr::Indexed {
        name: name.into(),
        indices,
    }
}

// ---------------------------------------------------------------- federated formulas

/// One piece of formula source = a declaration block + one or more formulas (the "federation"
/// model: later entries may reference the names of earlier ones).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FormulaSet {
    pub decls: Vec<Decl>,
    pub items: Vec<FormulaItem>,
    /// Dependencies from `use "relative path"` (in order of appearance). **Parse time only
    /// collects them, no IO is done** —
    /// the formula front end is a pure string API; filesystem matters belong to the loader lane.
    pub uses: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FormulaItem {
    pub name: Option<String>,
    pub expr: Expr,
    /// Starting line in the source file (1-based; filled in by the front end). 0 = unknown
    /// (synthesized/test construction).
    pub line: usize,
}

impl FormulaSet {
    /// Convenience accessor for a single formula (compatible with old call sites); errors when
    /// there are several.
    pub fn single(&self) -> Result<&Expr, Error> {
        match self.items.as_slice() {
            [one] => Ok(&one.expr),
            _ => Err(Error::ExpectedSingleFormula {
                got: self.items.len(),
            }),
        }
    }

    /// Single-formula compatibility unwrap: (decls, expr). With several formulas it takes the
    /// **last** one (federation semantics: the final value).
    /// Callers needing strict single-formula semantics should use `single()`.
    pub fn into_single(self) -> (Vec<Decl>, Expr) {
        let expr = self
            .items
            .last()
            .map(|i| i.expr.clone())
            .expect("FormulaSet 至少一条（parse 保证）");
        (self.decls, expr)
    }
}

/// Symbolic substitution: replace every occurrence of `name` with `by` (inline expansion of a
/// federation reference).
pub fn subst(e: &Expr, name: &str, by: &Expr) -> Expr {
    match e {
        Expr::Num(_) => e.clone(),
        Expr::Var(n) => {
            if n == name {
                by.clone()
            } else {
                e.clone()
            }
        }
        Expr::Add(a, b) => add(subst(a, name, by), subst(b, name, by)),
        Expr::Sub(a, b) => sub(subst(a, name, by), subst(b, name, by)),
        Expr::Mul(a, b) => mul(subst(a, name, by), subst(b, name, by)),
        Expr::Div(a, b) => div(subst(a, name, by), subst(b, name, by)),
        Expr::Pow(a, b) => pow(subst(a, name, by), subst(b, name, by)),
        Expr::Neg(a) => neg(subst(a, name, by)),
        Expr::Call(n, args) => {
            Expr::Call(n.clone(), args.iter().map(|a| subst(a, name, by)).collect())
        }
        Expr::Mat(rows) => Expr::Mat(
            rows.iter()
                .map(|r| r.iter().map(|c| subst(c, name, by)).collect())
                .collect(),
        ),
        Expr::MatPattern(p) => Expr::MatPattern(MatPattern {
            rows: p
                .rows
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|c| match c {
                            PatternCell::El(e) => PatternCell::El(subst(e, name, by)),
                            PatternCell::Dots => PatternCell::Dots,
                        })
                        .collect()
                })
                .collect(),
        }),
        Expr::Transpose(a) => transpose(subst(a, name, by)),
        Expr::Deriv(u, by_var) => deriv(subst(u, name, by), by_var.clone()),
        Expr::Grad(u) => grad(subst(u, name, by)),
        Expr::Laplacian(u) => laplacian(subst(u, name, by)),
        Expr::Sum {
            var,
            lower,
            upper,
            body,
        } => sum(
            var.clone(),
            subst(lower, name, by),
            subst(upper, name, by),
            subst(body, name, by),
        ),
        Expr::Integral {
            var,
            lower,
            upper,
            body,
        } => integral(
            var.clone(),
            subst(lower, name, by),
            subst(upper, name, by),
            subst(body, name, by),
        ),
        Expr::Indexed { name: n, indices } => indexed(
            if n == name {
                by_var_name(by)
            } else {
                n.clone()
            },
            indices.clone(),
        ),
    }
}

/// Naming when the substitution target degenerates to a name (the name slot of Indexed is a
/// String, not an Expr).
fn by_var_name(by: &Expr) -> String {
    match by {
        Expr::Var(n) => n.clone(),
        other => format!("__sub_{:x}", other as *const _ as usize),
    }
}

/// Federated dimension table: decls as the base, then the names of earlier formulas merged in
/// with the dimension of their expressions —
/// so referencing `area` inside `vol = area * h` is legal, and dimensions propagate.
pub fn dims_of_set(set: &FormulaSet) -> Result<HashMap<String, Dim>, Error> {
    let declared_dims = dims_of(&set.decls)?;
    let runtime_shapes = runtime::shapes_of_set(set).map_err(|error| error.to_string())?;
    let mut dims = declared_dims.clone();
    // A formula may reuse a decl's name (a = F/m), but the result must honor the dimension already
    // declared for that name;
    // otherwise s : Pa; s = F/d silently turns the same key into a different physical quantity.
    // Duplicate formula names are rejected outright, instead of waiting for rustc to report E0428
    // in the generated code.
    let mut item_names = BTreeSet::new();
    for item in &set.items {
        let Some(name) = &item.name else { continue };
        if !item_names.insert(name) {
            return Err(Error::DuplicateDeclaration {
                name: name.clone(),
                line: Some(item.line),
            });
        }
    }
    // **Federate first, then check**: when a later entry references an earlier name, the check
    // must see the expanded expression,
    // otherwise structural information such as "shape" (`det` needs the square order) is lost on
    // `Var(name)`.
    let items = federate_inline(set)?;
    for item in &items {
        if let Some(name) = &item.name {
            // Fold first: exponents such as `^(1/2)` must converge to Num before the dimension check
            let d = runtime::dim_check_runtime(&simplify(&item.expr), &dims, &runtime_shapes)?;
            if let Some(declared) = declared_dims.get(name) {
                if declared != &d {
                    return Err(Error::ResultDimensionMismatch {
                        name: name.clone(),
                        got: d.display(),
                        declared: declared.display(),
                        line: item.line,
                    });
                }
            }
            dims.insert(name.clone(), d);
        }
    }
    Ok(dims)
}

/// Federation expansion: starting from the second entry, **substitute** earlier named formulas
/// into the references (no function calls, each entry self-contained).
/// The cost is duplicated computation (the caller's CSE only helps within a single entry); the
/// macro inlining lane uses a let chain and pays no such cost.
pub fn federate_inline(set: &FormulaSet) -> Result<Vec<FormulaItem>, Error> {
    let mut acc: Vec<(String, Expr)> = Vec::new();
    let mut out = Vec::new();
    for item in &set.items {
        let mut e = item.expr.clone();
        for (n, v) in &acc {
            e = subst(&e, n, v);
        }
        e = simplify(&e);
        if let Some(n) = &item.name {
            acc.push((n.clone(), e.clone()));
        }
        out.push(FormulaItem {
            name: item.name.clone(),
            expr: e,
            line: item.line,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------- evaluation

pub fn eval(e: &Expr, env: &HashMap<String, f64>) -> Result<f64, Error> {
    let v = eval_value(e, env)?;
    let c = v.as_scalar().ok_or_else(|| {
        Error::MatrixScalarMix("scalar evaluation met a matrix; use eval_value".into())
    })?;
    if !c.is_real() {
        return Err(Error::ComplexEvalResult { re: c.re, im: c.im });
    }
    Ok(c.re)
}

/// Parameter order = **declaration order** (not lexicographic): the generated function signature
/// must be readable by a human in formula order.
///
/// Reserved constants (`j` = imaginary unit) **do not go into the parameter list**: the emitter
/// supplies `(0, 1)` itself;
/// putting it in the signature would be the absurd API of "making the caller pass the imaginary
/// unit".
pub fn params_in_decl_order(decls: &[Decl], e: &Expr) -> Vec<String> {
    // `j` is the reserved imaginary unit only when **not declared**; if the user explicitly
    // declared j (e.g. complex current density)
    // it must appear in the signature, otherwise code generation and evaluation semantics
    // diverge.
    let j_reserved = !decls.iter().any(|d| d.name == "j");
    let used: Vec<String> = vars_of(e)
        .into_iter()
        .filter(|n| !(j_reserved && n == "j"))
        .collect();
    let mut out: Vec<String> = decls
        .iter()
        .map(|d| d.name.clone())
        .filter(|n| used.contains(n))
        .collect();
    for u in used {
        if !out.contains(&u) {
            out.push(u);
        }
    }
    out
}

pub fn vars_of(e: &Expr) -> Vec<String> {
    fn go(e: &Expr, out: &mut Vec<String>) {
        match e {
            Expr::Num(_) => {}
            Expr::Var(n) => {
                if !out.contains(n) {
                    out.push(n.clone());
                }
            }
            Expr::Add(a, b)
            | Expr::Sub(a, b)
            | Expr::Mul(a, b)
            | Expr::Div(a, b)
            | Expr::Pow(a, b) => {
                go(a, out);
                go(b, out);
            }
            Expr::Neg(a) => go(a, out),
            Expr::Call(_, args) => {
                for a in args {
                    go(a, out);
                }
            }
            Expr::Mat(rows) => {
                for r in rows {
                    for c in r {
                        go(c, out);
                    }
                }
            }
            Expr::MatPattern(p) => {
                for e in p.elements() {
                    go(e, out);
                }
            }
            Expr::Transpose(a) => go(a, out),
            Expr::Deriv(u, by) => {
                go(u, out);
                let _ = by; // the differentiation variable is bound, not free
            }
            Expr::Grad(u) | Expr::Laplacian(u) => go(u, out),
            Expr::Sum {
                var,
                lower,
                upper,
                body,
            } => {
                go(lower, out);
                go(upper, out);
                go(body, out);
                out.retain(|n| n != var); // the summation index is bound
            }
            Expr::Integral {
                var,
                lower,
                upper,
                body,
            } => {
                go(lower, out);
                go(upper, out);
                go(body, out);
                out.retain(|n| n != var); // the integration variable is bound
            }
            Expr::Indexed { name, indices } => {
                // The indexed symbol itself is a free variable; its indices (i, j) are free
                // symbols as well
                let mut item = name.clone();
                item.push_str("__idx");
                let _ = item;
                if !out.contains(name) {
                    out.push(name.clone());
                }
                for i in indices {
                    // Numeric indices are values, not free parameters. Symbolic indices
                    // remain free until the runtime index lane handles them explicitly.
                    if i.parse::<usize>().is_err() && !out.contains(i) {
                        out.push(i.clone());
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    go(e, &mut out);
    out.sort();
    out
}

// ---------------------------------------------------------------- simplification

pub fn simplify(e: &Expr) -> Expr {
    let e = match e {
        Expr::Num(_) | Expr::Var(_) => e.clone(),
        Expr::Add(a, b) => add(simplify(a), simplify(b)),
        Expr::Sub(a, b) => sub(simplify(a), simplify(b)),
        Expr::Mul(a, b) => mul(simplify(a), simplify(b)),
        Expr::Div(a, b) => div(simplify(a), simplify(b)),
        Expr::Pow(a, b) => pow(simplify(a), simplify(b)),
        Expr::Neg(a) => neg(simplify(a)),
        Expr::Call(n, args) => Expr::Call(n.clone(), args.iter().map(simplify).collect()),
        Expr::Mat(rows) => Expr::Mat(
            rows.iter()
                .map(|r| r.iter().map(simplify).collect())
                .collect(),
        ),
        Expr::MatPattern(p) => Expr::MatPattern(MatPattern {
            rows: p
                .rows
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|c| match c {
                            PatternCell::El(e) => PatternCell::El(simplify(e)),
                            PatternCell::Dots => PatternCell::Dots,
                        })
                        .collect()
                })
                .collect(),
        }),
        Expr::Transpose(a) => transpose(simplify(a)),
        Expr::Deriv(u, by) => deriv(simplify(u), by.clone()),
        Expr::Grad(u) => grad(simplify(u)),
        Expr::Laplacian(u) => laplacian(simplify(u)),
        Expr::Sum {
            var,
            lower,
            upper,
            body,
        } => sum(
            var.clone(),
            simplify(lower),
            simplify(upper),
            simplify(body),
        ),
        Expr::Integral {
            var,
            lower,
            upper,
            body,
        } => integral(
            var.clone(),
            simplify(lower),
            simplify(upper),
            simplify(body),
        ),
        Expr::Indexed { name, indices } => indexed(name.clone(), indices.clone()),
    };
    fold_const(&e)
}

fn fold_const(e: &Expr) -> Expr {
    let z = |x: &Expr| matches!(x, Expr::Num(v) if *v == 0.0);
    let one = |x: &Expr| matches!(x, Expr::Num(v) if *v == 1.0);
    match e {
        Expr::Neg(a) => match a.as_ref() {
            Expr::Num(v) => num(-*v),
            Expr::Neg(b) => (**b).clone(),
            _ => neg((**a).clone()),
        },
        Expr::Add(a, b) => {
            if let (Expr::Num(x), Expr::Num(y)) = (a.as_ref(), b.as_ref()) {
                return num(x + y);
            }
            if z(a) {
                return (**b).clone();
            }
            if z(b) {
                return (**a).clone();
            }
            add((**a).clone(), (**b).clone())
        }
        Expr::Sub(a, b) => {
            if let (Expr::Num(x), Expr::Num(y)) = (a.as_ref(), b.as_ref()) {
                return num(x - y);
            }
            if z(b) {
                return (**a).clone();
            }
            if z(a) {
                return neg((**b).clone());
            }
            sub((**a).clone(), (**b).clone())
        }
        Expr::Mul(a, b) => {
            if let (Expr::Num(x), Expr::Num(y)) = (a.as_ref(), b.as_ref()) {
                return num(x * y);
            }
            if z(a) || z(b) {
                return num(0.0);
            }
            if one(a) {
                return (**b).clone();
            }
            if one(b) {
                return (**a).clone();
            }
            if let Expr::Num(v) = a.as_ref() {
                if *v == -1.0 {
                    return neg((**b).clone());
                }
            }
            mul((**a).clone(), (**b).clone())
        }
        Expr::Div(a, b) => {
            if let (Expr::Num(x), Expr::Num(y)) = (a.as_ref(), b.as_ref()) {
                return num(x / y);
            }
            if z(a) {
                return num(0.0);
            }
            if one(b) {
                return (**a).clone();
            }
            div((**a).clone(), (**b).clone())
        }
        Expr::Pow(a, b) => {
            if let (Expr::Num(x), Expr::Num(y)) = (a.as_ref(), b.as_ref()) {
                // Negative real base + non-integer exponent: the value layer gives a **complex**
                // number (principal branch), and folding it to
                // `powf` here would produce NaN — two lanes giving two answers is the worst form of
                // defect.
                // Do not fold; leave it for the evaluation layer to compute with complex
                // semantics.
                if *x < 0.0 && math::fract(*y) != 0.0 {
                    return e.clone();
                }
                return num(math::powf(*x, *y));
            }
            if let Expr::Num(y) = b.as_ref() {
                if *y == 1.0 {
                    return (**a).clone();
                }
                if *y == 0.0 && !matrix_ish(a) {
                    return num(1.0);
                }
            }
            pow((**a).clone(), (**b).clone())
        }
        _ => e.clone(),
    }
}

// ---------------------------------------------------------------- complex numbers

/// Complex number (real part + imaginary part).
///
/// The value layer uses `C` **uniformly**, rather than two variants "real scalar / complex
/// scalar": in scenarios like EIS
/// the matrix elements themselves may be complex, and two variants would immediately degenerate
/// into the four quadrants "real scalar / complex scalar /
/// real matrix / complex matrix" — a combinatorial explosion with every operation written four
/// times. Once unified, only
/// the two states "scalar / matrix" remain, and each operation has a single implementation.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct C {
    pub re: f64,
    pub im: f64,
}

// The method names `add/sub/mul/div/neg` being identical to std::ops is **deliberate**: the
// operators of the formula layer
// should be called exactly that. But they must not become std trait impls — `Div` cannot return a
// `Result`,
// and complex division by zero is an error at the formula layer (see `C::div`). So this clippy
// lint is explicitly waived here.
#[allow(clippy::should_implement_trait)]
impl C {
    pub const ZERO: C = C { re: 0.0, im: 0.0 };
    pub const ONE: C = C { re: 1.0, im: 0.0 };
    /// The imaginary unit `j` (the engineering convention; written i in mathematics).
    pub const I: C = C { re: 0.0, im: 1.0 };

    pub const fn new(re: f64, im: f64) -> Self {
        C { re, im }
    }

    /// Promote a real number to complex (zero imaginary part).
    pub const fn real(re: f64) -> Self {
        C { re, im: 0.0 }
    }

    /// Whether it lies on the real axis (imaginary part exactly 0, including -0.0).
    pub fn is_real(self) -> bool {
        self.im == 0.0
    }

    pub fn add(self, o: C) -> C {
        C::new(self.re + o.re, self.im + o.im)
    }
    pub fn sub(self, o: C) -> C {
        C::new(self.re - o.re, self.im - o.im)
    }
    pub fn neg(self) -> C {
        C::new(-self.re, -self.im)
    }
    pub fn mul(self, o: C) -> C {
        C::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }
    /// Division. Division by zero is an **error** (not a silent NaN): an error the formula layer
    /// can prove should not
    /// become runtime noise; the generated-code lane handles IEEE semantics separately (see the
    /// emitter comments).
    ///
    /// Both fast paths are **mandatory**, not optimizations:
    /// 1. Real divided by real goes straight to `a/b`. Using `a·b/(b²)` would
    ///    overflow to inf when `|a·b|` exceeds 1.8e308, and underflow `b²` to 0 when `|b|` is below
    ///    1.5e-162, thus falsely reporting division by zero
    ///    (`1e300/1e10` would be computed as inf, `1e-200/1e-200` would report "division by
    ///    zero").
    /// 2. Complex division uses Smith's algorithm (scaling first by the magnitudes of `|re|` and
    ///    `|im|`),
    ///    avoiding overflow/underflow of `re²+im²`.
    pub fn div(self, o: C) -> Result<C, Error> {
        if o.is_real() {
            if o.re == 0.0 {
                return Err(Error::DivisionByZero);
            }
            return Ok(C::new(self.re / o.re, self.im / o.re));
        }
        if o.re.abs() >= o.im.abs() {
            let r = o.im / o.re;
            let den = o.re + o.im * r;
            if den == 0.0 {
                return Err(Error::DivisionByZero);
            }
            Ok(C::new(
                (self.re + self.im * r) / den,
                (self.im - self.re * r) / den,
            ))
        } else {
            let r = o.re / o.im;
            let den = o.re * r + o.im;
            if den == 0.0 {
                return Err(Error::DivisionByZero);
            }
            Ok(C::new(
                (self.re * r + self.im) / den,
                (self.im * r - self.re) / den,
            ))
        }
    }

    /// Modulus `|z|`.
    pub fn abs(self) -> f64 {
        math::hypot(self.re, self.im)
    }
    /// Argument (principal value, (-π, π]).
    pub fn arg(self) -> f64 {
        math::atan2(self.im, self.re)
    }
    pub fn conj(self) -> C {
        C::new(self.re, -self.im)
    }
    /// Exponential. Reals take a fast path: the polar formula, when the real part overflows,
    /// yields `(inf·cos0, inf·sin0)` =
    /// `(inf, NaN)` — "the exponential of a real number" should not grow a NaN imaginary part.
    pub fn exp(self) -> C {
        if self.is_real() {
            return C::real(math::exp(self.re));
        }
        let m = math::exp(self.re);
        C::new(m * math::cos(self.im), m * math::sin(self.im))
    }
    /// Principal-value logarithm (negative real → imaginary part π).
    pub fn ln(self) -> Result<C, Error> {
        if self.re == 0.0 && self.im == 0.0 {
            return Err(Error::LogOfZero);
        }
        Ok(C::new(math::ln(self.abs()), self.arg()))
    }

    /// Real-exponent power (polar formula). `√s` / `s^(-1/2)` / `(jω)^(1/2)` all share this one.
    ///
    /// A real base is triaged first; never blindly go polar: `atan2(0, -1.3) = π` and
    /// `sin(2π) = -2.4e-16`, so `(-1.3)^2` would pick up a trace of a fake imaginary part,
    /// and the "must be real" gate of `eval` would misjudge a purely real formula as complex.
    pub fn pow_real(self, p: f64) -> C {
        // A non-finite exponent has no "real axis" to speak of: return a NaN real part directly, so
        // the polar formula does not
        // produce a pseudo-complex number with im=NaN that falsely trips the real gate of `eval`.
        if !p.is_finite() {
            return C::real(f64::NAN);
        }
        if self.is_real() {
            // Integer exponent: use powi (exact even for a negative base, and `powf` on a negative
            // base + integer exponent
            // depends on the platform pow's C99 behaviour, which is not worth betting on)
            if p == math::trunc(p) && p.abs() <= i32::MAX as f64 {
                return C::real(math::powi(self.re, p as i32));
            }
            if self.re >= 0.0 {
                return C::real(math::powf(self.re, p));
            }
        }
        let (r, t) = (self.abs(), self.arg());
        let m = math::powf(r, p);
        let a = t * p;
        C::new(m * math::cos(a), m * math::sin(a))
    }

    /// Complex-exponent power: `z^w = exp(w·ln z)` (principal branch).
    pub fn pow(self, o: C) -> Result<C, Error> {
        if o.is_real() {
            return Ok(self.pow_real(o.re));
        }
        Ok(o.mul(self.ln()?).exp())
    }

    /// Principal-value square root: an **algebraic closed form**, not polar (skipping the three
    /// transcendental functions atan2/cos/sin).
    ///
    /// Closed form of `√(x+iy)`: `r = hypot(x,y)`, `re = √((r+x)/2)`,
    /// `im = sign(y)·√((r-x)/2)`. This form is also numerically more stable (r+x does not cancel
    /// catastrophically),
    /// and it agrees pointwise with the polar form — guarded by the `sqrt² == itself` test.
    pub fn sqrt(self) -> C {
        if self.is_real() {
            // Real: non-negative goes to sqrt, negative gives `0 + √|x|·j` (principal value)
            return if self.re >= 0.0 {
                C::real(math::sqrt(self.re))
            } else {
                C::new(0.0, math::sqrt(-self.re))
            };
        }
        let r = self.abs();
        let re = math::sqrt(((r + self.re) * 0.5).max(0.0));
        let im = math::sqrt(((r - self.re) * 0.5).max(0.0)).copysign(self.im);
        C::new(re, im)
    }

    pub fn sin(self) -> C {
        C::new(
            math::sin(self.re) * math::cosh(self.im),
            math::cos(self.re) * math::sinh(self.im),
        )
    }
    pub fn cos(self) -> C {
        C::new(
            math::cos(self.re) * math::cosh(self.im),
            -math::sin(self.re) * math::sinh(self.im),
        )
    }
    pub fn sinh(self) -> C {
        C::new(
            math::sinh(self.re) * math::cos(self.im),
            math::cosh(self.re) * math::sin(self.im),
        )
    }
    pub fn cosh(self) -> C {
        C::new(
            math::cosh(self.re) * math::cos(self.im),
            math::sinh(self.re) * math::sin(self.im),
        )
    }
    pub fn tan(self) -> Result<C, Error> {
        self.sin().div(self.cos())
    }
    pub fn tanh(self) -> Result<C, Error> {
        self.sinh().div(self.cosh())
    }
}

/// Evaluation result: scalar or matrix. Matrix elements are `C`, so a real matrix and a complex
/// matrix are the **same**
/// variant (the imaginary part of a real number is always 0).
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Scalar(C),
    Matrix(Vec<Vec<C>>),
}

impl Value {
    /// Scalar view; returns None for a matrix.
    pub fn as_scalar(&self) -> Option<C> {
        match self {
            Value::Scalar(c) => Some(*c),
            Value::Matrix(_) => None,
        }
    }

    /// Whether it really carries an imaginary part (for diagnostics).
    pub fn is_complex(&self) -> bool {
        match self {
            Value::Scalar(c) => !c.is_real(),
            Value::Matrix(m) => m.iter().flatten().any(|c| !c.is_real()),
        }
    }
}

// ---------------------------------------------------------------- matrix algebra (shape arithmetic + evaluation + multiplication generation)

/// Static shape: (rows, cols) of a concrete matrix literal. Pattern matrices/symbols return None
/// (dynamic shape;
/// the first version does not infer shapes); a **concrete matrix with inconsistent shape**
/// (jagged rows) is a hard error.
pub fn shape_of(e: &Expr) -> Result<Option<(usize, usize)>, Error> {
    Ok(match e {
        Expr::Mat(rows) => {
            let cols = rows.first().map(|r| r.len());
            if let Some(n) = cols {
                if rows.iter().any(|r| r.len() != n) {
                    return Err(Error::ShapeMismatch("matrix literal rows have different lengths (jagged matrices are not supported)".into()));
                }
            }
            Some((rows.len(), cols.unwrap_or(0)))
        }
        Expr::MatPattern(_) => None, // dynamic: n is a runtime parameter
        Expr::Transpose(a) => shape_of(a)?.map(|(m, n)| (n, m)),
        _ => None, // scalar / symbol: does not participate in shape arithmetic
    })
}

/// Shape consistency check for matrix operations (operations between literals; symbols return Ok
/// and note that they were not checked).
/// This and the dimension check are **two orthogonal axes**: dimensions govern "physical meaning",
/// shapes govern "algebraic legality".
/// Matrix shape: ask the static `shape_of` first, and if that cannot answer, `flatten` (which
/// recognizes solve/roots/
/// poly_coeffs/products/transpose). Only when neither level can decide is the shape "unknown".
fn matrix_shape(e: &Expr) -> Result<Option<(usize, usize)>, Error> {
    if let Some(s) = shape_of(e)? {
        return Ok(Some(s));
    }
    Ok(flatten_matrix_value(e).and_then(|m| {
        let rows = m.len();
        let cols = m.first().map(|r| r.len())?;
        if m.iter().all(|r| r.len() == cols) {
            Some((rows, cols))
        } else {
            None
        }
    }))
}

pub fn shape_check(e: &Expr) -> Result<(), Error> {
    match e {
        Expr::Add(a, b) | Expr::Sub(a, b) => {
            shape_check(a)?;
            shape_check(b)?;
            // Matrix-ness uses the **precise** predicate `is_matrix_value` (it goes through
            // flatten and recognizes products/transposes/solution vectors);
            // `shape_of` is used only to compare shapes, and returns None for matrix expressions
            // "whose shape is unknown at compile time".
            let (ma, mb) = (is_matrix_value(a), is_matrix_value(b));
            // Matrices and scalars cannot be added/subtracted: `3 + roots(...)` would be emitted
            // as `3.0 + vec![vec![…]]` (E0277), while the diagnostics used to say "clean".
            // Exception for the zero element (the additive identity, usually already gone after
            // simplification).
            let is_zero = |x: &Expr| matches!(simplify(x), Expr::Num(v) if v == 0.0);
            if ma != mb {
                let scalar_side = if ma { b } else { a };
                if !is_zero(scalar_side) {
                    return Err(Error::MatrixScalarMix(
                        "matrix vs scalar add/sub: operate element-wise or reshape".to_string(),
                    ));
                }
            } else if ma && mb {
                // Both sides are matrices: ask `shape_of` for the shape first, then `flatten`
                // (the shape of `roots(...) + roots(...)` exists only after expansion)
                let (sa, sb) = (matrix_shape(a)?, matrix_shape(b)?);
                if let (Some(sa), Some(sb)) = (sa, sb) {
                    if sa != sb {
                        return Err(Error::ShapeMismatch(format!(
                            "matrix add/sub shapes differ: {sa:?} vs {sb:?}"
                        )));
                    }
                }
            }
            Ok(())
        }
        Expr::Mul(a, b) => {
            shape_check(a)?;
            shape_check(b)?;
            // Shape determination must be two-level: `shape_of` only recognizes static shapes
            // (matrix literals/pattern matrices),
            // while things like `solve/roots/poly_coeffs` are only known **after expansion** —
            // checking only the former misses the case
            // "neither matrix can be determined → emit as scalars", and the output becomes
            // `vec![…] * vec![…]`.
            let sa = matrix_shape(a)?;
            let sb = matrix_shape(b)?;
            match (sa, sb) {
                (Some(sa), Some(sb)) => {
                    if sa.1 != sb.0 {
                        return Err(Error::ShapeMismatch(format!(
                "cannot multiply {sa:?} by {sb:?}: columns of the left operand ≠ rows of the right"
            )));
                    }
                }
                // One side is a matrix, the other's shape is unknown → undecidable, reject
                // explicitly (no guessing)
                (Some(_), None) if is_matrix_value(b) => {
                    return Err(Error::ShapeMismatch(
                "one side of the product has a shape unknown at compile time, so it cannot be checked".into(),
            ))
                }
                (None, Some(_)) if is_matrix_value(a) => {
                    return Err(Error::ShapeMismatch(
                "one side of the product has a shape unknown at compile time, so it cannot be checked".into(),
            ))
                }
                _ => {}
            }
            Ok(())
        }
        Expr::Div(a, b) => {
            shape_check(a)?;
            // Matrix division (inversion) is unsupported in the first version: a matrix literal as
            // the right operand = explicit error
            if shape_of(b)?.is_some() {
                return Err(Error::MatrixDivisionUnsupported(
                    "write an explicit inverse (`inv(A) * b`) or use the solver".to_string(),
                ));
            }
            Ok(())
        }
        Expr::Neg(a) | Expr::Transpose(a) => shape_check(a),
        Expr::Deriv(u, _) | Expr::Grad(u) | Expr::Laplacian(u) => shape_check(u),
        Expr::Sum {
            lower, upper, body, ..
        }
        | Expr::Integral {
            lower, upper, body, ..
        } => {
            shape_check(lower)?;
            shape_check(upper)?;
            shape_check(body)
        }
        Expr::Indexed { .. } => Ok(()), // an indexed symbol's shape depends on its declaration; the first version lets it through
        Expr::Pow(a, b) => {
            shape_check(a)?;
            shape_check(b)
        }
        Expr::Call(_, args) => {
            for a in args {
                shape_check(a)?;
            }
            Ok(())
        }
        Expr::Mat(rows) => {
            shape_of(e)?;
            for r in rows {
                for c in r {
                    shape_check(c)?;
                }
            }
            Ok(())
        }
        Expr::MatPattern(p) => {
            p.grid().ok_or("模式矩阵各行长度不一致")?;
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Structured evaluation: both scalars (real/complex) and matrices can be computed.
/// `eval` is its "must be a real scalar" convenience wrapper.
pub fn eval_value(e: &Expr, env: &HashMap<String, f64>) -> Result<Value, Error> {
    use Expr as E;
    Ok(match e {
        E::Num(v) => Value::Scalar(C::real(*v)),
        // Reserved symbol `j`: the imaginary unit (dimensionless); if `j` is given explicitly in
        // env, use env's value
        E::Var(n) if n == "j" && !env.contains_key("j") => Value::Scalar(C::I),
        E::Var(n) => Value::Scalar(C::real(
            *env.get(n).ok_or_else(|| format!("求值时缺少变量 `{n}`"))?,
        )),
        E::Neg(a) => match eval_value(a, env)? {
            Value::Scalar(c) => Value::Scalar(c.neg()),
            Value::Matrix(m) => Value::Matrix(map_mat(&m, |c| c.neg())),
        },
        E::Add(a, b) => c_bin(a, b, env, |x, y| Ok(x.add(y)))?,
        E::Sub(a, b) => c_bin(a, b, env, |x, y| Ok(x.sub(y)))?,
        E::Div(a, b) => {
            // **Matrix divided by scalar** is element-wise, consistent with the emission lane (the
            // Div branch of `flatten_matrix_value`
            // divides cell by cell); matrix divided by matrix is still rejected — that is exactly
            // the confusing point of "inversion"
            // semantics (`*` is the matrix product, so if `/` also did matrix algebra it would
            // become a solver).
            let (va, vb) = (eval_value(a, env)?, eval_value(b, env)?);
            match (va, vb) {
                (Value::Matrix(m), Value::Scalar(k)) => {
                    let mut rows = Vec::with_capacity(m.len());
                    for r in m {
                        let mut out = Vec::with_capacity(r.len());
                        for c in r {
                            out.push(c.div(k)?);
                        }
                        rows.push(out);
                    }
                    Value::Matrix(rows)
                }
                (Value::Matrix(_), Value::Matrix(_)) => {
                    return Err(Error::MatrixDivisionUnsupported("write the explicit inverse (`inv(A) * b`) or use a solver".into()))
                }
                (Value::Scalar(_), Value::Matrix(_)) => {
                    return Err(Error::MatrixScalarMix("a scalar divided by a matrix is undefined; write matrix equations as `inv(A) * b`".into()))
                }
                (x, y) => {
                    let (Value::Scalar(x), Value::Scalar(y)) = (x, y) else {
                        unreachable!("上面已排除所有含矩阵的情形")
                    };
                    Value::Scalar(x.div(y)?)
                }
            }
        }
        E::Pow(a, b) => {
            // Non-negative integer matrix powers (0..=8, power 0 = identity): the same folding
            // semantics as the generator.
            let base_is_matrix =
                matches!(a.as_ref(), Expr::Mat(_) | Expr::MatPattern(_)) || is_matrix_value(a);
            if base_is_matrix {
                let Expr::Num(k) = b.as_ref() else {
                    return Err(Error::InvalidInput(
                        "the exponent of a matrix power must be an integer constant".into(),
                    ));
                };
                if !(*k >= 0.0 && math::fract(*k) == 0.0 && *k <= 8.0) {
                    return Err(Error::InvalidInput(format!(
                        "matrix powers support non-negative integers 0–8, got {k}; write the explicit product or use a numeric library"
                    )));
                }
                let mut acc = eval_value(a, env)?;
                if *k == 0.0 {
                    let Value::Matrix(m) = acc else {
                        return Err(Error::ShapeMismatch(
                            "the base of a zero matrix power must be a square matrix".into(),
                        ));
                    };
                    if m.iter().any(|r| r.len() != m.len()) {
                        return Err(Error::ShapeMismatch(
                            "the base of a zero matrix power must be a square matrix".into(),
                        ));
                    }
                    return Ok(Value::Matrix(
                        (0..m.len())
                            .map(|i| {
                                (0..m.len())
                                    .map(|j| C::real(if i == j { 1.0 } else { 0.0 }))
                                    .collect()
                            })
                            .collect(),
                    ));
                }
                for _ in 1..(*k as usize) {
                    acc = mat_mul_value_of(acc, eval_value(a, env)?)?;
                }
                return Ok(acc);
            }
            // The exponent side uses the **precise predicate** (same shape as the base side):
            // `matrix_ish` is an over-approximation
            // that would misjudge exponents like `2 ^ det(J)` / `x ^ norm(v)` — "scalar value whose
            // parameters involve a matrix" — as matrices
            // (the emission lane gets the same expressions right).
            let expo_is_matrix =
                matches!(b.as_ref(), Expr::Mat(_) | Expr::MatPattern(_)) || is_matrix_value(b);
            if expo_is_matrix {
                return Err(Error::MatrixScalarMix(
                    "a matrix cannot be an exponent".into(),
                ));
            }
            c_bin(a, b, env, |x, y| x.pow(y))?
        }
        E::Mul(a, b) => {
            let (va, vb) = (eval_value(a, env)?, eval_value(b, env)?);
            mat_mul_value_of(va, vb)?
        }
        E::Call(name, args) => {
            let Some(arg) = args.first() else {
                return Err(Error::FunctionArity {
                    name: name.clone(),
                    expected: 1,
                    got: 0,
                });
            };
            if matches!(name.as_str(), "__quad_root_1" | "__quad_root_2") {
                if args.len() != 3 {
                    return Err(Error::FunctionArity {
                name: name.clone(),
                expected: 3,
                got: args.len(),
            });
                }
                let a = as_c(&eval_value(&args[0], env)?, "the quadratic coefficient")?;
                let b = as_c(&eval_value(&args[1], env)?, "the linear coefficient")?;
                let c = as_c(&eval_value(&args[2], env)?, "the constant term")?;
                return Ok(Value::Scalar(stable_quadratic_root(
                    name == "__quad_root_1",
                    a,
                    b,
                    c,
                )?));
            }
            // Dot product/norm: expand into an explicit expression, then evaluate (the same
            // strategy as det/trace, only one copy of the math)
            if name == "dot" || name == "norm" {
                let expanded = expand_call(e).ok_or_else(|| {
                    format!("`{name}` 需要具体矩阵字面量（形状编译期未知时无法展开）")
                })?;
                return eval_value(&expanded, env);
            }
            // Linear solving / polynomial root finding: expand into an explicit expression first,
            // then evaluate (no second solver implementation)
            if name == "solve" || name == "roots" || name == "poly_coeffs" {
                let expanded = expand_call(e).ok_or_else(|| {
                    "`solve` 的条件不满足（第二个参数必须是变量名，且方程对它线性）".to_string()
                })?;
                let value = eval_value(&expanded, env)?;
                if name == "roots" && !uses_complex(e) && value.is_complex() {
                    return Err(Error::UnsupportedOperation(
                        "the quadratic has no real roots; write the complex lane explicitly with `j`"
                            .into(),
                    ));
                }
                return Ok(value);
            }
            // Matrix operators: det / trace / conjugate transpose / inverse
            if matches!(name.as_str(), "det" | "trace" | "adjoint" | "inv") {
                let v = eval_value(arg, env)?;
                let m = match v {
                    Value::Matrix(m) => m,
                    Value::Scalar(_) => {
                        return Err(Error::MatrixScalarMix(format!(
                            "`{name}` needs a matrix argument, got a scalar"
                        )))
                    }
                };
                return match name.as_str() {
                    "trace" => {
                        if m.len() != m.first().map(|r| r.len()).unwrap_or(0) {
                            return Err(Error::ShapeMismatch(
                                "`trace` needs a square matrix".into(),
                            ));
                        }
                        let mut acc = C::ZERO;
                        for (i, row) in m.iter().enumerate() {
                            acc = acc.add(row[i]);
                        }
                        Ok(Value::Scalar(acc))
                    }
                    "det" => Ok(Value::Scalar(mat_det(&m)?)),
                    // inv goes through IR expansion (the same copy of the math as the emission
                    // lane: orders 2 and 3 are both supported).
                    // The value layer once had only a 2×2 closed form written, and `himari-eval`
                    // failed outright on its own order-3
                    // fixture — the two lanes must share the same implementation.
                    "inv" => {
                        let expanded = expand_matrix_call(e).ok_or_else(|| {
                            "`inv` 只支持 1–3 阶方阵（更高阶请走数值库）".to_string()
                        })?;
                        eval_value(&expanded, env)
                    }
                    _ => {
                        // adjoint = conjugate transpose (a plain transpose over the reals)
                        let cols = m.first().map(|r| r.len()).unwrap_or(0);
                        let mut t = vec![vec![C::ZERO; m.len()]; cols];
                        for (i, row) in m.iter().enumerate() {
                            for (j, c) in row.iter().enumerate() {
                                t[j][i] = c.conj();
                            }
                        }
                        Ok(Value::Matrix(t))
                    }
                };
            }
            if args.len() > 1 {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(as_c(
                        &eval_value(a, env)?,
                        &format!("the argument of `{name}`"),
                    )?);
                }
                return Ok(Value::Scalar(c_call_n(name, &vals)?));
            }
            let c = as_c(&eval_value(arg, env)?, &format!("the argument of `{name}`"))?;
            Value::Scalar(c_call(name, c)?)
        }
        E::Mat(rows) => {
            let mut out = Vec::with_capacity(rows.len());
            for r in rows {
                let mut row = Vec::with_capacity(r.len());
                for cell in r {
                    row.push(as_c(&eval_value(cell, env)?, "a matrix element")?);
                }
                out.push(row);
            }
            Value::Matrix(out)
        }
        E::Transpose(a) => match eval_value(a, env)? {
            Value::Matrix(m) => {
                if m.is_empty() {
                    return Ok(Value::Matrix(vec![]));
                }
                let (r, c) = (m.len(), m[0].len());
                let mut t = vec![vec![C::ZERO; r]; c];
                for i in 0..r {
                    for j in 0..c {
                        t[j][i] = m[i][j];
                    }
                }
                Value::Matrix(t)
            }
            Value::Scalar(_) => {
                return Err(Error::MatrixScalarMix(
                    "a scalar cannot be transposed".into(),
                ))
            }
        },
        E::Sum {
            var,
            lower,
            upper,
            body,
        } => {
            let lo = eval_i(lower, env)?;
            let hi = eval_i(upper, env)?;
            if hi >= lo && (hi - lo) as u64 > MAX_STEPS {
                return Err(Error::InvalidInput(format!(
                "the sum would take {} steps (limit {MAX_STEPS}); sums are explicit teaching loops, not a numeric library",
                hi - lo + 1
            )));
            }
            let mut acc = C::ZERO;
            for k in lo..=hi {
                let mut inner = env.clone();
                inner.insert(var.clone(), k as f64);
                acc = acc.add(as_c(&eval_value(body, &inner)?, "the summand")?);
            }
            Value::Scalar(acc)
        }
        E::Integral {
            var,
            lower,
            upper,
            body,
        } => {
            let lo = eval(lower, env)?;
            let hi = eval(upper, env)?;
            // Composite Simpson (128 panels, weight 4 at odd indices): teaching accuracy, no
            // attempt at golub-welsch
            let n = 128usize;
            let h = (hi - lo) / n as f64;
            let f = |x: f64| -> Result<C, Error> {
                let mut inner = env.clone();
                inner.insert(var.clone(), x);
                as_c(&eval_value(body, &inner)?, "the integrand")
            };
            let mut acc = f(lo)?.add(f(hi)?);
            for k in 1..n {
                let x = lo + h * k as f64;
                let w = if k % 2 == 1 { 4.0 } else { 2.0 };
                acc = acc.add(f(x)?.mul(C::real(w)));
            }
            Value::Scalar(acc.mul(C::real(h / 3.0)))
        }
        E::Deriv(..) => return Err(Error::UnsupportedOperation(
            "`∂` is a symbolic operator: differentiate symbolically first, or use a finite difference".into(),
        )),
        E::Grad(_) | E::Laplacian(_) => {
            return Err(
                ("∇/∇² 是符号算子：数值求值需要离散化（走生成代码车道）".to_string()).into(),
            )
        }
        E::Indexed { .. } => {
            return Err(Error::UnsupportedOperation(
            "evaluating an indexed symbol needs an index environment, which this version does not provide".into(),
        ))
        }
        E::MatPattern(_) => {
            return Err(
                ("求值不支持模式矩阵（先 recognize_pattern 再走生成代码）".to_string()).into(),
            )
        }
    })
}

fn map_mat(m: &[Vec<C>], f: impl Fn(C) -> C) -> Vec<Vec<C>> {
    m.iter()
        .map(|r| r.iter().map(|c| f(*c)).collect())
        .collect()
}

/// Take the scalar view, otherwise give a **contextual** error (the matrix lane and the scalar
/// lane are never mixed).
fn as_c(v: &Value, what: &str) -> Result<C, Error> {
    v.as_scalar().ok_or_else(|| {
        Error::MatrixScalarMix(format!(
                "{what} must be a scalar (write matrices element by element; there is no implicit broadcast)"
            ))
    })
}

/// Function registry: name → required arity.
///
/// **Single source of truth**: post-parse arity checking, evaluation, dimensions, emission and
/// write-back all consult this one table.
/// Previously each lane had its own match, so a defect such as "`arg` is emitted as a method that
/// does not exist in the real lane"
/// could survive a whole version.
pub fn func_arity(name: &str) -> Option<usize> {
    match name {
        // Unary scalar
        "exp" | "ln" | "log" | "sqrt" | "sin" | "cos" | "tan" | "sinh" | "cosh" | "tanh"
        | "abs" | "arg" | "conj" | "re" | "im" => Some(1),
        // Binary / ternary scalar (clamping and min/max on the game side)
        "min" | "max" => Some(2),
        "clamp" => Some(3),
        // Linear solving: solve(expr, x) solves `expr = 0` into `x = -b/a`
        "solve" => Some(2),
        "roots" => Some(2),
        "poly_coeffs" => Some(2),
        // Unary matrix operators
        "det" | "trace" | "adjoint" | "inv" | "norm" => Some(1),
        // Binary matrix operators (dot product: two equal-length sequences → a scalar)
        "dot" => Some(2),
        _ => None,
    }
}

/// Edit distance (used only to find "which function you probably meant to write").
fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        core::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// The function name in the registry closest to `name` (edit distance ≤ 2).
/// The **single list** of builtin functions (the truth source for `func_arity`). Used by the
/// "registry consistency" test:
/// adding an operator means changing only this and `func_arity`; a lane that forgets to register
/// it gets caught by the test.
pub const FUNC_NAMES: &[&str] = &[
    "exp",
    "ln",
    "log",
    "sqrt",
    "sin",
    "cos",
    "tan",
    "sinh",
    "cosh",
    "tanh",
    "abs",
    "arg",
    "conj",
    "re",
    "im",
    "min",
    "max",
    "clamp",
    "det",
    "trace",
    "adjoint",
    "inv",
    "solve",
    "roots",
    "poly_coeffs",
    "dot",
    "norm",
];

pub fn closest_func(name: &str) -> Option<&'static str> {
    const NAMES: &[&str] = &[
        "exp",
        "ln",
        "log",
        "sqrt",
        "sin",
        "cos",
        "tan",
        "sinh",
        "cosh",
        "tanh",
        "abs",
        "arg",
        "conj",
        "re",
        "im",
        "min",
        "max",
        "clamp",
        "det",
        "trace",
        "adjoint",
        "inv",
        "solve",
        "roots",
        "poly_coeffs",
        "dot",
        "norm",
    ];
    // A flat "distance <= 2" admits nonsense for short names (`foo` → `log`, `x` → `exp`),
    // so short typos must be within one edit; longer names get two.
    let max_distance = if name.len() <= 4 { 1 } else { 2 };
    NAMES
        .iter()
        .map(|n| (*n, edit_distance(name, n)))
        .filter(|(_, d)| *d <= max_distance)
        .min_by_key(|(_, d)| *d)
        .map(|(n, _)| n)
}

/// Arity checking of all function calls in an expression (called once at the end of the front-end
/// parse).
pub fn check_calls(e: &Expr) -> Result<(), Error> {
    check_calls_with_shapes(e, &runtime::RuntimeShapes::new())
}

/// Validate calls with declaration ranks available, allowing runtime solve(A,b)
/// without weakening the existing symbolic solve checks.
pub fn check_calls_with_shapes(
    e: &Expr,
    runtime_shapes: &runtime::RuntimeShapes,
) -> Result<(), Error> {
    match e {
        Expr::Call(name, args) => {
            let Some(want) = func_arity(name) else {
                // The payload is the bare name; the spelling suggestion is the LSP quick-fix's
                // job (`closest_func` there), so the catalogue sentence stays translatable.
                // `suggestion` stays EMPTY unless a real near-match exists — echoing the
                // user's own typo back as advice is worse than no advice.
                let suggestion = closest_func(name).unwrap_or_default().to_string();
                return Err(Error::UnknownFunction {
                    name: name.clone(),
                    suggestion,
                });
            };
            if args.len() != want {
                return Err(Error::FunctionArity {
                    name: name.clone(),
                    expected: want,
                    got: args.len(),
                });
            }
            // poly_coeffs(poly, x): the same precondition as roots (a univariate polynomial in x)
            if name == "poly_coeffs" {
                let Expr::Var(v) = &args[1] else {
                    return Err(Error::InvalidInput(
                        "`poly_coeffs(poly, x)`: the second argument must be a variable name"
                            .into(),
                    ));
                };
                poly_coeffs_vec(&args[0], v)?;
            }
            // roots(poly, x): the second argument must be a variable name; the degree and
            // non-polynomial shapes are rejected right here
            if name == "roots" {
                let Expr::Var(v) = &args[1] else {
                    return Err(Error::InvalidInput(
                        "`roots(poly, x)`: the second argument must be a variable name".into(),
                    ));
                };
                solve_poly(&args[0], v)?;
            }
            // solve: both forms — single-variable `solve(eq, x)` and the system
            // `solve([eq…], [x…])` —
            // are solved once right at the parse exit (fail fast) — letting
            // non-linearity/underdetermination/contradiction blow up only at evaluation or
            // emission time amounts to pushing the error further away from the user.
            if name == "solve" {
                match &args[1] {
                    Expr::Var(v) => {
                        let is_runtime_solve = runtime::runtime_kind_of(&args[0], runtime_shapes)
                            .map(|kind| kind == runtime::RuntimeKind::Matrix)
                            .unwrap_or(false)
                            && runtime::runtime_kind_of(&args[1], runtime_shapes)
                                .map(|kind| kind == runtime::RuntimeKind::Vector)
                                .unwrap_or(false);
                        if !is_runtime_solve {
                            solve_linear(&args[0], v)?;
                        }
                    }
                    Expr::Mat(_) => {
                        let vars = mat_var_names(&args[1]).ok_or_else(|| {
                            "`solve([方程…], [未知数…])` 的第二个参数必须是变量名列表".to_string()
                        })?;
                        let eqs = flat_mat_items(&args[0]).ok_or_else(|| {
                            "方程组形态：`solve` 的第一个参数必须是方程列表 \
                             `[eq1, eq2, …]`（每条写成「式 = 0」）"
                                .to_string()
                        })?;
                        solve_system(&eqs, &vars)?;
                    }
                    _ => {
                        return Err(Error::InvalidInput(
            "`solve(expr, x)`: the second argument must be a variable name or a list of names".into(),
        ))
                    }
                }
            }
            for a in args {
                check_calls_with_shapes(a, runtime_shapes)?;
            }
            Ok(())
        }
        Expr::Num(_) | Expr::Var(_) | Expr::Indexed { .. } => Ok(()),
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
            check_calls_with_shapes(a, runtime_shapes)?;
            check_calls(b)
        }
        Expr::Neg(a)
        | Expr::Transpose(a)
        | Expr::Grad(a)
        | Expr::Laplacian(a)
        | Expr::Deriv(a, _) => check_calls_with_shapes(a, runtime_shapes),
        Expr::Mat(rows) => {
            for r in rows {
                for c in r {
                    check_calls_with_shapes(c, runtime_shapes)?;
                }
            }
            Ok(())
        }
        Expr::MatPattern(p) => {
            for e in p.elements() {
                check_calls_with_shapes(e, runtime_shapes)?;
            }
            Ok(())
        }
        Expr::Sum {
            lower, upper, body, ..
        }
        | Expr::Integral {
            lower, upper, body, ..
        } => {
            check_calls_with_shapes(lower, runtime_shapes)?;
            check_calls_with_shapes(upper, runtime_shapes)?;
            check_calls_with_shapes(body, runtime_shapes)
        }
    }
}

/// Scalar function registry — reals and complex numbers share one table (a real number is just a
/// complex number with zero imaginary part).
/// Unregistered names always error: silently falling back to 0 would let wrong formulas pass
/// through sick.
/// Evaluation of multi-argument/matrix operators (kept apart from the unary scalar table, so
/// min/max are not squeezed into the scalar match).
fn c_call_n(name: &str, args: &[C]) -> Result<C, Error> {
    let real = |v: C| -> Result<f64, Error> {
        if v.is_real() {
            Ok(v.re)
        } else {
            Err(Error::InvalidInput(format!(
                "`{name}` is defined for real numbers only (got {v:?})"
            )))
        }
    };
    Ok(match name {
        "min" => C::real(real(args[0])?.min(real(args[1])?)),
        "max" => C::real(real(args[0])?.max(real(args[1])?)),
        // clamp(v, lo, hi) = v.max(lo).min(hi) (the same shape as in Rust).
        // Note that with lo > hi the result is **hi** (max first, then min); no "swap the bounds"
        // guess is made —
        // guessing the bounds would silently turn a miswritten call into a clamp that "looks
        // fine".
        "clamp" => C::real(real(args[0])?.max(real(args[1])?).min(real(args[2])?)),
        other => {
            return Err(Error::InvalidInput(format!(
                "`{other}` is not a scalar multi-argument function"
            )))
        }
    })
}

fn c_call(name: &str, x: C) -> Result<C, Error> {
    Ok(match name {
        "exp" => x.exp(),
        "ln" | "log" => x.ln()?,
        "sqrt" => x.sqrt(),
        "sin" => x.sin(),
        "cos" => x.cos(),
        "tan" => x.tan()?,
        "sinh" => x.sinh(),
        "cosh" => x.cosh(),
        "tanh" => x.tanh()?,
        // Four complex-specific ones: the result always lies on the real axis
        "abs" => C::real(x.abs()),
        "arg" => C::real(x.arg()),
        "conj" => x.conj(),
        "re" => C::real(x.re),
        "im" => C::real(x.im),
        other => {
            return Err(Error::UnsupportedOperation(format!(
                "the function `{other}` is not supported during evaluation"
            )))
        }
    })
}

/// Determinant (complex-aware): recursive cofactor expansion along the first row, up to order 4.
///
/// The same algorithm as the IR-level `det_of` — if the two implementations disagreed, `det`'s
/// evaluation and the generated code
/// would give different numbers (that kind of defect is the hardest to find). The tests compare
/// numerically for orders 2/3/4.
fn mat_det(m: &[Vec<C>]) -> Result<C, Error> {
    let n = m.len();
    if n == 0 || m.iter().any(|r| r.len() != n) {
        return Err(Error::ShapeMismatch(
            "`det` needs a non-empty square matrix".into(),
        ));
    }
    if n > 4 {
        return Err(Error::InvalidInput(format!(
            "`det` supports orders 1–4 only (got {n}); use a numeric library for higher orders"
        )));
    }
    if n == 1 {
        return Ok(m[0][0]);
    }
    let mut acc = C::ZERO;
    for (j, cell) in m[0].iter().enumerate() {
        let sub: Vec<Vec<C>> = m
            .iter()
            .skip(1)
            .map(|row| {
                row.iter()
                    .enumerate()
                    .filter(|(c, _)| *c != j)
                    .map(|(_, x)| *x)
                    .collect()
            })
            .collect();
        let term = cell.mul(mat_det(&sub)?);
        acc = if j % 2 == 0 {
            acc.add(term)
        } else {
            acc.sub(term)
        };
    }
    Ok(acc)
}

/// Scalar binary operations (complex-aware) + element-wise matrix operations (shapes must match
/// exactly).
fn c_bin(
    a: &Expr,
    b: &Expr,
    env: &HashMap<String, f64>,
    f: impl Fn(C, C) -> Result<C, Error>,
) -> Result<Value, Error> {
    let (va, vb) = (eval_value(a, env)?, eval_value(b, env)?);
    match (va, vb) {
        (Value::Scalar(x), Value::Scalar(y)) => Ok(Value::Scalar(f(x, y)?)),
        (Value::Matrix(ma), Value::Matrix(mb)) => {
            if ma.len() != mb.len() {
                return Err(
                    (format!("矩阵逐元素运算行数不一致：{} vs {}", ma.len(), mb.len())).into(),
                );
            }
            let mut out = Vec::with_capacity(ma.len());
            for (ra, rb) in ma.iter().zip(mb.iter()) {
                if ra.len() != rb.len() {
                    return Err(Error::ShapeMismatch(format!(
                        "element-wise operation on matrices with different column counts: {} vs {}",
                        ra.len(),
                        rb.len()
                    )));
                }
                let mut row = Vec::with_capacity(ra.len());
                for (x, y) in ra.iter().zip(rb.iter()) {
                    row.push(f(*x, *y)?);
                }
                out.push(row);
            }
            Ok(Value::Matrix(out))
        }
        _ => Err(Error::MatrixScalarMix("write element-wise matrix/scalar operations explicitly (there is no implicit broadcast)".into())),
    }
}

/// Multiplication on evaluated values: matrix×matrix, matrix×scalar, scalar×scalar.
fn mat_mul_value_of(a: Value, b: Value) -> Result<Value, Error> {
    match (a, b) {
        (Value::Scalar(x), Value::Scalar(y)) => Ok(Value::Scalar(x.mul(y))),
        (Value::Matrix(ma), Value::Matrix(mb)) => {
            let n_inner = ma.first().map(|r| r.len()).unwrap_or(0);
            if mb.len() != n_inner {
                return Err(Error::ShapeMismatch(format!(
                    "cannot multiply: inner dimension {n_inner} ≠ rows {}",
                    mb.len()
                )));
            }
            let cols = mb.first().map(|r| r.len()).unwrap_or(0);
            let mut out = vec![vec![C::ZERO; cols]; ma.len()];
            for (i, row) in ma.iter().enumerate() {
                for (j, cell) in out[i].iter_mut().enumerate() {
                    let mut acc = C::ZERO;
                    for (k, v) in row.iter().enumerate() {
                        acc = acc.add(v.mul(mb[k][j]));
                    }
                    *cell = acc;
                }
            }
            Ok(Value::Matrix(out))
        }
        (Value::Matrix(m), Value::Scalar(k)) => Ok(Value::Matrix(map_mat(&m, |v| v.mul(k)))),
        (Value::Scalar(k), Value::Matrix(m)) => Ok(Value::Matrix(map_mat(&m, |v| v.mul(k)))),
    }
}

/// Evaluate summation bounds: must be integer-valued (the summation index is a counter).
fn eval_i(e: &Expr, env: &HashMap<String, f64>) -> Result<i64, Error> {
    let v = eval(e, env)?;
    if math::fract(v) != 0.0 {
        return Err(Error::InvalidInput(format!(
            "the sum bounds must be integers, got {v}"
        )));
    }
    Ok(v as i64)
}

// ---------------------------------------------------------------- symbolic differentiation

/// Whether the expression contains `diff`'s "not symbolically differentiable" marker
/// (`Num(NaN)`).
///
/// This is the last gate before emission/generation: once `NaN` is emitted as a literal,
/// the output contains an illegal `NaN` identifier, and the error points at the generated code.
pub fn contains_nan_expr(e: &Expr) -> bool {
    match e {
        Expr::Num(v) => v.is_nan(),
        Expr::Var(_) | Expr::Indexed { .. } => false,
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
            contains_nan_expr(a) || contains_nan_expr(b)
        }
        Expr::Neg(a)
        | Expr::Transpose(a)
        | Expr::Grad(a)
        | Expr::Laplacian(a)
        | Expr::Deriv(a, _) => contains_nan_expr(a),
        Expr::Call(_, args) => args.iter().any(contains_nan_expr),
        Expr::Mat(rows) => rows.iter().flatten().any(contains_nan_expr),
        Expr::MatPattern(p) => p.elements().into_iter().any(contains_nan_expr),
        Expr::Sum {
            lower, upper, body, ..
        }
        | Expr::Integral {
            lower, upper, body, ..
        } => contains_nan_expr(lower) || contains_nan_expr(upper) || contains_nan_expr(body),
    }
}

/// Whether the expression contains **any** non-finite floating-point constant (inf / NaN).
///
/// `contains_nan_expr` only checks the NaN marker (diff's sentinel); the pre-emission constant
/// check
/// needs "any non-finite counts" (`inf` equally cannot be emitted as a legal literal).
pub fn contains_nonfinite_expr(e: &Expr) -> bool {
    match e {
        Expr::Num(v) => !v.is_finite(),
        Expr::Var(_) | Expr::Indexed { .. } => false,
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
            contains_nonfinite_expr(a) || contains_nonfinite_expr(b)
        }
        Expr::Neg(a)
        | Expr::Transpose(a)
        | Expr::Grad(a)
        | Expr::Laplacian(a)
        | Expr::Deriv(a, _) => contains_nonfinite_expr(a),
        Expr::Call(_, args) => args.iter().any(contains_nonfinite_expr),
        Expr::Mat(rows) => rows.iter().flatten().any(contains_nonfinite_expr),
        Expr::MatPattern(p) => p.elements().into_iter().any(contains_nonfinite_expr),
        Expr::Sum {
            lower, upper, body, ..
        }
        | Expr::Integral {
            lower, upper, body, ..
        } => {
            contains_nonfinite_expr(lower)
                || contains_nonfinite_expr(upper)
                || contains_nonfinite_expr(body)
        }
    }
}

/// Names of functions/operators `diff` cannot symbolically differentiate (for a **named** error,
/// instead of just dropping a NaN and leaving the caller to guess).
pub fn nondiffable(e: &Expr) -> Vec<String> {
    fn go(e: &Expr, out: &mut Vec<String>) {
        match e {
            Expr::Call(n, args) => {
                // det/trace have IR-level expansions (after expansion they are ordinary
                // differentiable expressions),
                // adjoint is matrix-valued (the derivative lane does not take it anyway), so only
                // the following are truly non-differentiable
                // inv/adjoint are matrix-valued operators and never enter a scalar Jacobian (when
                // they appear in a scalar expression
                // they have already been absorbed by the trace/det expansion), so they do not count
                // as "non-differentiable"
                let named = matches!(n.as_str(), "arg" | "min" | "max" | "clamp");
                if named && !out.contains(n) {
                    out.push(n.clone());
                }
                for a in args {
                    go(a, out);
                }
            }
            Expr::Deriv(..) | Expr::Sum { .. } | Expr::Integral { .. } | Expr::Indexed { .. } => {
                if !out.contains(&"算子/求和/积分".to_string()) {
                    out.push("算子/求和/积分".to_string());
                }
            }
            Expr::Mat(rows) => {
                for r in rows {
                    for c in r {
                        go(c, out);
                    }
                }
            }
            Expr::MatPattern(p) => {
                for e in p.elements() {
                    go(e, out);
                }
            }
            Expr::Add(a, b)
            | Expr::Sub(a, b)
            | Expr::Mul(a, b)
            | Expr::Div(a, b)
            | Expr::Pow(a, b) => {
                go(a, out);
                go(b, out);
            }
            Expr::Neg(a) | Expr::Transpose(a) | Expr::Grad(a) | Expr::Laplacian(a) => go(a, out),
            _ => {}
        }
    }
    let mut out = Vec::new();
    go(e, &mut out);
    out
}

pub fn diff(e: &Expr, x: &str) -> Expr {
    match e {
        Expr::Num(_) => num(0.0),
        Expr::Var(n) => num(if n == x { 1.0 } else { 0.0 }),
        Expr::Add(a, b) => add(diff(a, x), diff(b, x)),
        Expr::Sub(a, b) => sub(diff(a, x), diff(b, x)),
        Expr::Mul(a, b) => add(
            mul(diff(a, x), (**b).clone()),
            mul((**a).clone(), diff(b, x)),
        ),
        Expr::Div(a, b) => div(
            sub(
                mul(diff(a, x), (**b).clone()),
                mul((**a).clone(), diff(b, x)),
            ),
            pow((**b).clone(), num(2.0)),
        ),
        Expr::Neg(a) => neg(diff(a, x)),
        Expr::Pow(a, b) => match b.as_ref() {
            Expr::Num(n) => mul(mul(num(*n), pow((**a).clone(), num(n - 1.0))), diff(a, x)),
            _ => mul(
                pow((**a).clone(), (**b).clone()),
                add(
                    mul(diff(b, x), Expr::Call("ln".into(), vec![(**a).clone()])),
                    div(mul((**b).clone(), diff(a, x)), (**a).clone()),
                ),
            ),
        },
        Expr::Call(name, args) => {
            let Some(u) = args.first().cloned() else {
                return num(f64::NAN);
            };
            let du = diff(&u, x);
            let one = |v: Expr| Expr::Call(name.clone(), vec![v]);
            match name.as_str() {
                "exp" => mul(one(u), du),
                "ln" | "log" => div(du, u),
                "sin" => mul(Expr::Call("cos".into(), vec![u]), du),
                "cos" => neg(mul(Expr::Call("sin".into(), vec![u]), du)),
                "tan" => mul(add(num(1.0), pow(one(u), num(2.0))), du),
                "sinh" => mul(Expr::Call("cosh".into(), vec![u]), du),
                "cosh" => mul(Expr::Call("sinh".into(), vec![u]), du),
                "tanh" => mul(sub(num(1.0), pow(one(u), num(2.0))), du),
                "sqrt" => div(du, mul(num(2.0), one(u))),
                // |u| over the reals: d|u|/dx = (u/|u|)·u' (not differentiable at u=0, giving NaN
                // — that is a mathematical fact)
                "abs" => mul(div(u.clone(), Expr::Call("abs".into(), vec![u])), du),
                // det / trace: expand into an ordinary expression on the literal first, then
                // apply the chain rule
                "det" | "trace" | "solve" | "roots" | "dot" | "norm" => match expand_call(e) {
                    Some(expanded) => diff(&expanded, x),
                    None => num(f64::NAN),
                },
                // The piecewise selection of min/max/clamp cannot be expressed in the IR (there is
                // no comparison operator):
                // their derivatives need an "indicator function", and forcing it would produce an
                // expression that looks right but is wrong.
                "min" | "max" | "clamp" | "inv" => num(f64::NAN),
                // Over the reals conj/re/im are all identities, and the chain rule degenerates to
                // u'
                "conj" | "re" | "im" => du,
                // arg over the reals is only 0 or π (a step), not differentiable
                "arg" => num(f64::NAN),
                _ => num(f64::NAN),
            }
        }
        Expr::Mat(_) | Expr::MatPattern(_) => num(f64::NAN),
        Expr::Transpose(a) => transpose(diff(a, x)),
        // Chain rules for operators (∂²/ Leibniz) are future work: the first version gives NaN
        // explicitly and blocks it before emit
        Expr::Deriv(..)
        | Expr::Grad(_)
        | Expr::Laplacian(_)
        | Expr::Sum { .. }
        | Expr::Integral { .. }
        | Expr::Indexed { .. } => num(f64::NAN),
    }
}

// ---------------------------------------------------------------- dimension checking

pub fn dims_of(decls: &[Decl]) -> Result<HashMap<String, Dim>, Error> {
    let mut m = HashMap::new();
    for d in decls {
        let dim = match &d.unit {
            Some(u) => parse_unit(u).map_err(|unit| Error::DeclarationUnit {
                name: d.name.clone(),
                unit: Box::new(unit),
            })?,
            None => Dim::ZERO,
        };
        if m.insert(d.name.clone(), dim).is_some() {
            // A symbol declared twice with (possibly) different dimensions.
            return Err(Error::DuplicateDeclaration {
                name: d.name.clone(),
                line: None, // decl blocks carry no position; never fabricate line 0
            });
        }
    }
    Ok(m)
}

fn is_zero_lit(e: &Expr) -> bool {
    matches!(e, Expr::Num(v) if *v == 0.0)
}

pub fn dim_check(e: &Expr, dims: &HashMap<String, Dim>) -> Result<Dim, Error> {
    use Expr::*;
    Ok(match e {
        Num(_) => Dim::ZERO,
        // Reserved symbol j = the imaginary unit, always dimensionless (used for EIS complex
        // impedance)
        Var(n) if n == "j" => Dim::ZERO,
        Var(n) => *dims
            .get(n)
            .ok_or_else(|| Error::UndeclaredSymbol(n.clone()))?,
        Neg(a) => dim_check(a, dims)?,
        Add(a, b) | Sub(a, b) => {
            // `0` is the zero element of **any** dimension space: the `0 - x` produced by symbolic
            // differentiation must not be judged a dimension conflict.
            if is_zero_lit(a) {
                return dim_check(b, dims);
            }
            if is_zero_lit(b) {
                return dim_check(a, dims);
            }
            let da = dim_check(a, dims)?;
            let db = dim_check(b, dims)?;
            if da != db {
                return Err(Error::DimensionMismatch {
                    left: da.display(),
                    right: db.display(),
                });
            }
            da
        }
        Mul(a, b) => dim_check(a, dims)?.mul_dim(dim_check(b, dims)?),
        Div(a, b) => dim_check(a, dims)?.div_dim(dim_check(b, dims)?),
        Pow(a, b) => {
            let db = dim_check(b, dims)?;
            if !db.is_zero() {
                return Err(Error::ExponentNotDimensionless(db.display()));
            }
            let da = dim_check(a, dims)?;
            // Rational exponents p/2 (including 0.5 and the **-0.5 produced by symbolic
            // differentiation**) are all handled as
            // `exp * p / 2`: every dimension exponent must be divisible by 2.
            //
            // Fold the exponent to a constant first: `f ^ (1/2)`, `T ^ (-1/2)` are **natural
            // notations**,
            // and requiring the caller to simplify first before passing the gate is a trap
            // (everyone forgets).
            let folded;
            let expo = match b.as_ref() {
                Num(n) => Some(*n),
                other => {
                    // Use recursive simplify rather than a single level of fold_const:
                    // notations like `(1+1)/2`,
                    // `2*0.5` should pass the gate too
                    folded = simplify(other);
                    match &folded {
                        Num(n) => Some(*n),
                        _ => None,
                    }
                }
            };
            // When the base is already dimensionless, any (dimensionless) exponent keeps it
            // dimensionless: `2^j`, `j^j`
            // can be computed at the value layer, so the dimension gate should not block them
            // (otherwise "complex powers" would be an empty promise to the outside)
            if da.is_zero() {
                return Ok(Dim::ZERO);
            }
            match expo {
                Some(n) => {
                    let two_n = n * 2.0;
                    if math::fract(two_n) != 0.0 || two_n.abs() > 1.0e6 {
                        return Err(Error::ExponentNotHalfInteger(format!("{n} is not p/2")));
                    }
                    let p = two_n as i32;
                    let mut r = [0i16; 7];
                    for (a, b) in r.iter_mut().zip(da.0) {
                        let prod = b as i32 * p;
                        if prod % 2 != 0 {
                            return Err(Error::InvalidInput(format!(
                                "`^{n}` needs every exponent of {} to stay even after scaling by {p}, otherwise the dimension is undefined",
                                da.display()
                            )));
                        }
                        *a = (prod / 2) as i16;
                    }
                    Dim(r)
                }
                None => {
                    return Err(Error::InvalidInput(
                        "the exponent must be a numeric constant".into(),
                    ))
                }
            }
        }
        Call(name, args) => {
            let Some(arg0) = args.first() else {
                return Err(Error::FunctionArity {
                    name: name.clone(),
                    expected: 1,
                    got: 0,
                });
            };
            // The system form `solve([eq…], [x…])`: `arg0` is a **list of equations**,
            // each equation only needs to be **homogeneous under addition/subtraction** on its
            // own, and different equations may have different dimensions
            // (a force balance is N, a geometric constraint is m — that is a completely legal
            // system).
            // Going through the generic `Mat` check would require the elements to share a
            // dimension, and would wrongly reject such systems.
            if name == "solve" && matches!(arg0, Expr::Mat(_)) {
                if let Some(eqs) = flat_mat_items(arg0) {
                    for eq in &eqs {
                        // Homogeneity of a single equation: not required to equal some particular
                        // dimension, only to check through
                        dim_check(eq, dims)?;
                    }
                }
                return match expand_call(e) {
                    Some(x) => dim_check(&x, dims),
                    None => Err(Error::InvalidInput(
            "`solve` needs its second argument to be a variable name or a list of names, and the equations to be linear in the unknowns".into(),
        )),
                };
            }
            if matches!(name.as_str(), "__quad_root_1" | "__quad_root_2") {
                if args.len() != 3 {
                    return Err(Error::FunctionArity {
                        name: name.clone(),
                        expected: 3,
                        got: args.len(),
                    });
                }
                // When a·x²+b·x+c is homogeneous, both b/a and c/b have x's dimension.
                let da = dim_check(&args[0], dims)?;
                let db = dim_check(&args[1], dims)?;
                return Ok(db.div_dim(da));
            }
            let d = dim_check(arg0, dims)?;
            match name.as_str() {
                "exp" | "ln" | "log" | "sin" | "cos" | "tan" | "sinh" | "cosh" | "tanh" => {
                    if !d.is_zero() {
                        return Err(
                            (format!("`{name}` 的参数必须无量纲，得到 {}", d.display())).into()
                        );
                    }
                    Dim::ZERO
                }
                "sqrt" => {
                    let mut r = [0i16; 7];
                    for (a, b) in r.iter_mut().zip(d.0) {
                        if b % 2 != 0 {
                            return Err(Error::InvalidInput(format!(
                                "`sqrt` needs a dimension whose exponents are all even, got {}",
                                d.display()
                            )));
                        }
                        *a = b / 2;
                    }
                    Dim(r)
                }
                // The four/five complex ones: modulus / argument / conjugate / real part /
                // imaginary part.
                // arg is always dimensionless (radians); the rest keep the argument's dimension.
                "arg" => Dim::ZERO,
                "abs" | "conj" | "re" | "im" => d,
                // min/max/clamp: all three arguments must share a dimension, and the result has
                // the same dimension
                "min" | "max" | "clamp" => {
                    for a in &args[1..] {
                        let other = dim_check(a, dims)?;
                        if other != d {
                            return Err(Error::InvalidInput(format!(
                                "`{name}` requires all its arguments to have the same dimension: {} vs {}",
                                d.display(),
                                other.display()
                            )));
                        }
                    }
                    d
                }
                // Matrix operators: det is the n-th power, trace/adjoint keep the same dimension.
                // For the order, prefer flatten (which looks through inv/adjoint — after folding
                // they are
                // literals), so `det(inv(J))` is no longer wrongly rejected by the dimension
                // gate.
                "det" => {
                    let n = match flatten_matrix_value(&args[0]) {
                        Some(m) if !m.is_empty() && m.iter().all(|r| r.len() == m.len()) => m.len(),
                        _ => match shape_of(&args[0])? {
                            Some((r, c)) if r == c && r > 0 => r,
                            _ => {
                                return Err(Error::InvalidInput("`det` needs a concrete square matrix literal (its shape is unknown, so the dimension cannot be computed)".into()))
                            }
                        },
                    };
                    d.scale_ratio(n as i32, 1)?
                }
                "trace" | "adjoint" => d,
                // The dimension of an inverse matrix is the reciprocal
                "inv" => d.scale_ratio(-1, 1)?,
                // Dot product: both sides must share a dimension, and the result is the square of
                // that dimension
                "dot" => {
                    let d2 = dim_check(&args[1], dims)?;
                    if d2 != d {
                        return Err(Error::InvalidInput(format!(
                            "`dot` requires both arguments to have the same dimension: {} vs {}",
                            d.display(),
                            d2.display()
                        )));
                    }
                    d.mul_dim(d)
                }
                // Norm: √Σv² has the same dimension as the elements
                "norm" => d,
                // Linear solving / root finding / coefficient extraction: the dimension is always
                // determined by the expansion (single mathematical source)
                "roots" | "solve" | "poly_coeffs" => {
                    // A coefficient vector requires **elements of the same dimension** (the value
                    // lane of matrix expressions), while for a homogeneous polynomial
                    // the c_k have dimension `dim(poly)/dim(x)^k` — they agree only when x is
                    // dimensionless.
                    // This is not an implementation restriction but an algebraic fact: numbers of
                    // different dimensions cannot be arranged into a matrix.
                    // So it is explained explicitly here, rather than letting downstream report
                    // "inconsistent matrix element dimensions: m vs 1".
                    if name == "poly_coeffs" {
                        if let Some(Expr::Var(v)) = args.get(1) {
                            let dv = dims.get(v).copied().unwrap_or(Dim::ZERO);
                            if !dv.is_zero() {
                                return Err(Error::InvalidInput(format!(
                "`poly_coeffs`: the coefficients of `{v}` have dimension {}, but matrix elements must share one dimension; take coefficients of a dimensionless variable (for example divide by a scale first) or use `roots`",
                dv.display()
            )));
                            }
                        }
                    }
                    match expand_call(e) {
                        Some(x) => return dim_check(&x, dims),
                        // The error message must use the **actual** function name and criterion:
                        // `roots` talks about polynomials,
                        // not "the equation is linear in it" (a `roots(exp(x), x)` used to be
                        // reported as a solve error).
                        None if name == "poly_coeffs" => {
                            return Err(Error::InvalidInput(
            "`poly_coeffs(poly, x)` needs its second argument to be a variable name and its first to be a univariate polynomial in it".into(),
        ))
                        }
                        None if name == "roots" => {
                            return Err(Error::InvalidInput(
            "`roots(poly, x)` needs its second argument to be a variable name and its first to be a univariate polynomial in it".into(),
        ))
                        }
                        None => return Err(Error::InvalidInput(
            "`solve` needs its second argument to be a variable name and the equation to be linear in it".into(),
        )),
                    }
                }
                other => {
                    return Err(Error::UnknownFunction {
                        name: other.to_string(),
                        suggestion: String::new(),
                    })
                }
            }
        }
        Mat(rows) => {
            let mut acc: Option<Dim> = None;
            for r in rows {
                for c in r {
                    if is_zero_lit(c) {
                        continue; // the zero element is dimension-polymorphic
                    }
                    let d = dim_check(c, dims)?;
                    match acc {
                        None => acc = Some(d),
                        Some(prev) if prev == d => {}
                        Some(prev) => {
                            return Err(Error::InvalidInput(format!(
                                "matrix elements must all share one dimension: {} vs {}",
                                prev.display(),
                                d.display()
                            )))
                        }
                    }
                }
            }
            acc.unwrap_or(Dim::ZERO)
        }
        MatPattern(p) => {
            // A pattern matrix only constrains its **concrete** elements (ellipsis cells do not
            // participate).
            let grid = p.grid().ok_or("模式矩阵各行长度不一致")?;
            if grid.0 < 2 || grid.1 < 2 {
                return Err(Error::InvalidInput(
                    "a pattern matrix must be at least 2×2, otherwise the ellipsis means nothing"
                        .into(),
                ));
            }
            let mut acc: Option<Dim> = None;
            for e in p.elements() {
                if is_zero_lit(e) {
                    continue; // the zero element is dimension-polymorphic
                }
                let d = dim_check(e, dims)?;
                match acc {
                    None => acc = Some(d),
                    Some(prev) if prev == d => {}
                    Some(prev) => {
                        return Err(Error::InvalidInput(format!(
                            "pattern-matrix elements must all share one dimension: {} vs {}",
                            prev.display(),
                            d.display()
                        )))
                    }
                }
            }
            acc.unwrap_or(Dim::ZERO)
        }
        Transpose(a) => dim_check(a, dims)?,
        Deriv(u, by) => {
            let du = dim_check(u, dims)?;
            // The differentiation variable must have a known dimension (implicit function
            // differentiation is not in the first version)
            let dx = *dims
                .get(by)
                .ok_or_else(|| format!("微分变量 `{by}` 未声明量纲"))?;
            du.div_dim(dx)
        }
        Grad(u) => dim_check(u, dims)?.div_dim(parse_unit("m")?), // Cartesian metric assumption
        Laplacian(u) => {
            let d = dim_check(u, dims)?;
            let m2 = parse_unit("m")?.scale_ratio(2, 1)?;
            d.div_dim(m2)
        }
        Sum {
            var,
            lower,
            upper,
            body,
        } => {
            let dl = dim_check(lower, dims)?;
            let du = dim_check(upper, dims)?;
            if !dl.is_zero() || !du.is_zero() {
                return Err(Error::InvalidInput(
                    "the sum bounds must be dimensionless (the index is a counter)".into(),
                ));
            }
            let mut inner = dims.clone();
            inner.insert(var.clone(), Dim::ZERO); // the bound index is dimensionless
            dim_check(body, &inner)?
        }
        Integral {
            var,
            lower,
            upper,
            body,
        } => {
            let dl = dim_check(lower, dims)?;
            let du = dim_check(upper, dims)?;
            let dv = *dims
                .get(var)
                .ok_or_else(|| format!("积分变量 `{var}` 未声明量纲"))?;
            if dl != du {
                return Err(
                    (format!("积分界限量纲不一致：{} vs {}", dl.display(), du.display())).into(),
                );
            }
            let db = dim_check(body, dims)?;
            db.mul_dim(dv)
        }
        Indexed { name, indices } => {
            let d = *dims
                .get(name)
                .ok_or_else(|| format!("索引符号 `{name}` 未声明量纲"))?;
            for i in indices {
                if !dims.get(i).map(|x| x.is_zero()).unwrap_or(false) {
                    return Err(Error::InvalidInput(format!(
                "the index `{i}` must be a dimensionless symbol (it is undeclared or carries a dimension)"
            )));
                }
            }
            d
        }
    })
}

// ---------------------------------------------------------------- structural key (CSE / equality)

pub fn key(e: &Expr) -> String {
    let mut s = String::new();
    key_into(e, &mut s);
    s
}

fn key_into(e: &Expr, s: &mut String) {
    match e {
        Expr::Num(v) => {
            let _ = write!(s, "#{:016x}", v.to_bits());
        }
        Expr::Var(n) => {
            s.push('$');
            s.push_str(n);
        }
        Expr::Add(a, b) => bin(s, 'A', a, b),
        Expr::Sub(a, b) => bin(s, 'S', a, b),
        Expr::Mul(a, b) => bin(s, 'M', a, b),
        Expr::Div(a, b) => bin(s, 'D', a, b),
        Expr::Pow(a, b) => bin(s, 'P', a, b),
        Expr::Neg(a) => {
            s.push_str("(N");
            key_into(a, s);
            s.push(')');
        }
        Expr::Call(n, args) => {
            s.push_str("(C");
            s.push_str(n);
            for a in args {
                key_into(a, s);
            }
            s.push(')');
        }
        Expr::Mat(rows) => {
            s.push_str("(T");
            for r in rows {
                for c in r {
                    key_into(c, s);
                }
                s.push('|');
            }
            s.push(')');
        }
        Expr::MatPattern(p) => {
            s.push_str("(TP");
            for r in &p.rows {
                for c in r {
                    match c {
                        PatternCell::El(e) => key_into(e, s),
                        PatternCell::Dots => s.push('\u{b7}'),
                    }
                }
                s.push('|');
            }
            s.push(')');
        }
        Expr::Transpose(a) => {
            s.push_str("(TR");
            key_into(a, s);
            s.push(')');
        }
        Expr::Deriv(u, by) => {
            s.push_str("(DV");
            key_into(u, s);
            s.push('$');
            s.push_str(by);
            s.push(')');
        }
        Expr::Grad(u) => {
            s.push_str("(GR");
            key_into(u, s);
            s.push(')');
        }
        Expr::Laplacian(u) => {
            s.push_str("(LP");
            key_into(u, s);
            s.push(')');
        }
        Expr::Sum {
            var,
            lower,
            upper,
            body,
        } => {
            s.push_str("(SM$");
            s.push_str(var);
            key_into(lower, s);
            key_into(upper, s);
            key_into(body, s);
            s.push(')');
        }
        Expr::Integral {
            var,
            lower,
            upper,
            body,
        } => {
            s.push_str("(IN$");
            s.push_str(var);
            key_into(lower, s);
            key_into(upper, s);
            key_into(body, s);
            s.push(')');
        }
        Expr::Indexed { name, indices } => {
            s.push_str("(IX$");
            s.push_str(name);
            for i in indices {
                s.push('$');
                s.push_str(i);
            }
            s.push(')');
        }
    }
}

fn bin(s: &mut String, tag: char, a: &Expr, b: &Expr) {
    s.push('(');
    s.push(tag);
    key_into(a, s);
    key_into(b, s);
    s.push(')');
}

// ---------------------------------------------------------------- LaTeX write-back

fn vs(n: &str) -> Expr {
    Expr::Var(n.to_string())
}

pub fn to_latex(e: &Expr) -> String {
    to_latex_prec(e, 0)
}

/// Precedence-aware LaTeX write-back: `(j·ω)^(1/2)` without its parentheses renders as `j·√ω`
/// — the write-back is the same single source of truth for doc comments / hover / sidecar, so
/// wrong parentheses mean a wrong formula.
fn to_latex_prec(e: &Expr, min: u8) -> String {
    let (text, p) = to_latex_raw(e);
    wrap(text, p, min)
}

fn to_latex_raw(e: &Expr) -> (String, u8) {
    match e {
        Expr::Num(v) => (fmt_num(*v), 9),
        Expr::Var(n) => (latex_var(n), 9),
        Expr::Neg(a) => (format!("-{}", to_latex_prec(a, 5)), 4),
        Expr::Add(a, b) => (
            format!("{} + {}", to_latex_prec(a, 2), to_latex_prec(b, 3)),
            2,
        ),
        Expr::Sub(a, b) => (
            format!("{} - {}", to_latex_prec(a, 2), to_latex_prec(b, 3)),
            2,
        ),
        Expr::Mul(a, b) => (
            format!("{} \\cdot {}", to_latex_prec(a, 3), to_latex_prec(b, 4)),
            3,
        ),
        // \frac already groups, so numerator and denominator need no extra parentheses
        Expr::Div(a, b) => (
            format!(
                "\\frac{{{}}}{{{}}}",
                to_latex_prec(a, 0),
                to_latex_prec(b, 0)
            ),
            3,
        ),
        // The superscript braces already group, but if the **base** is itself a power/transpose it
        // must get parentheses:
        // `x^{2}^{3}` is a TeX double-superscript error (rendering fails outright), and
        // `x^{T}^{2}` would also be
        // parsed back as Pow(Pow(x,T),2) — the transpose is lost. Hence min is taken as 7 > the
        // power precedence 6.
        Expr::Pow(a, b) => (
            format!("{}^{{{}}}", to_latex_prec(a, 7), to_latex_prec(b, 0)),
            6,
        ),
        Expr::Call(n, args) => {
            let inner: Vec<String> = args.iter().map(|x| to_latex_prec(x, 0)).collect();
            let text = match n.as_str() {
                // This dialect accepts only \sqrt{...} (\sqrt(...) is not LaTeX syntax)
                "sqrt" => format!("\\sqrt{{{}}}", inner.join(", ")),
                // The complex operators have no fixed glyph (`|z|` is ambiguous with matrix/
                // determinant delimiters),
                // so they are spelled with \operatorname{...}, ensuring the write-back can be
                // consumed verbatim by the parser.
                "abs" | "arg" | "conj" | "re" | "im" | "clamp" | "trace" | "adjoint" | "inv"
                | "solve" | "roots" | "poly_coeffs" | "norm" | "dot" => {
                    format!("\\operatorname{{{}}}({})", n, inner.join(", "))
                }
                _ => format!("\\{}({})", n.as_str(), inner.join(", ")),
            };
            (text, 9)
        }
        Expr::Mat(rows) => {
            let body: Vec<String> = rows
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|x| to_latex_prec(x, 0))
                        .collect::<Vec<_>>()
                        .join(" & ")
                })
                .collect();
            (
                format!("\\begin{{bmatrix}}{}\\end{{bmatrix}}", body.join(" \\\\ ")),
                9,
            )
        }
        Expr::MatPattern(p) => {
            let (m, n) = p.grid().unwrap_or((p.rows.len(), 0));
            let has_el_row: Vec<bool> = p
                .rows
                .iter()
                .map(|r| r.iter().any(|c| matches!(c, PatternCell::El(_))))
                .collect();
            let has_el_col: Vec<bool> = (0..n.max(p.rows.first().map(|r| r.len()).unwrap_or(0)))
                .map(|j| {
                    p.rows
                        .iter()
                        .any(|r| matches!(r.get(j), Some(PatternCell::El(_))))
                })
                .collect();
            let _ = m;
            let body: Vec<String> = p
                .rows
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    r.iter()
                        .enumerate()
                        .map(|(j, c)| match c {
                            PatternCell::El(e) => to_latex_prec(e, 0),
                            PatternCell::Dots => {
                                // Direction inference: the row has a concrete element → cdots;
                                // otherwise the column has one → vdots; otherwise ddots.
                                if has_el_row[i] {
                                    "\\cdots".to_string()
                                } else if has_el_col[j] {
                                    "\\vdots".to_string()
                                } else {
                                    "\\ddots".to_string()
                                }
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" & ")
                })
                .collect();
            (
                format!("\\begin{{bmatrix}}{}\\end{{bmatrix}}", body.join(" \\\\ ")),
                9,
            )
        }
        Expr::Transpose(a) => (
            // If the transpose's base is another transpose: `x^{T}^{T}` is a TeX double-superscript
            // error, so it must get parentheses
            format!("{}^{{T}}", to_latex_prec(a, 7)),
            6,
        ),
        Expr::Deriv(u, by) => (
            format!(
                "\\frac{{\\partial {}}}{{\\partial {}}}",
                to_latex_prec(u, 0),
                to_latex_prec(&vs(by), 0)
            ),
            3,
        ),
        // ∇ is a **prefix** operator: when it is the base of a power/transpose it must get
        // parentheses, otherwise
        // `(∇u)^2` and `∇(u^2)` would be written as the same string (the semantics get
        // rewritten). p=5 < 7.
        Expr::Grad(u) => (format!("\\nabla {}", to_latex_prec(u, 5)), 5),
        Expr::Laplacian(u) => (format!("\\nabla^2 {}", to_latex_prec(u, 5)), 5),
        Expr::Sum {
            var,
            lower,
            upper,
            body,
        } => (
            format!(
                "\\sum_{{{}={}}}^{{{}}} {}",
                to_latex_prec(&vs(var), 0),
                to_latex_prec(lower, 0),
                to_latex_prec(upper, 0),
                to_latex_prec(body, 5)
            ),
            // Must get parentheses as a power base: `\sum i^{2}` with another superscript is a TeX
            // double-superscript error
            5,
        ),
        Expr::Integral {
            var,
            lower,
            upper,
            body,
        } => (
            format!(
                "\\int_{{{}}}^{{{}}} {} \\, d{}",
                to_latex_prec(lower, 0),
                to_latex_prec(upper, 0),
                to_latex_prec(body, 5),
                to_latex_prec(&vs(var), 0)
            ),
            5,
        ),
        Expr::Indexed { name, indices } => {
            let idx: Vec<String> = indices.iter().map(|i| to_latex_prec(&vs(i), 0)).collect();
            (
                format!("{}_{{{}}}", to_latex_prec(&vs(name), 0), idx.join(", ")),
                9,
            )
        }
    }
}

/// Variable name → LaTeX form.
///
/// Three cases, and the order must not change:
/// 1. A single ASCII letter → verbatim (`x`);
/// 2. The name is **exactly** a Greek letter name (or its `_subscript` form) → the real glyph
///    (`sigma` → `\sigma`);
/// 3. Any other multi-letter name → `\mathrm{name}`.
///
/// The implementation used to be `replace("alpha", "\\alpha").replace("eta", "\\eta")` — that is
/// **substring replacement**: `alphabet` became `\alphabet`, `betamax` became `b\etamax`,
/// and the more common `sigma` was treated as the product of the five variables s·i·g·m·a. The
/// write-back is the same single source of truth for hover/sidecar/doc,
/// so being wrong here means the formula is wrong.
pub fn latex_var(n: &str) -> String {
    const GREEK: &[&str] = &[
        "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa",
        "lambda", "mu", "nu", "xi", "pi", "rho", "sigma", "tau", "upsilon", "phi", "chi", "psi",
        "omega", "Gamma", "Delta", "Theta", "Lambda", "Xi", "Sigma", "Upsilon", "Phi", "Psi",
        "Omega",
    ];
    if n.len() == 1 && n.chars().all(|c| c.is_ascii_alphabetic()) {
        return n.to_string();
    }
    let (head, tail) = match n.split_once('_') {
        Some((h, t)) => (h, format!("_{t}")),
        None => (n, String::new()),
    };
    if GREEK.contains(&head) {
        return format!("\\{head}{tail}");
    }
    format!("\\mathrm{{{n}}}")
}

pub fn fmt_num(v: f64) -> String {
    if v == math::trunc(v) && v.abs() < 1e15 {
        format!("{v:.0}")
    } else {
        format!("{v:?}")
    }
}

// ---------------------------------------------------------------- Rust code generation

fn wrap(s: String, p: u8, min: u8) -> String {
    if p < min {
        format!("({s})")
    } else {
        s
    }
}

fn rust_ident(n: &str) -> String {
    let mut s: String = n
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true) {
        s.insert(0, '_');
    }
    s
}

/// Generate a block expression: named intermediates (one `let tN` per shared subtree) + the final
/// expression.
///
/// The named intermediates are the real source of "readability": debugger / profiler / generated
/// code are aligned through them.
/// Floating-point width of the generated code. The `f32` variant only swaps the **literal suffix**
/// (all other emission rules are identical).
#[derive(Clone, Copy, PartialEq)]
pub enum Fx {
    F64,
    F32,
    /// `no_std` + `libm`: transcendental functions go through `libm::*`, integer powers through
    /// `libm::pow`
    /// (`f64::powi` is a std thing, not available in `core`).
    F64Libm,
}

impl Fx {
    /// Literal text: `f64` keeps `{:?}` verbatim, `f32` adds a suffix to force type inference.
    fn lit(self, v: f64) -> String {
        match self {
            Fx::F64 | Fx::F64Libm => format!("{v:?}"),
            Fx::F32 => format!("{:?}f32", v as f32),
        }
    }

    fn is_libm(self) -> bool {
        matches!(self, Fx::F64Libm)
    }
}

/// Function names in `libm::*` (used by the `no_std` lane).
fn libm_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "exp" => "exp",
        "ln" | "log" => "log",
        "sqrt" => "sqrt",
        "sin" => "sin",
        "cos" => "cos",
        "tan" => "tan",
        "sinh" => "sinh",
        "cosh" => "cosh",
        "tanh" => "tanh",
        "abs" => "fabs",
        _ => return None,
    })
}

/// The `no_std` (libm) variant block: the same IR, with transcendental functions going through
/// `libm::*`.
///
/// The consumer needs the `libm` dependency (`extern crate libm;` or libm's std compatibility
/// layer).
/// The output itself is still pure `core` syntax: no `std`, no allocation, no `f64::powi`.
pub fn emit_block_libm(root: &Expr) -> String {
    emit_block_fx(root, Fx::F64Libm)
}

pub fn emit_block(root: &Expr) -> String {
    emit_block_fx(root, Fx::F64)
}

/// The `f32` variant block: the same IR, with literals carrying the `f32` suffix.
pub fn emit_block_f32(root: &Expr) -> String {
    emit_block_fx(root, Fx::F32)
}

fn emit_block_fx(root: &Expr, fx: Fx) -> String {
    // The no_std lane is scalars only: matrix-valued expressions (`vec![…]` needs alloc) are
    // rejected as a whole block
    if fx.is_libm() && is_matrix_value(root) {
        return "{\n        compile_error!(\"no_std 车道不做矩阵值公式（vec! 要 alloc）：请在 CPU 侧装配，或走 std 车道\")\n    }"
            .to_string();
    }
    let mut counts: HashMap<String, usize> = HashMap::new();
    count_nodes(root, &mut counts);
    let mut out: Vec<String> = Vec::new();
    let mut names: HashMap<String, String> = HashMap::new();
    let mut counter = 0usize;
    let (body, _) = emit_node(root, true, &counts, &mut names, &mut out, &mut counter, fx);
    let mut s = String::from("{\n");
    for line in &out {
        let _ = writeln!(s, "        {line}");
    }
    let _ = writeln!(s, "        {body}");
    s.push_str("    }");
    s
}

// ---------------------------------------------------------------- complex code generation
//
// The complex emission lane is **completely separate** from the real one and does not share
// emit_node: the real lane squeezes parentheses to the minimum,
// while in the complex lane every operator is one function call (naturally atomic); sharing would
// only make both sides uglier.
//
// Generated values are represented as a **tuple `(f64, f64)` = (re, im)**, rather than defining a
// new `struct Cmplx`:
// a tuple is a type a block expression can return directly, so the consumer
// `let z = formula!(...)` just gets it,
// with no named type visible at the call site — this is a hard constraint for the **macro inlining**
// lane (a macro cannot
// inject type definitions at the call site).

/// Whether the expression contains the imaginary unit `j` (deciding whether to take the complex
/// emission lane).
pub fn uses_complex(e: &Expr) -> bool {
    match e {
        Expr::Var(n) => n == "j",
        Expr::Num(_) | Expr::Indexed { .. } => false,
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
            uses_complex(a) || uses_complex(b)
        }
        Expr::Neg(a)
        | Expr::Transpose(a)
        | Expr::Deriv(a, _)
        | Expr::Grad(a)
        | Expr::Laplacian(a) => uses_complex(a),
        Expr::Call(_, args) => args.iter().any(uses_complex),
        Expr::Mat(rows) => rows.iter().flatten().any(uses_complex),
        Expr::MatPattern(p) => p.elements().into_iter().any(uses_complex),
        Expr::Sum {
            lower, upper, body, ..
        }
        | Expr::Integral {
            lower, upper, body, ..
        } => uses_complex(lower) || uses_complex(upper) || uses_complex(body),
    }
}

/// Source of the complex operator closures. Emitted only when **actually used** — the output must
/// pass `clippy -D warnings`, and an unused closure would be judged dead code.
fn complex_helper(name: &str) -> &'static str {
    // Use a **block-level fn item** rather than a closure: `fn` can carry `#[inline(always)]`,
    // a closure cannot.
    // Measured: the closure form is 5× slower than "the same thing written out by hand" in batch
    // processing — the operators were not inlined,
    // so the loop could not be vectorized. After switching to fn items the gap narrows to ~1.3×.
    match name {
        "__c_add" => "#[inline(always)]\nfn __c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {\n    (a.0 + b.0, a.1 + b.1)\n}",
        "__c_sub" => "#[inline(always)]\nfn __c_sub(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {\n    (a.0 - b.0, a.1 - b.1)\n}",
        "__c_neg" => "#[inline(always)]\nfn __c_neg(a: (f64, f64)) -> (f64, f64) {\n    (-a.0, -a.1)\n}",
        "__c_mul" => "#[inline(always)]\nfn __c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {\n    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)\n}",
        // Ordinary magnitudes take the naive form = 1 division; when |b| leaves the safe range,
        // scale first and then compute
        // (the naive re²+im² overflows for |b| > ~1e154 and underflows to a false division by zero
        // for |b| < ~1e-162).
        // The same stable algorithm as Smith's at the value layer; extreme bit patterns may differ
        // by 1 ulp.
        "__c_div" => "#[inline(always)]\nfn __c_div(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {\n    let m = b.0.abs().max(b.1.abs());\n    if m > 1e-150 && m < 1e150 {\n        let d = b.0 * b.0 + b.1 * b.1;\n        ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)\n    } else {\n        let (br, bi) = (b.0 / m, b.1 / m);\n        let s = m * (br * br + bi * bi);\n        ((a.0 * br + a.1 * bi) / s, (a.1 * br - a.0 * bi) / s)\n    }\n}",
        "__c_abs" => "#[inline(always)]\nfn __c_abs(a: (f64, f64)) -> (f64, f64) {\n    (a.0.hypot(a.1), 0.0)\n}",
        "__c_arg" => "#[inline(always)]\nfn __c_arg(a: (f64, f64)) -> (f64, f64) {\n    (a.1.atan2(a.0), 0.0)\n}",
        "__c_conj" => "#[inline(always)]\nfn __c_conj(a: (f64, f64)) -> (f64, f64) {\n    (a.0, -a.1)\n}",
        "__c_re" => "#[inline(always)]\nfn __c_re(a: (f64, f64)) -> (f64, f64) {\n    (a.0, 0.0)\n}",
        "__c_im" => "#[inline(always)]\nfn __c_im(a: (f64, f64)) -> (f64, f64) {\n    (a.1, 0.0)\n}",
        "__c_exp" => "#[inline(always)]\nfn __c_exp(a: (f64, f64)) -> (f64, f64) {\n    if a.1 == 0.0 {\n        return (a.0.exp(), 0.0);\n    }\n    let m = a.0.exp();\n    (m * a.1.cos(), m * a.1.sin())\n}",
        "__c_ln" => "#[inline(always)]\nfn __c_ln(a: (f64, f64)) -> (f64, f64) {\n    (a.0.hypot(a.1).ln(), a.1.atan2(a.0))\n}",
        // Principal-value square root: algebraic closed form. The modulus must use hypot:
        // `re²+im²` overflows for |a|>~1e154,
        // while the value layer has always used hypot; the generated side must not silently give
        // inf here.
        "__c_sqrt" => "#[inline(always)]\nfn __c_sqrt(a: (f64, f64)) -> (f64, f64) {\n    if a.1 == 0.0 {\n        return if a.0 >= 0.0 { (a.0.sqrt(), 0.0) } else { (0.0, (-a.0).sqrt()) };\n    }\n    if a.0 == 0.0 {\n        let m = (a.1.abs() * 0.5).sqrt();\n        return (m, m.copysign(a.1));\n    }\n    let r = a.0.hypot(a.1);\n    let re = (((r + a.0) * 0.5).max(0.0)).sqrt();\n    let im = (((r - a.0) * 0.5).max(0.0)).sqrt().copysign(a.1);\n    (re, im)\n}",
        // Stable quadratic roots: q/a and c/q, where `first` preserves the original
        // (−b−√D)-first return order.
        // A zero double root with q=0 falls back to −b/(2a), producing no 0/0.
        "__c_quad_root" => "#[inline(always)]\nfn __c_quad_root(a: (f64, f64), b: (f64, f64), c: (f64, f64), first: bool) -> (f64, f64) {\n    let d = __c_sub(__c_mul(b, b), __c_mul((4.0, 0.0), __c_mul(a, c)));\n    let s = __c_sqrt(d);\n    let q_first = __c_mul(__c_conj(b), s).0 >= 0.0;\n    let signed_s = if q_first { s } else { __c_neg(s) };\n    let q = __c_mul((-0.5, 0.0), __c_add(b, signed_s));\n    if q.0 == 0.0 && q.1 == 0.0 {\n        return __c_div(__c_neg(b), __c_mul((2.0, 0.0), a));\n    }\n    let large = __c_div(q, a);\n    let small = __c_div(c, q);\n    if first == q_first { large } else { small }\n}",
        // Agrees pointwise with the value layer's `C::pow_real`: an integer power of a negative
        // real base must land exactly on the real axis
        "__c_powf" => "#[inline(always)]\nfn __c_powf(a: (f64, f64), p: f64) -> (f64, f64) {\n    if !p.is_finite() {\n        return (f64::NAN, 0.0);\n    }\n    if a.1 == 0.0 {\n        if p == p.trunc() && p.abs() <= i32::MAX as f64 {\n            return (a.0.powi(p as i32), 0.0);\n        }\n        if a.0 >= 0.0 {\n            return (a.0.powf(p), 0.0);\n        }\n    }\n    let (r, t) = (a.0.hypot(a.1), a.1.atan2(a.0));\n    let m = r.powf(p);\n    let g = t * p;\n    (m * g.cos(), m * g.sin())\n}",
        "__c_pow" => "#[inline(always)]\nfn __c_pow(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {\n    let lr = a.0.hypot(a.1).ln();\n    let li = a.1.atan2(a.0);\n    let (er, ei) = (b.0 * lr - b.1 * li, b.0 * li + b.1 * lr);\n    let m = er.exp();\n    (m * ei.cos(), m * ei.sin())\n}",
        "__c_sin" => "#[inline(always)]\nfn __c_sin(a: (f64, f64)) -> (f64, f64) {\n    (a.0.sin() * a.1.cosh(), a.0.cos() * a.1.sinh())\n}",
        "__c_cos" => "#[inline(always)]\nfn __c_cos(a: (f64, f64)) -> (f64, f64) {\n    (a.0.cos() * a.1.cosh(), -(a.0.sin() * a.1.sinh()))\n}",
        "__c_sinh" => "#[inline(always)]\nfn __c_sinh(a: (f64, f64)) -> (f64, f64) {\n    (a.0.sinh() * a.1.cos(), a.0.cosh() * a.1.sin())\n}",
        "__c_cosh" => "#[inline(always)]\nfn __c_cosh(a: (f64, f64)) -> (f64, f64) {\n    (a.0.cosh() * a.1.cos(), a.0.sinh() * a.1.sin())\n}",
        "__c_tan" => "#[inline(always)]\nfn __c_tan(a: (f64, f64)) -> (f64, f64) {\n    let s = (a.0.sin() * a.1.cosh(), a.0.cos() * a.1.sinh());\n    let c = (a.0.cos() * a.1.cosh(), -(a.0.sin() * a.1.sinh()));\n    let d = c.0 * c.0 + c.1 * c.1;\n    ((s.0 * c.0 + s.1 * c.1) / d, (s.1 * c.0 - s.0 * c.1) / d)\n}",
        "__c_tanh" => "#[inline(always)]\nfn __c_tanh(a: (f64, f64)) -> (f64, f64) {\n    let s = (a.0.sinh() * a.1.cos(), a.0.cosh() * a.1.sin());\n    let c = (a.0.cosh() * a.1.cos(), a.0.sinh() * a.1.sin());\n    let d = c.0 * c.0 + c.1 * c.1;\n    ((s.0 * c.0 + s.1 * c.1) / d, (s.1 * c.0 - s.0 * c.1) / d)\n}",
        other => panic!("未登记的复数算子 `{other}`"),
    }
}

struct CEmit {
    counts: HashMap<String, usize>,
    names: HashMap<String, String>,
    helpers: BTreeSet<&'static str>,
    temps: Vec<String>,
    counter: usize,
    /// Bindings that are already `(f64, f64)` (the federation let chain of the macro lane): when
    /// referencing them one must **not**
    /// wrap them in `(x, 0.0)` again, otherwise it becomes a tuple of tuples.
    cx_vars: BTreeSet<String>,
}

impl CEmit {
    fn help(&mut self, name: &'static str) -> &'static str {
        self.helpers.insert(name);
        name
    }

    /// CSE: lift a temporary once the same subexpression appears ≥2 times (the same criterion as
    /// the real lane).
    fn node(&mut self, e: &Expr, is_root: bool) -> String {
        let k = key(e);
        let shareable = !is_root
            && self.counts.get(&k).copied().unwrap_or(0) >= 2
            && !matches!(e, Expr::Num(_) | Expr::Var(_));
        if shareable {
            if let Some(n) = self.names.get(&k) {
                return n.clone();
            }
        }
        let text = self.value(e);
        if shareable {
            let name = format!("__t{}", self.counter);
            self.counter += 1;
            self.temps.push(format!("let {name} = {text};"));
            self.names.insert(k, name.clone());
            return name;
        }
        text
    }

    fn call1(&mut self, helper: &'static str, e: &Expr) -> String {
        let a = self.node(e, false);
        let h = self.help(helper);
        format!("{h}({a})")
    }

    fn call2(&mut self, helper: &'static str, a: &Expr, b: &Expr) -> String {
        let x = self.node(a, false);
        let y = self.node(b, false);
        let h = self.help(helper);
        format!("{h}({x}, {y})")
    }

    fn value(&mut self, e: &Expr) -> String {
        // Matrix **values** are all folded into literals here and then emitted cell by cell.
        // Without this step, `Mat+Mat` / `k*Mat` / `Mat*Mat` fall into `__c_add`/`__c_mul`
        // — those two helpers only take `(f64, f64)` scalars, so the output is `error[E0308]`
        // (a diagnostic worse than the explicit `compile_error!` it replaced). The real lane always
        // goes through
        // `flatten_matrix_value`; this is the same strategy in the complex lane.
        if !matches!(e, Expr::Mat(_)) {
            if let Some(m) = flatten_matrix_value(e) {
                if !m.is_empty() {
                    let mut parts: Vec<String> = Vec::with_capacity(m.len());
                    for r in &m {
                        let mut cols: Vec<String> = Vec::with_capacity(r.len());
                        for cell in r {
                            cols.push(self.node(cell, false));
                        }
                        parts.push(format!("vec![{}]", cols.join(", ")));
                    }
                    return format!("vec![{}]", parts.join(", "));
                }
            }
        }
        match e {
            Expr::Num(v) => format!("({v:?}, 0.0)"),
            Expr::Var(n) if n == "j" => "(0.0, 1.0)".to_string(),
            Expr::Var(n) if self.cx_vars.contains(n) => rust_ident(n),
            Expr::Var(n) => format!("({}, 0.0)", rust_ident(n)),
            Expr::Neg(a) => self.call1("__c_neg", a),
            Expr::Add(a, b) => self.call2("__c_add", a, b),
            Expr::Sub(a, b) => self.call2("__c_sub", a, b),
            Expr::Mul(a, b) => self.call2("__c_mul", a, b),
            Expr::Div(a, b) => self.call2("__c_div", a, b),
            Expr::Pow(a, b) => match b.as_ref() {
                Expr::Num(n) if *n == 2.0 => self.call2("__c_mul", a, a),
                Expr::Num(n) if *n == 0.5 => self.call1("__c_sqrt", a),
                // `z^-1` and `z^-0.5` take "reciprocal / square root first", likewise bypassing
                // polar coordinates
                Expr::Num(n) if *n == -1.0 => {
                    let x = self.node(a, false);
                    let h = self.help("__c_div");
                    format!("{h}((1.0, 0.0), {x})")
                }
                Expr::Num(n) if *n == -0.5 => {
                    let x = self.node(a, false);
                    let s = self.help("__c_sqrt");
                    let d = self.help("__c_div");
                    format!("{d}((1.0, 0.0), {s}({x}))")
                }
                Expr::Num(n) => {
                    let x = self.node(a, false);
                    let h = self.help("__c_powf");
                    format!("{h}({x}, {n:?})")
                }
                _ => self.call2("__c_pow", a, b),
            },
            Expr::Call(name, args) => {
                if matches!(name.as_str(), "__quad_root_1" | "__quad_root_2") {
                    if args.len() != 3 {
                        return "compile_error!(\"内部二次根算子需要三个系数\")".to_string();
                    }
                    let a = self.node(&args[0], false);
                    let b = self.node(&args[1], false);
                    let c = self.node(&args[2], false);
                    for helper in [
                        "__c_add", "__c_sub", "__c_mul", "__c_neg", "__c_div", "__c_conj",
                        "__c_sqrt",
                    ] {
                        self.help(helper);
                    }
                    let helper = self.help("__c_quad_root");
                    let first = if name == "__quad_root_1" {
                        "true"
                    } else {
                        "false"
                    };
                    return format!("{helper}({a}, {b}, {c}, {first})");
                }
                let Some(arg0) = args.first() else {
                    return "compile_error!(\"函数调用缺参数\")".to_string();
                };
                // Expandable operators: the same strategy as the real lane — expand into an
                // ordinary expression on the IR first,
                // then emit. Without this step, when `emit_block_complex` is called directly (the
                // macro lane)
                // `det/solve/roots` fall through to the catch-all below and become compile_error,
                // while the generator lane happened to be fine because it ran expand_deep first —
                // the two lanes were inconsistent.
                if matches!(
                    name.as_str(),
                    "det"
                        | "trace"
                        | "adjoint"
                        | "inv"
                        | "solve"
                        | "roots"
                        | "poly_coeffs"
                        | "dot"
                        | "norm"
                ) {
                    return match expand_call(e) {
                        Some(x) => self.node(&x, false),
                        None => "compile_error!(\"det/trace/adjoint/inv/solve/roots/dot/norm 需要具体矩阵字面量\")".to_string(),
                    };
                }
                let helper = match name.as_str() {
                    "exp" => "__c_exp",
                    "ln" | "log" => "__c_ln",
                    "sqrt" => "__c_sqrt",
                    "sin" => "__c_sin",
                    "cos" => "__c_cos",
                    "tan" => "__c_tan",
                    "sinh" => "__c_sinh",
                    "cosh" => "__c_cosh",
                    "tanh" => "__c_tanh",
                    "abs" => "__c_abs",
                    "arg" => "__c_arg",
                    "conj" => "__c_conj",
                    "re" => "__c_re",
                    "im" => "__c_im",
                    // Unregistered function name: blow up at compile time, never leave a runtime
                    // surprise
                    _ => {
                        return "compile_error!(\"复数车道不支持该函数（exp/ln/sqrt/sin/cos/tan/sinh/cosh/tanh/abs/arg/conj/re/im）\")".to_string()
                    }
                };
                self.call1(helper, arg0)
            }
            // Σ with constant bounds: expand at compile time into a __c_add chain (the same
            // strategy as the real lane)
            Expr::Sum {
                var,
                lower,
                upper,
                body,
            } => {
                if let (Expr::Num(lo), Expr::Num(hi)) = (lower.as_ref(), upper.as_ref()) {
                    if math::fract(*lo) == 0.0 && math::fract(*hi) == 0.0 && (hi - lo) < 64.0 {
                        let mut parts = Vec::new();
                        let mut k = *lo;
                        while k <= *hi {
                            let b = subst(body, var, &num(k));
                            parts.push(self.node(&b, false));
                            k += 1.0;
                        }
                        // Empty interval (lo > hi) = the zero element, consistent with the value
                        // layer/real lane
                        if parts.is_empty() {
                            return "(0.0, 0.0)".to_string();
                        }
                        let mut acc = parts.remove(0);
                        for p in parts {
                            let h = self.help("__c_add");
                            acc = format!("{h}({acc}, {p})");
                        }
                        return acc;
                    }
                }
                "compile_error!(\"非常量求和界限请走 .formula 文件车道\")".to_string()
            }
            Expr::Integral { .. } => {
                "compile_error!(\"定积分请走 .formula 文件车道（数值积分是运行期循环）\")"
                    .to_string()
            }
            // Complex matrix: elements go through the same complex emitter and are wrapped as
            // `vec![vec![(re, im), …], …]`.
            // The generated function's return type is `Vec<Vec<(f64, f64)>>` (see buildhelper).
            // The solution of a "complex linear system" such as AC circuits / EIS is a complex
            // column vector — the value layer
            // always supported it, while the emission layer used to reject it wholesale; the two
            // sides were inconsistent.
            Expr::Mat(rows) => {
                let mut parts: Vec<String> = Vec::with_capacity(rows.len());
                for r in rows {
                    let mut cols: Vec<String> = Vec::with_capacity(r.len());
                    for cell in r {
                        cols.push(self.node(cell, false));
                    }
                    parts.push(format!("vec![{}]", cols.join(", ")));
                }
                format!("vec![{}]", parts.join(", "))
            }
            // Transpose: a plain transpose **does not conjugate** (conjugation is a separate
            // `conj`, see the operator table in the README).
            // Only matrices that can be flattened into a literal can be transposed; other shapes are
            // rejected explicitly.
            Expr::Transpose(a) => match flatten_matrix_value(a) {
                Some(m) if !m.is_empty() && m.iter().all(|r| r.len() == m[0].len()) => {
                    let (nr, nc) = (m.len(), m[0].len());
                    let mut parts: Vec<String> = Vec::with_capacity(nc);
                    for j in 0..nc {
                        let mut cols: Vec<String> = Vec::with_capacity(nr);
                        for row in &m {
                            cols.push(self.node(&row[j], false));
                        }
                        parts.push(format!("vec![{}]", cols.join(", ")));
                    }
                    format!("vec![{}]", parts.join(", "))
                }
                _ => "compile_error!(\"复数车道的转置需要形状编译期已知的矩阵字面量\")".to_string(),
            },
            Expr::MatPattern(_) => {
                "compile_error!(\"复数车道不做模式矩阵（先 `recognize_pattern` 落到具体形状）\")"
                    .to_string()
            }
            Expr::Deriv(..) | Expr::Grad(_) | Expr::Laplacian(_) | Expr::Indexed { .. } => {
                "compile_error!(\"符号算子/索引符号请走 .formula 文件车道\")".to_string()
            }
        }
    }
}

/// Emission of a complex block (equivalent to `emit_block`, but values are `(f64, f64)`).
///
/// The generated block contains only the closures **actually used**, so it passes
/// `clippy -D warnings` directly.
pub fn emit_block_complex(root: &Expr) -> String {
    emit_block_complex_with(root, &[])
}

/// Complex block emission (with the information "which names are already complex").
///
/// The macro lane expands federated formulas into a `let` chain, where later entries reference the
/// earlier bindings directly —
/// at that point `Var(y)` is already `(f64, f64)` and must not be wrapped in another `(y, 0.0)`.
pub fn emit_block_complex_with(root: &Expr, complex_vars: &[String]) -> String {
    let mut counts: HashMap<String, usize> = HashMap::new();
    count_nodes(root, &mut counts);
    let mut em = CEmit {
        counts,
        names: HashMap::new(),
        helpers: BTreeSet::new(),
        temps: Vec::new(),
        counter: 0,
        cx_vars: complex_vars.iter().cloned().collect(),
    };
    let body = em.node(root, true);
    let mut s = String::from("{\n");
    for h in &em.helpers {
        let line = complex_helper(h);
        for l in line.lines() {
            let _ = writeln!(s, "        {l}");
        }
    }
    for t in &em.temps {
        let _ = writeln!(s, "        {t}");
    }
    let _ = writeln!(s, "        {body}");
    s.push_str("    }");
    s
}

fn count_nodes(e: &Expr, counts: &mut HashMap<String, usize>) {
    *counts.entry(key(e)).or_insert(0) += 1;
    match e {
        Expr::Num(_) | Expr::Var(_) => {}
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
            count_nodes(a, counts);
            count_nodes(b, counts);
        }
        Expr::Neg(a) => count_nodes(a, counts),
        Expr::Call(_, args) => {
            for a in args {
                count_nodes(a, counts);
            }
        }
        Expr::Mat(rows) => {
            for r in rows {
                for c in r {
                    count_nodes(c, counts);
                }
            }
        }
        Expr::MatPattern(p) => {
            for e in p.elements() {
                count_nodes(e, counts);
            }
        }
        Expr::Transpose(a) => count_nodes(a, counts),
        Expr::Deriv(u, _) | Expr::Grad(u) | Expr::Laplacian(u) => count_nodes(u, counts),
        Expr::Sum {
            lower, upper, body, ..
        }
        | Expr::Integral {
            lower, upper, body, ..
        } => {
            count_nodes(lower, counts);
            count_nodes(upper, counts);
            count_nodes(body, counts);
        }
        Expr::Indexed { name, indices } => {
            *counts.entry(format!("${name}")).or_insert(0) += 1;
            let _ = indices;
        }
    }
}

fn emit_node(
    e: &Expr,
    is_root: bool,
    counts: &HashMap<String, usize>,
    names: &mut HashMap<String, String>,
    out: &mut Vec<String>,
    counter: &mut usize,
    fx: Fx,
) -> (String, u8) {
    let k = key(e);
    let shareable = !is_root
        && counts.get(&k).copied().unwrap_or(0) >= 2
        && !matches!(e, Expr::Num(_) | Expr::Var(_));
    if shareable {
        if let Some(n) = names.get(&k) {
            return (n.clone(), 9);
        }
    }
    let (text, p) = emit_uncached(e, counts, names, out, counter, fx);
    if shareable {
        let name = format!("__t{counter}");
        *counter += 1;
        out.push(format!("let {name} = {text};"));
        names.insert(k, name.clone());
        return (name, 9);
    }
    (text, p)
}

/// Whether it **might** be matrix-valued (unknown shape counts). Used to reject the misjudgment of
/// "scalar broadcasting":
/// only a side that is definitely not a matrix may be multiplied in as a scalar.
fn matrix_ish(e: &Expr) -> bool {
    match e {
        Expr::Mat(_) | Expr::MatPattern(_) | Expr::Transpose(_) | Expr::Indexed { .. } => true,
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
            matrix_ish(a) || matrix_ish(b)
        }
        Expr::Neg(a) => matrix_ish(a),
        Expr::Call(_, args) => args.iter().any(matrix_ish),
        _ => false,
    }
}

/// Matrix product (element-level IR): `C_ij = Σ_k A_ik·B_kj`. Returns None on a shape mismatch.
fn mat_mul_exprs(ma: &[Vec<Expr>], mb: &[Vec<Expr>]) -> Option<Vec<Vec<Expr>>> {
    let inner = ma.first()?.len();
    if mb.len() != inner {
        return None;
    }
    let cols = mb.first()?.len();
    let mut out: Vec<Vec<Expr>> = Vec::with_capacity(ma.len());
    for row in ma {
        let mut r2: Vec<Expr> = Vec::with_capacity(cols);
        // The column index `j` also indexes `mb[k][j]` (k varies in the inner loop) — not a
        // single-set iteration
        #[allow(clippy::needless_range_loop)]
        for j in 0..cols {
            let mut acc: Option<Expr> = None;
            for (k, x) in row.iter().enumerate() {
                let prod = mul(x.clone(), mb[k][j].clone());
                acc = Some(match acc {
                    None => prod,
                    Some(p) => add(p, prod),
                });
            }
            r2.push(acc?);
        }
        out.push(r2);
    }
    Some(out)
}

/// Fold a **statically expandable** matrix expression into a concrete matrix IR
/// (element level).
///
/// Only shapes whose leaves are all concrete literals are accepted: the size of a symbolic matrix
/// is unknown at compile time,
/// and that is the job of pattern matrices + the generator lane. If it cannot be folded, returns
/// None and leaves it to the generic emitter
/// to produce a **loud compile error** (better a compile failure than a silent miscalculation).
pub fn flatten_matrix_value(e: &Expr) -> Option<Vec<Vec<Expr>>> {
    fn same_shape(a: &[Vec<Expr>], b: &[Vec<Expr>]) -> bool {
        a.len() == b.len()
            && a.first().map(|r| r.len()) == b.first().map(|r| r.len())
            && a.iter().zip(b.iter()).all(|(x, y)| x.len() == y.len())
    }
    fn zip(
        a: Vec<Vec<Expr>>,
        b: Vec<Vec<Expr>>,
        f: fn(Expr, Expr) -> Expr,
    ) -> Option<Vec<Vec<Expr>>> {
        if !same_shape(&a, &b) {
            return None;
        }
        Some(
            a.into_iter()
                .zip(b)
                .map(|(ra, rb)| ra.into_iter().zip(rb).map(|(x, y)| f(x, y)).collect())
                .collect(),
        )
    }
    let scale = |m: Vec<Vec<Expr>>, k: Expr| -> Vec<Vec<Expr>> {
        m.into_iter()
            .map(|r| r.into_iter().map(|x| mul(k.clone(), x)).collect())
            .collect()
    };
    match e {
        Expr::Mat(rows) => Some(rows.clone()),
        Expr::Transpose(a) => {
            let m = flatten_matrix_value(a)?;
            let cols = m.first()?.len();
            if m.iter().any(|r| r.len() != cols) {
                return None;
            }
            Some(
                (0..cols)
                    .map(|j| m.iter().map(|r| r[j].clone()).collect())
                    .collect(),
            )
        }
        Expr::Neg(a) => {
            let m = flatten_matrix_value(a)?;
            Some(
                m.into_iter()
                    .map(|r| r.into_iter().map(neg).collect())
                    .collect(),
            )
        }
        // Matrix / scalar: element-wise division (matrix division is itself rejected explicitly at
        // the value layer, so the divisor
        // can only be a scalar). **Missing this branch once let the matrix guard miss**
        // `([1,2;3,4]/2)*x` —
        // it could not tell it was a matrix, so it emitted code like `f64 / Vec<Vec<f64>>` that does
        // not compile.
        Expr::Div(a, b) => {
            if matrix_ish(b) {
                return None;
            }
            let m = flatten_matrix_value(a)?;
            Some(
                m.into_iter()
                    .map(|r| r.into_iter().map(|x| div(x, (**b).clone())).collect())
                    .collect(),
            )
        }
        // Matrix operators: only the results of adjoint / inv are still matrices (det/trace/norm/dot
        // fold to scalars)
        Expr::Call(name, _) if name == "adjoint" || name == "inv" => {
            flatten_matrix_value(&expand_matrix_call(e)?)
        }
        // The result of solving a system is a column vector (single-variable solve is a scalar and
        // falls back to None after expansion)
        Expr::Call(name, _) if name == "solve" || name == "roots" || name == "poly_coeffs" => {
            flatten_matrix_value(&expand_call(e)?)
        }
        Expr::Add(a, b) => zip(flatten_matrix_value(a)?, flatten_matrix_value(b)?, add),
        Expr::Sub(a, b) => zip(flatten_matrix_value(a)?, flatten_matrix_value(b)?, sub),
        Expr::Mul(a, b) => match (flatten_matrix_value(a), flatten_matrix_value(b)) {
            (Some(ma), Some(mb)) => mat_mul_exprs(&ma, &mb),
            // Only a side that is definitely not a matrix may broadcast as a scalar
            (Some(ma), None) if !matrix_ish(b) => Some(scale(ma, (**b).clone())),
            (None, Some(mb)) if !matrix_ish(a) => Some(scale(mb, (**a).clone())),
            _ => None,
        },
        // Non-negative integer matrix powers: power 0 gives the identity of the same order,
        // 1..=8 repeated matrix multiplications;
        // negative powers (need the inverse) and fractional powers (need eigendecomposition) are
        // explicitly unsupported.
        Expr::Pow(a, b) => match b.as_ref() {
            Expr::Num(k) if *k == 0.0 => {
                let m = flatten_matrix_value(a)?;
                if !m.iter().all(|r| r.len() == m.len()) {
                    return None;
                }
                // The zero power of a matrix is the identity of the same order, not the scalar 1.
                // The type must still be a matrix too.
                Some(
                    (0..m.len())
                        .map(|i| {
                            (0..m.len())
                                .map(|j| num(if i == j { 1.0 } else { 0.0 }))
                                .collect()
                        })
                        .collect(),
                )
            }
            Expr::Num(k) if *k >= 1.0 && math::fract(*k) == 0.0 && *k <= 8.0 => {
                let m = flatten_matrix_value(a)?;
                if !m.iter().all(|r| r.len() == m.len()) {
                    return None;
                }
                let mut acc = m.clone();
                for _ in 1..(*k as usize) {
                    acc = mat_mul_exprs(&acc, &m)?;
                }
                Some(acc)
            }
            _ => None,
        },
        _ => None,
    }
}

/// Whether the expression contains a given free variable.
fn has_var(e: &Expr, var: &str) -> bool {
    vars_of(e).iter().any(|v| v == var)
}

/// Decompose `e` with respect to `var` into `a·var + b` (neither `a` nor `b` contains `var`).
///
/// Only **linear** expressions are accepted: `var*var`, `var^2`, `sin(var)`, `1/var` all error
/// with an explanation.
/// Silently returning a wrong coefficient is worse than guessing wrong — an error in the steady-state
/// concentration gets taken as the true value downstream.
fn lin_coeffs(e: &Expr, var: &str) -> Result<(Expr, Expr), Error> {
    use Expr as E;
    let zero = || num(0.0);
    Ok(match e {
        E::Num(_) | E::Indexed { .. } => (zero(), e.clone()),
        E::Var(n) => {
            if n == var {
                (num(1.0), zero())
            } else {
                (zero(), e.clone())
            }
        }
        E::Add(x, y) => {
            let (a1, b1) = lin_coeffs(x, var)?;
            let (a2, b2) = lin_coeffs(y, var)?;
            (add(a1, a2), add(b1, b2))
        }
        E::Sub(x, y) => {
            let (a1, b1) = lin_coeffs(x, var)?;
            let (a2, b2) = lin_coeffs(y, var)?;
            (sub(a1, a2), sub(b1, b2))
        }
        E::Neg(x) => {
            let (a, b) = lin_coeffs(x, var)?;
            (neg(a), neg(b))
        }
        E::Mul(x, y) => match (has_var(x, var), has_var(y, var)) {
            (false, false) => (zero(), e.clone()),
            (true, false) => {
                let (a, b) = lin_coeffs(x, var)?;
                (mul(a, (**y).clone()), mul(b, (**y).clone()))
            }
            (false, true) => {
                let (a, b) = lin_coeffs(y, var)?;
                (mul((**x).clone(), a), mul((**x).clone(), b))
            }
            (true, true) => {
                return Err(Error::NonlinearEquation {
                    name: var.to_string(),
                    why: "the unknown appears twice in one product".to_string(),
                })
            }
        },
        E::Div(x, y) => {
            if has_var(y, var) {
                return Err(Error::NonlinearEquation {
                    name: var.to_string(),
                    why: "the unknown appears in a denominator".to_string(),
                });
            }
            let (a, b) = lin_coeffs(x, var)?;
            (div(a, (**y).clone()), div(b, (**y).clone()))
        }
        E::Pow(x, y) => {
            // The exponent side may also hide the unknown (`2^x`): checking only the base was a
            // historical blind spot —
            // back then the whole term was taken as a constant term, the "solution" still carried x,
            // and once generated it became
            // a legal function "taking the solved-for quantity as an input" that silently returned
            // wrong values.
            if has_var(y, var) {
                return Err(Error::NonlinearEquation {
                    name: var.to_string(),
                    why: "it appears in an exponent".into(),
                });
            }
            if !has_var(x, var) {
                (zero(), e.clone())
            } else if matches!(y.as_ref(), E::Num(n) if *n == 1.0) {
                lin_coeffs(x, var)?
            } else {
                return Err(Error::NonlinearEquation {
                    name: var.to_string(),
                    why: "the unknown appears in a power or exponent".to_string(),
                });
            }
        }
        E::Call(name, args) => {
            // Check **all** arguments: x hidden in the second argument of `clamp(y, x, 10)` also
            // counts as non-linear
            // (once only the first was checked, so x leaked into the "solution" and the generator
            // listed it back in the parameter list)
            if args.iter().any(|a| has_var(a, var)) {
                return Err(Error::NonlinearEquation {
                    name: var.to_string(),
                    why: format!("the unknown appears inside `{name}`"),
                });
            }
            (zero(), e.clone())
        }
        _ => {
            if has_var(e, var) {
                return Err(Error::NonlinearEquation {
                    name: var.to_string(),
                    why: "this construct is not supported by the linear solver".into(),
                });
            }
            (zero(), e.clone())
        }
    })
}

/// Linear solving: `solve(expr, x)` → the `x` satisfying `expr = 0` (`expr` must be linear in x).
///
/// This is the entry point for chemistry/circuit "steady state" style problems:
/// `k_on·E·S − (k_off + k_cat)·ES = 0`
/// solved for `ES` is the Michaelis complex concentration. The solved expression is still IR, so
/// it can continue through dimension checking,
/// evaluation, code generation and differentiation — no second mechanism is needed.
pub fn solve_linear(e: &Expr, var: &str) -> Result<Expr, Error> {
    let (a, b) = lin_coeffs(e, var)?;
    // Simplify the coefficients first: the decomposition leaves shapes like
    // `Sub(Mul(M, 1.0), 0.0)`,
    // which the matrix predicate does not recognize (flatten only recognizes pure matrix
    // structures); only after simplification is it M itself.
    let (a, b) = (simplify(&a), simplify(&b));
    // The coefficients are matrix-valued (`solve(M*x - b, x)`): the solution has the shape
    // Div(matrix, matrix),
    // and the scalar emission lane would treat it as a vec! division and generate code that does not
    // compile — reject explicitly and point to
    // the correct form (rejection is a feature).
    // The test is "contains a matrix literal" rather than just `is_matrix_value`: for a form like
    // `1/Mat` with the **matrix in the
    // divisor position**, `flatten_matrix_value` deliberately returns None (matrix division is
    // unsupported),
    // so `is_matrix_value` is false, and the guard once missed it and returned a matrix-containing
    // "solution".
    fn has_matrix_literal(e: &Expr) -> bool {
        match e {
            Expr::Mat(_) | Expr::MatPattern(_) => true,
            Expr::Add(x, y)
            | Expr::Sub(x, y)
            | Expr::Mul(x, y)
            | Expr::Div(x, y)
            | Expr::Pow(x, y) => has_matrix_literal(x) || has_matrix_literal(y),
            Expr::Neg(x) | Expr::Transpose(x) => has_matrix_literal(x),
            _ => false,
        }
    }
    if is_matrix_value(&a)
        || is_matrix_value(&b)
        || has_matrix_literal(&a)
        || has_matrix_literal(&b)
    {
        return Err(Error::MatrixCoefficient(
            "`solve` solves scalar equations only; write matrix equations as `inv(A) * b`, or pass a list of scalar equations".into(),
        ));
    }
    // a identically zero ⇒ var is not in the equation at all (identity or contradiction). Simplify
    // first:
    // the var coefficient inside `a + b` is a foldable shape like `0 + 0`.
    if let Expr::Num(v) = simplify(&a) {
        if v == 0.0 {
            return Err(Error::NonlinearEquation {
                name: var.to_string(),
                why: "the equation has no linear term in it".into(),
            });
        }
    }
    Ok(simplify(&neg(div(b, a))))
}

/// Split the expression into "signed terms": `(sign, term)`, where a term has no top-level
/// addition/subtraction.
fn signed_terms(e: &Expr, sign: f64, out: &mut Vec<(f64, Expr)>) {
    match e {
        Expr::Add(a, b) => {
            signed_terms(a, sign, out);
            signed_terms(b, sign, out);
        }
        Expr::Sub(a, b) => {
            signed_terms(a, sign, out);
            signed_terms(b, -sign, out);
        }
        Expr::Neg(a) => signed_terms(a, -sign, out),
        _ => out.push((sign, e.clone())),
    }
}

/// Split a term into "numeric coefficient × multiset of factors", with the factors sorted by
/// their canonical key.
///
/// Only multiplication/division/negation/integer powers get a **structural** expansion;
/// addition/subtraction does not enter (they delimit terms).
/// Forms that cannot be split (symbolic divisors, non-integer powers) **do not return None**;
/// instead the whole term becomes one atomic
/// factor key — identical expressions still share a key (so `x/k − x/k = 0` is provable), while
/// different writings get different keys
/// (`sqrt(x)` and `x^0.5` cannot be proven equal; that is the conservative direction, which never
/// wrongly judges something as zero).
fn term_factors(e: &Expr) -> Option<(f64, Vec<String>)> {
    // Use the pure functional style "return (coefficient, factor key list)" rather than push/pop
    // into a shared buffer:
    // the latter produced a real bug with powers — `e^n` pushed only n−1 factors, so `e^n` and
    // `e^(n−1)` got the same
    // multiset and `a^2 − a` was judged identically zero (the solver then silently returned a wrong
    // solution).
    fn go(e: &Expr) -> Option<(f64, Vec<String>)> {
        match e {
            Expr::Num(v) => Some((*v, Vec::new())),
            Expr::Neg(a) => go(a).map(|(c, k)| (-c, k)),
            Expr::Mul(a, b) => {
                let (ca, mut ka) = go(a)?;
                let (cb, kb) = go(b)?;
                ka.extend(kb);
                Some((ca * cb, ka))
            }
            // Division accepts only a **numeric divisor**: a symbolic divisor cannot be split into
            // a factor multiset, so conservatively take the whole term as one atom
            Expr::Div(a, b) => match b.as_ref() {
                Expr::Num(d) if *d != 0.0 => go(a).map(|(c, k)| (c / d, k)),
                _ => Some((1.0, vec![key(e)])),
            },
            Expr::Pow(a, b) => match b.as_ref() {
                // Integer power: the base factors are **repeated n times as a whole**, and the
                // numeric coefficient is raised to the n-th power
                Expr::Num(n) if *n >= 0.0 && math::fract(*n) == 0.0 && *n <= 8.0 => {
                    let (cb, kb) = go(a)?;
                    let mut keys = Vec::with_capacity(kb.len() * (*n as usize));
                    for _ in 0..(*n as usize) {
                        keys.extend(kb.iter().cloned());
                    }
                    Some((math::powi(cb, *n as i32), keys))
                }
                // Non-integer powers (`x^0.5`, `x^(-1)`) are not split: the whole term is one atom.
                // Identical expressions still share a key (so `x^0.5 − x^0.5 = 0` is provable),
                // but it is a different key from `sqrt(x)` — that is merely **unprovable**, not a
                // wrong judgment.
                _ => Some((1.0, vec![key(e)])),
            },
            _ => Some((1.0, vec![key(e)])),
        }
    }
    let (coef, mut keys) = go(e)?;
    keys.sort();
    Some((coef, keys))
}

/// **Structural zero test**: expressions that are algebraically identically zero and provable as
/// such by "combining like terms".
///
/// Why it is needed: `simplify` only folds constants and 0/1 identities and does **not combine like
/// terms**.
/// So `k − k`, `k·a − a·k`, `2x − x − x` cannot be folded, and the solver treated "cannot be
/// folded"
/// as "non-zero", taking rows with zero coefficients as pivots — the consequence is `0/0`: the
/// evaluation layer errors,
/// the emission layer silently gives NaN, or an underdetermined system is taken as a unique
/// solution.
///
/// Implementation: split into signed terms → split each term into "numeric coefficient × sorted
/// factor multiset" →
/// add the coefficients sharing a key. The criterion is **sufficient**, not necessary: if zero
/// cannot be proven, return false (conservative).
pub fn is_structurally_zero(e: &Expr) -> bool {
    let s = simplify(e);
    if matches!(s, Expr::Num(v) if v == 0.0) {
        return true;
    }
    let mut terms: Vec<(f64, Expr)> = Vec::new();
    signed_terms(&s, 1.0, &mut terms);
    let mut acc: BTreeMap<String, f64> = BTreeMap::new();
    for (sign, t) in terms {
        // `term_factors` always gives Some (unsplittable means an atom), so there is no "give up"
        // branch here
        let Some((coef, keys)) = term_factors(&t) else {
            return false;
        };
        let k = keys.join("\u{1}");
        *acc.entry(k).or_insert(0.0) += sign * coef;
    }
    acc.values().all(|v| *v == 0.0)
}

/// Multivariate linear coefficient decomposition: `e = Σ_j a_j·v_j + b` (no `a_j` and no `b`
/// contains any `v_j`).
///
/// Semantics are the same as `lin_coeffs`, except the unknowns go from one to a group — the
/// coefficient source for solving systems of equations.
/// Non-linearity errors must state clearly **which operation** the problem occurred in, the same
/// discipline as the single-variable version.
// The coefficient vector is located by unknown index (the k-th item is the k-th unknown); this is
// a semantic index, not an iteration
#[allow(clippy::needless_range_loop)]
fn lin_coeffs_multi(e: &Expr, vars: &[String]) -> Result<(Vec<Expr>, Expr), Error> {
    use Expr as E;
    let n = vars.len();
    let zeros = || vec![num(0.0); n];
    let hit = |x: &Expr| vars.iter().any(|v| has_var(x, v));
    Ok(match e {
        E::Num(_) | E::Indexed { .. } => (zeros(), e.clone()),
        E::Var(name) => match vars.iter().position(|v| v == name) {
            Some(k) => {
                let mut a = zeros();
                a[k] = num(1.0);
                (a, num(0.0))
            }
            None => (zeros(), e.clone()),
        },
        E::Add(x, y) => {
            let (mut a, b) = lin_coeffs_multi(x, vars)?;
            let (a2, b2) = lin_coeffs_multi(y, vars)?;
            for k in 0..n {
                a[k] = add(a[k].clone(), a2[k].clone());
            }
            (a, add(b, b2))
        }
        E::Sub(x, y) => {
            let (mut a, b) = lin_coeffs_multi(x, vars)?;
            let (a2, b2) = lin_coeffs_multi(y, vars)?;
            for k in 0..n {
                a[k] = sub(a[k].clone(), a2[k].clone());
            }
            (a, sub(b, b2))
        }
        E::Neg(x) => {
            let (mut a, b) = lin_coeffs_multi(x, vars)?;
            for k in 0..n {
                a[k] = neg(a[k].clone());
            }
            (a, neg(b))
        }
        E::Mul(x, y) => match (hit(x), hit(y)) {
            (false, false) => (zeros(), e.clone()),
            (true, true) => {
                return Err(Error::NonlinearEquation {
                    name: "the product".into(),
                    why: "both factors contain the unknown".into(),
                })
            }
            // The pure-coefficient side multiplies into every unknown's coefficient and the
            // constant term
            (true, false) => {
                let (mut a, b) = lin_coeffs_multi(x, vars)?;
                for k in 0..n {
                    a[k] = mul(a[k].clone(), (**y).clone());
                }
                (a, mul(b, (**y).clone()))
            }
            (false, true) => {
                let (mut a, b) = lin_coeffs_multi(y, vars)?;
                for k in 0..n {
                    a[k] = mul((**x).clone(), a[k].clone());
                }
                (a, mul((**x).clone(), b))
            }
        },
        E::Div(x, y) => {
            if hit(y) {
                return Err(Error::NonlinearEquation {
                    name: "the equation".into(),
                    why: "the unknown appears in a denominator".into(),
                });
            }
            let (mut a, b) = lin_coeffs_multi(x, vars)?;
            for k in 0..n {
                a[k] = div(a[k].clone(), (**y).clone());
            }
            (a, div(b, (**y).clone()))
        }
        E::Pow(x, y) => {
            // An unknown hidden in the exponent side (`2^x`) must be **rejected explicitly**: it
            // used to be taken as a constant term, with a comment saying
            // "it will be conservatively rejected at the rank check" — as long as the same unknown
            // also appeared as a
            // linear term in that equation it slipped through, and the solved expression did not
            // satisfy the original equation at all.
            if hit(y) {
                return Err(Error::NonlinearEquation {
                    name: "the equation".into(),
                    why: "the unknown appears in an exponent".into(),
                });
            }
            if !hit(x) {
                (zeros(), e.clone())
            } else if matches!(y.as_ref(), E::Num(m) if *m == 1.0) {
                lin_coeffs_multi(x, vars)?
            } else {
                return Err(Error::NonlinearEquation {
                    name: "the equation".into(),
                    why: "the unknown appears in a power".into(),
                });
            }
        }
        E::Call(name, args) => {
            // Check **all** arguments: x hidden in the second argument of `clamp(y, x, 10)` also
            // counts as non-linear
            if args.iter().any(&hit) {
                return Err(Error::NonlinearEquation {
                    name: name.clone(),
                    why: "the unknown appears inside a function argument".to_string(),
                });
            }
            (zeros(), e.clone())
        }
        _ => {
            if hit(e) {
                return Err(Error::NonlinearEquation {
                    name: "the equation".into(),
                    why: "this construct is not supported by the linear solver".into(),
                });
            }
            (zeros(), e.clone())
        }
    })
}

/// Shape convention for equation / unknown lists: a matrix literal is **flattened** into entries
/// (row-major).
/// `[e1, e2]` and `[e1; e2]` are the same set of equations — the user need not remember the
/// direction.
fn flat_mat_items(e: &Expr) -> Option<Vec<Expr>> {
    match e {
        Expr::Mat(rows) => {
            let items: Vec<Expr> = rows.iter().flatten().cloned().collect();
            if items.is_empty() {
                None
            } else {
                Some(items)
            }
        }
        _ => None,
    }
}

/// Unknown list: after flattening, every entry must be a bare variable name.
fn mat_var_names(e: &Expr) -> Option<Vec<String>> {
    let mut out = Vec::new();
    for c in flat_mat_items(e)? {
        match c {
            Expr::Var(n) => out.push(n),
            _ => return None,
        }
    }
    Some(out)
}

/// Linear system solving: `solve([eq1, eq2, …], [x1, x2, …])`, each equation written as
/// "expression = 0".
///
/// Gauss–Jordan elimination runs over the **expression ring**: the coefficients are IR
/// subexpressions rather than numbers, and `simplify` normalizes after every row operation. It
/// returns the solution vector (column-matrix IR) — eval / emission / dimensions / CSE all go
/// through
/// the existing lanes, with no second mechanism.
///
/// Only full rank gives a unique solution: underdetermined and contradictory systems are rejected
/// outright here. Removal of redundant equations is **in the foldable sense**:
/// if a redundant row with symbolic coefficients cannot be simplified to zero, a "contradiction"
/// is reported conservatively (better to refuse than to give a wrong solution).
/// Pivot non-zeroness follows the same line — if zero cannot be proven, treat it as non-zero
/// (consistent with the single-variable solve trusting `a ≠ 0`).
// The Gaussian elimination loops are all row/column semantics (r/c serve as indices and for bounds
// checks), not set iteration
#[allow(clippy::needless_range_loop)]
pub fn solve_system(eqs: &[Expr], vars: &[String]) -> Result<Expr, Error> {
    if vars.is_empty() {
        return Err(Error::InvalidInput(
            "the list of unknowns is empty; at least one is required".into(),
        ));
    }
    if eqs.is_empty() {
        return Err(Error::InvalidInput(
            "the list of equations is empty; at least one is required".into(),
        ));
    }
    let mut seen = BTreeSet::new();
    for v in vars {
        if !seen.insert(v.as_str()) {
            return Err(Error::InvalidInput(format!(
                "the unknown `{v}` appears twice in the list"
            )));
        }
    }
    let (m, n) = (eqs.len(), vars.len());
    // Augmented matrix [A | −b]: eq_i = Σ_j a_ij·v_j + b_i = 0 ⟺ A·v = −b
    let mut aug: Vec<Vec<Expr>> = Vec::with_capacity(m);
    for (i, eq) in eqs.iter().enumerate() {
        // Propagate the typed error: wrapping it in a `String` here (with the equation index)
        // downgraded every catalogue variant back to `Message`.
        let (a, b) = lin_coeffs_multi(eq, vars)?;
        // Matrix-valued coefficients: the solution would be a matrix division, and the scalar lane
        // would generate code like
        // `f64 / Vec<Vec<f64>>` that does not compile — the same guard as the single-variable lane
        for (j, aj) in a.iter().enumerate() {
            if is_matrix_value(&simplify(aj)) {
                return Err(Error::MatrixCoefficient(vars[j].clone()));
            }
        }
        if is_matrix_value(&simplify(&b)) {
            // The constant term has no variable name; name it by role.
            return Err(Error::MatrixCoefficient(format!(
                "<constant of equation {}>",
                i + 1
            )));
        }
        let mut row = a;
        row.push(neg(b));
        aug.push(row);
    }
    // The zero test must use the **structural zero**: `simplify` does not combine like terms, so
    // `k − k` cannot be folded,
    // and taking it as a pivot yields 0/0 (an error at the evaluation layer / NaN at the emission
    // layer) or takes an underdetermined system as unique.
    let is_zero = |e: &Expr| is_structurally_zero(e);
    let mut pivots = 0usize;
    for col in 0..n {
        let Some(p) = (pivots..m).find(|&r| !is_zero(&aug[r][col])) else {
            return Err(Error::Underdetermined(vars[col].clone()));
        };
        aug.swap(pivots, p);
        // Normalize the pivot row: divide the whole row by the pivot (a symbolic expression can
        // serve as a denominator too).
        // Set the first element to 1 **explicitly**: `simplify` cannot fold a shape like `piv/piv`
        // (it does not combine
        // like terms), and leaving it would make later row operations compute fake non-zeros such as
        // `x − 1·x`.
        let piv = aug[pivots][col].clone();
        for c in (col + 1)..=n {
            aug[pivots][c] = simplify(&div(aug[pivots][c].clone(), piv.clone()));
        }
        aug[pivots][col] = num(1.0);
        // Eliminate this column from the other rows. Afterwards set that cell to zero
        // **explicitly** — this column is mathematically already
        // eliminated exactly, and relying on `simplify` to prove `a − a` is zero is unreliable.
        for r in 0..m {
            if r == pivots || is_zero(&aug[r][col]) {
                continue;
            }
            let f = aug[r][col].clone();
            for c in (col + 1)..=n {
                aug[r][c] = simplify(&sub(
                    aug[r][c].clone(),
                    mul(f.clone(), aug[pivots][c].clone()),
                ));
            }
            aug[r][col] = num(0.0);
        }
        pivots += 1;
    }
    // After the unknowns run out, the remaining equations must be identities: 0 = non-zero is a
    // contradiction
    for r in pivots..m {
        if !is_zero(&aug[r][n]) {
            return Err(Error::Inconsistent);
        }
    }
    // Row j is now exactly `1·v_j = aug[j][n]`
    let sols: Vec<Vec<Expr>> = (0..n).map(|j| vec![simplify(&aug[j][n].clone())]).collect();
    Ok(Expr::Mat(sols))
}

/// Coefficient vector of a polynomial (**low degree to high degree**):
/// `poly_coeffs_vec(poly, x)` → a `1×n` row vector.
///
/// Complementary to `roots`: `roots` gives the roots, this gives the coefficients — both
/// presuppose "a univariate polynomial in x",
/// and non-polynomial shapes are always rejected (no guessing).
///
/// Dimensions: each coefficient has dimension `dim(the k-th term of poly) / dim(x)^k`, so the
/// coefficients have **different dimensions**;
/// the result is returned through the **matrix-value lane**, where the matrix elements must share
/// a dimension — therefore the polynomial itself must be
/// **dimensionally homogeneous** (all terms share a dimension ⇒ the coefficients lose one power of
/// x's dimension per degree), and that is checked explicitly.
pub fn poly_coeffs_vec(e: &Expr, var: &str) -> Result<Expr, Error> {
    let coeffs = poly_coeffs(e, var).ok_or_else(|| {
        format!("`poly_coeffs` 需要关于 `{var}` 的一元多项式：`{var}` 出现在函数/分母/非整数幂里时不适用")
    })?;
    let n = coeffs.len();
    let mut trimmed = coeffs;
    while trimmed.len() > 1
        && matches!(simplify(&trimmed[trimmed.len() - 1]), Expr::Num(v) if v == 0.0)
    {
        trimmed.pop();
    }
    let out: Vec<Expr> = trimmed.into_iter().map(|c| simplify(&c)).collect();
    if out.len() == 1 && matches!(out[0], Expr::Num(v) if v == 0.0) {
        return Err(Error::InvalidInput(format!(
            "`poly_coeffs`: `{var}` is a zero polynomial, so there are no coefficients to report"
        )));
    }
    let _ = n;
    // Row vector (1×k): kept distinct from the column vector of `roots`, to avoid the shape
    // confusion of "coefficients taken for roots"
    Ok(Expr::Mat(vec![out]))
}

/// Polynomial root finding: `roots(poly, x)` → **all roots** satisfying `poly = 0` (column
/// vector).
///
/// Only up to degree 2: degree 1 is delegated to `solve_linear` (the same copy of the math),
/// degree 2 uses the closed-form root formula.
/// The Cardano/Ferrari closed forms for degree 3 and above have terrible conditioning in floating
/// point and nobody could read what is written anyway —
/// rather than giving a "seemingly exact approximation", reject explicitly and state the degree.
///
/// The dimension of the roots **automatically equals the dimension of `x`**: the equation must be
/// dimensionally homogeneous (`dim_check` checks the equation first in the Call branch;
/// `Σ aᵢxⁱ` homogeneous ⇒ `dim(aᵢ) + i·dim(x)` constant ⇒ the roots' dimension is dim(x)),
/// so no extra dimension derivation is needed here.
///
/// Multiple roots appear repeated by **multiplicity** (`x² − 2x + 1` gives two 1s), consistent with
/// the fundamental theorem of algebra.
///
/// The real emission lane returns NaN for a negative discriminant (not an exception); a constant
/// negative discriminant is rejected before emission.
/// The generic `eval_value` is the complex value layer and gives complex roots; the complex
/// generation lane requires the formula to contain `j` explicitly.
/// The two roots are computed with the stable q/a and c/q, and the return order still corresponds
/// to the original (−b−√D), (−b+√D).
pub fn solve_poly(e: &Expr, var: &str) -> Result<Expr, Error> {
    let coeffs = poly_coeffs(e, var).ok_or_else(|| {
        format!("`roots` 需要关于 `{var}` 的多项式：`{var}` 出现在函数/分母/非整数幂里时不适用")
    })?;
    // Drop high-degree zero coefficients: `x^3 - x^3 + x - 1` is still a degree-1 equation
    let mut c = coeffs;
    while c.len() > 1 && matches!(simplify(&c[c.len() - 1]), Expr::Num(v) if v == 0.0) {
        c.pop();
    }
    // Matrix-valued coefficients (`roots(M*x + b, x)`): the solution would be a matrix division, and
    // the scalar lane would generate
    // a `vec!` division that does not compile — the same line as `solve`: reject explicitly and
    // point to the correct form
    let mut normalized = Vec::with_capacity(c.len());
    for (k, coef) in c.iter().enumerate() {
        let cs = simplify(coef);
        if is_matrix_value(&cs) {
            return Err(Error::MatrixCoefficient(format!(
                "`roots`: the degree-{k} coefficient is matrix-valued; root finding handles scalar coefficients only"
            )));
        }
        normalized.push(cs);
    }
    c = normalized;
    match c.len() - 1 {
        0 => Err(Error::NonlinearEquation {
                name: var.to_string(),
                why: "the equation does not contain it, so there is nothing to solve for".into(),
            }),
        // Degree 1: `a·x + b = 0 → x = −b/a` (the same math as `solve_linear`).
        // Note that the **normalized coefficients** are used here rather than handing the original
        // expression to solve_linear: the original may
        // still hold high-degree terms that have already cancelled (`x³ − x³ + x − 1`), and
        // lin_coeffs would judge
        // `x³` as non-linear — even though it clearly cancels. The polynomial normalization path has
        // already peeled them off.
        1 => Ok(Expr::Mat(vec![vec![simplify(&neg(div(
            c[0].clone(),
            c[1].clone(),
        )))]])),
        2 => {
            let (a, b, c0) = (c[2].clone(), c[1].clone(), c[0].clone());
            // Using (−b±√D)/(2a) directly cancels catastrophically when b²≫4ac.
            // Instead use q = −½(b + sign(b)√D), with the large root = q/a and the small root = c/q;
            // the two internal scalar nodes keep the original (−b−√D), (−b+√D) return order.
            // q=0 only happens in degenerate forms such as a zero double root with b=c=0; both
            // evaluation and emission fall back to −b/(2a).
            let r1 = Expr::Call(
                "__quad_root_1".into(),
                vec![a.clone(), b.clone(), c0.clone()],
            );
            let r2 = Expr::Call("__quad_root_2".into(), vec![a, b, c0]);
            Ok(Expr::Mat(vec![vec![r1], vec![r2]]))
        }
        n => Err(Error::InvalidInput(format!(
                "`roots` solves degree 1–2 only (got degree {n}): closed forms above that are unstable in floating point, and this crate does not offer a look-exact approximation — reduce the degree first, or compute the coefficients numerically and use a numeric root finder"
            ))),
    }
}

/// Numerically stable quadratic roots (the value layer and both emission lanes share the same
/// selection rule).
///
/// `first=true` corresponds to `(−b−√D)/(2a)` in textbook order, `false` to the other root.
/// When choosing `q`, b is added to a signed √D of the same sign, preventing the large root from
/// being cancelled first; the small root uses c/q.
fn stable_quadratic_root(first: bool, a: C, b: C, c: C) -> Result<C, Error> {
    let d = b.mul(b).sub(C::real(4.0).mul(a).mul(c));
    let s = d.sqrt();
    // When Re(conj(b)·s) ≥ 0, q/a is the first root in the original order; otherwise it is the
    // second.
    let q_is_first = b.conj().mul(s).re >= 0.0;
    let signed_s = if q_is_first { s } else { s.neg() };
    let q = b.add(signed_s).mul(C::real(-0.5));
    if q.re == 0.0 && q.im == 0.0 {
        // b=c=0 (including a zero double root): c/q would become 0/0, but both roots are just
        // −b/(2a).
        return b.neg().div(C::real(2.0).mul(a));
    }
    let large = q.div(a)?;
    let small = c.div(q)?;
    Ok(if first == q_is_first { large } else { small })
}

/// **Deep expansion**: replace the expandable operators (det/trace/adjoint/inv/solve/roots/
/// dot/norm) **anywhere in the tree** with their expanded ordinary IR.
///
/// Why it is needed: `expand_call` only looks at the root node. In nested forms like
/// `2 * solve(...)`,
/// the solved-for unknown stays in the parameter list — generating a function
/// "asking the caller to pass the solved-for quantity"
/// while that parameter is not used in the function body at all (it gets by on
/// `#[allow(unused_variables)]`).
///
/// Termination: the expansion result cannot contain another Call of the same name (`solve`'s
/// solution is a Mat, `det` is a scalar expression),
/// so the recursion necessarily converges; but to be safe a depth limit is still set, returning the
/// input unchanged beyond it.
pub fn expand_deep(e: &Expr) -> Expr {
    fn go(e: &Expr, depth: usize) -> Expr {
        // The cap is aligned with the global depth limit (`check_depth` guarantees the input is ≤
        // MAX_DEPTH, so this
        // is in practice unreachable). It used to be 64: that meant **silently not expanding**, so a
        // `2 * solve(...)` inside a legal 70-level formula left the solved-for parameter behind — and
        // MAX_DEPTH is 2048.
        if depth > MAX_DEPTH {
            return e.clone();
        }
        // The root node is expandable → keep digging after expansion (the expansion may still hold
        // inner operators)
        if let Some(x) = expand_call(e) {
            return go(&x, depth + 1);
        }
        match e {
            Expr::Num(_) | Expr::Var(_) | Expr::Indexed { .. } => e.clone(),
            // The cells of a pattern matrix must be expanded too (cells containing operators must
            // not be left as they are)
            Expr::MatPattern(p) => Expr::MatPattern(MatPattern {
                rows: p
                    .rows
                    .iter()
                    .map(|r| {
                        r.iter()
                            .map(|c| match c {
                                PatternCell::El(x) => PatternCell::El(go(x, depth + 1)),
                                PatternCell::Dots => PatternCell::Dots,
                            })
                            .collect()
                    })
                    .collect(),
            }),
            Expr::Add(a, b) => add(go(a, depth + 1), go(b, depth + 1)),
            Expr::Sub(a, b) => sub(go(a, depth + 1), go(b, depth + 1)),
            Expr::Mul(a, b) => mul(go(a, depth + 1), go(b, depth + 1)),
            Expr::Div(a, b) => div(go(a, depth + 1), go(b, depth + 1)),
            Expr::Pow(a, b) => pow(go(a, depth + 1), go(b, depth + 1)),
            Expr::Neg(a) => neg(go(a, depth + 1)),
            Expr::Call(n, args) => {
                Expr::Call(n.clone(), args.iter().map(|a| go(a, depth + 1)).collect())
            }
            Expr::Mat(rows) => Expr::Mat(
                rows.iter()
                    .map(|r| r.iter().map(|c| go(c, depth + 1)).collect())
                    .collect(),
            ),
            Expr::Transpose(a) => transpose(go(a, depth + 1)),
            Expr::Deriv(u, by) => deriv(go(u, depth + 1), by.clone()),
            Expr::Grad(u) => grad(go(u, depth + 1)),
            Expr::Laplacian(u) => laplacian(go(u, depth + 1)),
            Expr::Sum {
                var,
                lower,
                upper,
                body,
            } => sum(
                var.clone(),
                go(lower, depth + 1),
                go(upper, depth + 1),
                go(body, depth + 1),
            ),
            Expr::Integral {
                var,
                lower,
                upper,
                body,
            } => integral(
                var.clone(),
                go(lower, depth + 1),
                go(upper, depth + 1),
                go(body, depth + 1),
            ),
        }
    }
    go(e, 0)
}

/// Expand the matrix operators (det / trace / adjoint / inv) on **concrete literals** into
/// ordinary IR.
///
/// All three lanes (evaluation/emission/symbolic differentiation) share this one expansion: once
/// `det` is expanded into a closed-form scalar expression,
/// it becomes an ordinary differentiable expression — far cheaper than "writing a separate chain
/// rule for det".
// The indexed loops here are all **row/column semantics** (i/j index rows and columns
// respectively), not set iteration
#[allow(clippy::needless_range_loop)]
pub fn expand_matrix_call(e: &Expr) -> Option<Expr> {
    let Expr::Call(name, args) = e else {
        return None;
    };
    if args.len() != 1 {
        return None;
    }
    let m = flatten_matrix_value(&args[0])?;
    let n = m.len();
    if n == 0 {
        return None;
    }
    // The square-matrix check is meaningful only for det/trace/adjoint/inv; `norm` takes
    // rectangular input (a vector is just 1×k)
    let square = |m: &[Vec<Expr>]| m.iter().all(|r| r.len() == m.len());
    match name.as_str() {
        "trace" => {
            if !square(&m) {
                return None;
            }
            Some(trace_of(&m))
        }
        "det" => {
            if !square(&m) {
                return None;
            }
            det_of(&m)
        }
        // Norm: `norm(v)` = √Σv² (Frobenius; for a vector it is the 2-norm)
        "norm" => {
            let mut acc: Option<Expr> = None;
            for row in &m {
                for x in row {
                    let sq = mul(x.clone(), x.clone());
                    acc = Some(match acc {
                        None => sq,
                        Some(a) => add(a, sq),
                    });
                }
            }
            Some(Expr::Call("sqrt".into(), vec![acc?]))
        }
        // adjoint = conjugate transpose. Conjugation is the identity in the real lane and `__c_conj`
        // in the complex lane,
        // so emitting conj directly here is correct for both lanes.
        "adjoint" => {
            #[allow(clippy::needless_range_loop)] // the column index j also indexes m[i][j]
            let cols = m.first().map(|r| r.len()).unwrap_or(0);
            let mut t: Vec<Vec<Expr>> = Vec::with_capacity(cols);
            for j in 0..cols {
                let mut row: Vec<Expr> = Vec::with_capacity(n);
                for i in 0..n {
                    row.push(Expr::Call("conj".into(), vec![m[i][j].clone()]));
                }
                t.push(row);
            }
            Some(Expr::Mat(t))
        }
        // Inverse: `M⁻¹ = adj(M)/det(M)`, where the (i,j) entry of the adjugate is
        // `(-1)^{i+j}·det(M_{ji})`.
        // Only up to order 3: order 4 needs 16 3×3 minors (~100 terms) and the generated code
        // explodes;
        // 2D/3D isoparametric transforms use exactly orders 2/3.
        "inv" => {
            if n > 3 || !square(&m) {
                return None;
            }
            let det = det_of(&m)?;
            let mut adj: Vec<Vec<Expr>> = Vec::with_capacity(n);
            for i in 0..n {
                let mut row: Vec<Expr> = Vec::with_capacity(n);
                for j in 0..n {
                    // Note the transpose: adj[i][j] uses minor(j, i)
                    let cof = det_of(&minor(&m, j, i))?;
                    let val = if (i + j) % 2 == 0 { cof } else { neg(cof) };
                    row.push(div(val, det.clone()));
                }
                adj.push(row);
            }
            Some(Expr::Mat(adj))
        }
        _ => None,
    }
}

fn trace_of(m: &[Vec<Expr>]) -> Expr {
    let mut acc = m[0][0].clone();
    for (i, row) in m.iter().enumerate().skip(1) {
        acc = add(acc, row[i].clone());
    }
    acc
}

/// The submatrix with row `i` and column `j` removed.
fn minor(m: &[Vec<Expr>], i: usize, j: usize) -> Vec<Vec<Expr>> {
    m.iter()
        .enumerate()
        .filter(|(r, _)| *r != i)
        .map(|(_, row)| {
            row.iter()
                .enumerate()
                .filter(|(c, _)| *c != j)
                .map(|(_, x)| x.clone())
                .collect()
        })
        .collect()
}

/// Determinant: cofactor expansion along the first row, recursing down to order 1.
///
/// Limit of order 4 (4×4 = 24 terms): higher-order expansions are both long and slow, and a
/// numerical library is more honest.
/// The expanded `det` is an **ordinary scalar expression**, so it is naturally differentiable (FEM
/// areas/volumes rely on this).
fn det_of(m: &[Vec<Expr>]) -> Option<Expr> {
    let n = m.len();
    if m.iter().any(|r| r.len() != n) || n > 4 {
        return None;
    }
    // Empty-product convention: the 0×0 determinant is 1 — that is the minor of `inv([[a]])`
    if n == 0 {
        return Some(num(1.0));
    }
    if n == 1 {
        return Some(m[0][0].clone());
    }
    let mut acc: Option<Expr> = None;
    for (j, cell) in m[0].iter().enumerate() {
        let sub = det_of(&minor(m, 0, j))?;
        let term = mul(cell.clone(), sub);
        let term = if j % 2 == 0 { term } else { neg(term) };
        acc = Some(match acc {
            None => term,
            Some(a) => add(a, term),
        });
    }
    acc
}

// ---------------------------------------------------------------- polynomials and Horner

/// Accepts **polynomials** only (in `var`): returns `coeffs[k]` = the coefficient of `var^k`.
///
/// Addition/subtraction, multiplication/division by constants, and **polynomial products**
/// (coefficient convolution) are supported —
/// expansions like `(a·x + b)(x + c)` are very common in physics formulas.
/// Numerical root finding (Durand–Kerner iteration): `coeffs` go from **low degree to high
/// degree** (`[c₀, c₁, …]`).
///
/// This is the exit for degree 3 and above: `roots` in the language only does closed forms up to
/// degree 2 (high-order closed forms have terrible conditioning in floating point), but
/// **numerical** root finding works for any degree, so it lives in the tool layer.
///
/// Complex numbers use `C` (no third-party numerics library); initial values are powers of
/// `0.4+0.9j`, scaled preferentially by the
/// geometric mean of the root moduli. DK is capped at 1000 rounds with relative step `1e-10`,
/// followed by at most 60 Newton steps to finish;
/// finally it is accepted independently by `|p(z)| / Σ|aₖ||z|ᵏ ≤ 1e-8`. **Both non-convergence and
/// an excessive residual are errors**.
///
/// The output only zeroes imaginary noise with `|im| ≤ 1e-12·|z|`; the real part is left alone,
/// to avoid wiping out small roots or complex roots.
pub fn numeric_roots(coeffs: &[f64]) -> Result<Vec<C>, Error> {
    let mut c = coeffs.to_vec();
    while c.len() > 1 && c[c.len() - 1] == 0.0 {
        c.pop();
    }
    if c.iter().all(|v| *v == 0.0) {
        return Err(Error::InvalidInput(
            "zero polynomial: every number is a root, so the solution set is not determined".into(),
        ));
    }
    if c.iter().any(|v| !v.is_finite()) {
        return Err(Error::NonFiniteValue(
            "the polynomial coefficients contain inf/NaN, so numeric root finding is impossible"
                .into(),
        ));
    }
    let n = c.len() - 1;
    match n {
        0 => {
            return Err(Error::InvalidInput(
                "a constant polynomial (degree 0) has no roots".into(),
            ))
        }
        1 => {
            // The quotient must be finite: even when each coefficient is finite, `c0/c1` can still
            // overflow to inf
            // (`numeric_roots(&[1e300, 1e-300])` → -inf). Returning Ok(root) amounts to
            // promising "this is a usable root", so the same standard applies here as on the
            // iteration path.
            let r = -c[0] / c[1];
            if !r.is_finite() {
                return Err(Error::NonFiniteValue(
            "the solution of the linear equation overflows (the coefficient ratio exceeds the f64 range); rescale the variable and retry".into(),
        ));
            }
            return Ok(vec![C::new(r, 0.0)]);
        }
        _ => {}
    }
    if n > 64 {
        return Err(Error::InvalidInput(format!(
            "degree {n} exceeds the numeric root-finding limit of 64"
        )));
    }
    let lead = c[n];
    let a: Vec<f64> = c.iter().map(|v| v / lead).collect(); // make monic, avoiding magnitude overflow
                                                            // Initial radius: try the "geometric mean of the root moduli" (|c₀/cₙ|^(1/n)) first; that is
                                                            // roughly where the roots are.
                                                            // It used to be 1+max|aᵢ| (the Cauchy bound) — for polynomials like ∏(x−k) whose "roots are far
                                                            // smaller than the coefficients",
                                                            // the initial values were too far from the roots, 500 iterations did not converge, and healthy
                                                            // polynomials were misreported as "ill-conditioned".
                                                            // There also used to be an absolute lower bound `geo > 1e-8` here — that pulled every polynomial
                                                            // "with roots below 1e-8"
                                                            // back to the Cauchy radius (≈1): `x²−1e-50` then started at 1 to look for roots of order 1e-25,
                                                            // DK stalled at the noise floor, 40 Newton steps could not rescue it, and it **silently returned
                                                            // wrong roots** (off by up to 1e125×).
                                                            // The criterion can only be "finite and non-zero", with no absolute threshold.
    let geo = if a[0] == 0.0 || !a[0].is_finite() {
        0.0
    } else {
        math::powf(a[0].abs(), 1.0 / n as f64)
    };
    let radius = if geo.is_finite() && geo > 0.0 {
        geo
    } else {
        1.0 + a[..n].iter().fold(0.0f64, |m, v| m.max(v.abs()))
    };
    // Initial values: z_k = radius · (0.4 + 0.9j)^k
    let seed = C::new(0.4, 0.9);
    let mut z: Vec<C> = Vec::with_capacity(n);
    let mut p = C::ONE;
    for _ in 0..n {
        z.push(C::new(p.re * radius, p.im * radius));
        p = p.mul(seed);
    }
    // The magnitude reference shared by the convergence criterion and the Newton stopping criterion
    // = the magnitude of the roots
    let scale_ref = radius.abs().max(1e-300);
    let mut converged = false;
    for _ in 0..1000 {
        let old = z.clone();
        let mut max_delta = 0.0f64;
        for i in 0..n {
            // p(zᵢ) — Horner, with real coefficients after making it monic
            let mut val = C::new(a[n], 0.0);
            for k in (0..n).rev() {
                val = val.mul(old[i]).add(C::new(a[k], 0.0));
            }
            // Π_{j≠i} (zᵢ − zⱼ), using the old values of the **same round** (simultaneous update,
            // more stable convergence)
            let mut den = C::ONE;
            for (j, zj) in old.iter().enumerate() {
                if j != i {
                    den = den.mul(old[i].sub(*zj));
                }
            }
            if den.abs() < 1e-300 {
                // Two initial values overlap until the denominator underflows: perturb and continue,
                // neither diverging nor returning garbage
                z[i] = old[i].add(C::new(1e-6 * (1.0 + i as f64), 1e-6));
                max_delta = f64::INFINITY;
                continue;
            }
            let step = val.div(den)?;
            z[i] = old[i].sub(step);
            if !step.re.is_finite()
                || !step.im.is_finite()
                || !z[i].re.is_finite()
                || !z[i].im.is_finite()
            {
                // Overflow/ill-conditioning: **declare failure immediately**. `f64::max` returns
                // the other side for NaN,
                // so a NaN step used to be taken as "converged" and the function returned Ok(all
                // NaN) — far worse than an error.
                return Err(Error::RootFindingFailed {
                    reason: "the iteration produced inf/NaN (divergence)".into(),
                });
            }
            // The criterion is relative to the **magnitude of the roots**, not to 1: the latter,
            // for roots of order 1e-50,
            // amounts to "absolute 1e-12", declaring convergence while still dozens of orders of
            // magnitude away from the root.
            let rel = step.abs() / scale_ref;
            max_delta = if rel.is_finite() {
                max_delta.max(rel)
            } else {
                f64::INFINITY
            };
        }
        // Loose convergence (1e-10): DK is only responsible for **separating** the points and
        // bringing them near the roots; precision is left to Newton + residual acceptance.
        // Pushing DK for the last bits is both slow and unstable (things like ∏(x−k) oscillate back
        // and forth).
        // Using 1e-14 as a hard criterion once made a healthy polynomial (∏(x−k), k≤8) run all 500
        // rounds and still be "not converged".
        if max_delta <= 1e-10 {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err(Error::RootFindingFailed {
            reason: "it did not converge within 1000 iterations".into(),
        });
    }

    // Newton finish: in the simple-root case it goes straight to machine precision; multiple roots
    // converge only linearly, but no worse.
    // `p'(z)` uses the derivative form of Horner (no extra allocation).
    // Indexed loop: `z[i]` is read and written repeatedly in the iteration (in-place Newton), not
    // set iteration
    #[allow(clippy::needless_range_loop)]
    for i in 0..n {
        for _ in 0..60 {
            let mut p = C::new(a[n], 0.0);
            let mut dp = C::ZERO;
            for k in (0..n).rev() {
                dp = dp.mul(z[i]).add(p);
                p = p.mul(z[i]).add(C::new(a[k], 0.0));
            }
            if dp.abs() < 1e-300 {
                break;
            }
            let Ok(step) = p.div(dp) else { break };
            let next = z[i].sub(step);
            if !next.re.is_finite() || !next.im.is_finite() {
                break;
            }
            z[i] = next;
            // The stopping criterion must be **relative**: for small roots (`x²−1e-26` has roots
            // ±1e-13), the absolute
            // criterion `1e-16·(1+|z|)` stops at a relative error of 1e-3 — a 22% error.
            let scale = z[i].abs().max(scale_ref * 1e-6).max(1e-300);
            if step.abs() <= 1e-15 * scale {
                break;
            }
        }
    }

    // Residual acceptance: `|p(z)|` must be small enough relative to the magnitudes of the
    // coefficients and z.
    // This step keeps out the case "the iteration finished but the result is meaningless" — DK does
    // not guarantee convergence to roots,
    // only fewer errors when it converges; the criterion must be independent of the iteration
    // itself.
    for zi in &z {
        let mut p = C::new(a[n], 0.0);
        for k in (0..n).rev() {
            p = p.mul(*zi).add(C::new(a[k], 0.0));
        }
        // The denominator of the relative backward error must be **Σ|a_k|·|z|^k** (including the
        // k=n term).
        // It used to be written as "|a_n| + Σ_{k<n}|a_k||z|^k", which for small |z| degenerates to
        // the constant |a_n|,
        // so `x²−1e-100` at z=4e-25 (off by a factor of 4e25) still had a residual ratio < 1e-49 —
        // the acceptance was a formality.
        let mut scale = 0.0f64;
        let mut pw = 1.0f64;
        // `pw` is the rolling value of |z|^k, in one-to-one correspondence with a[k] (a semantic
        // index, not set iteration)
        #[allow(clippy::needless_range_loop)]
        for k in 0..=n {
            scale += a[k].abs() * pw;
            pw *= zi.abs();
        }
        // Relative residual; when scale is zero (all-zero coefficients) fall back to the absolute
        // criterion
        let rel = if scale > 0.0 {
            p.abs() / scale
        } else {
            p.abs()
        };
        if rel > 1e-8 {
            return Err(Error::UnstableResidual {
                residual: rel,
                limit: 1e-8,
            });
        }
    }

    // Zero out numerical noise: **the imaginary part only**. A converged real root carries imaginary
    // residue of order 1e-16
    // (`1 + 1e-38j`), which is an iteration artifact; but a small real part may be a **real** root
    // (`x² − 1e-26` has the two roots ±1e-13) — zeroing it amounts to reporting a fake root.
    // The criterion is `|im| ≤ 1e-12·|z|`; the bound is documented.
    // The criterion must be **relative** to `|z|`: an absolute threshold (it used to be
    // `1e-12·(1+|re|)`) would wipe
    // the whole imaginary part of "complex roots with modulus below 1e-12" as residue — and that is
    // all the information those roots have.
    // The three roots of `x³−1e-60` are `1e-20` and `−5e-21 ± 8.66e-21j`; the absolute criterion
    // would turn the latter two
    // into two real roots (relative error 0.87) without reporting an error. The relative criterion
    // is insensitive to magnitude.
    for c in z.iter_mut() {
        let mag = c.abs();
        if mag > 0.0 && c.im.abs() <= 1e-12 * mag {
            c.im = 0.0;
        }
    }
    // Deterministic output: sort by (real part, imaginary part)
    z.sort_by(|a, b| {
        a.re.partial_cmp(&b.re)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(
                a.im.partial_cmp(&b.im)
                    .unwrap_or(core::cmp::Ordering::Equal),
            )
    });
    Ok(z)
}

pub fn poly_coeffs(e: &Expr, var: &str) -> Option<Vec<Expr>> {
    use Expr as E;
    match e {
        E::Num(_) | E::Indexed { .. } => Some(vec![e.clone()]),
        E::Var(n) => {
            if n == var {
                Some(vec![num(0.0), num(1.0)])
            } else {
                Some(vec![e.clone()])
            }
        }
        E::Neg(a) => Some(poly_coeffs(a, var)?.into_iter().map(neg).collect()),
        E::Add(a, b) | E::Sub(a, b) => {
            let (ca, cb) = (poly_coeffs(a, var)?, poly_coeffs(b, var)?);
            let n = ca.len().max(cb.len());
            let mut out = Vec::with_capacity(n);
            for k in 0..n {
                let x = ca.get(k).cloned().unwrap_or_else(|| num(0.0));
                let y = cb.get(k).cloned().unwrap_or_else(|| num(0.0));
                out.push(if matches!(e, E::Add(..)) {
                    add(x, y)
                } else {
                    sub(x, y)
                });
            }
            Some(out)
        }
        E::Mul(a, b) => {
            let (ca, cb) = (poly_coeffs(a, var)?, poly_coeffs(b, var)?);
            // Coefficient convolution = polynomial multiplication
            let mut out = vec![num(0.0); ca.len() + cb.len() - 1];
            for (i, x) in ca.iter().enumerate() {
                for (j, y) in cb.iter().enumerate() {
                    let prod = mul(x.clone(), y.clone());
                    out[i + j] = add(core::mem::replace(&mut out[i + j], num(0.0)), prod);
                }
            }
            Some(out)
        }
        E::Div(a, b) => {
            if has_var(b, var) {
                return None;
            }
            Some(
                poly_coeffs(a, var)?
                    .into_iter()
                    .map(|c| div(c, (**b).clone()))
                    .collect(),
            )
        }
        E::Pow(a, b) => match b.as_ref() {
            E::Num(n) if *n >= 0.0 && math::fract(*n) == 0.0 && *n <= 16.0 => {
                let c = poly_coeffs(a, var)?;
                let mut acc = vec![num(1.0)];
                for _ in 0..(*n as usize) {
                    let mut next = vec![num(0.0); acc.len() + c.len() - 1];
                    for (i, x) in acc.iter().enumerate() {
                        for (j, y) in c.iter().enumerate() {
                            let prod = mul(x.clone(), y.clone());
                            next[i + j] = add(core::mem::replace(&mut next[i + j], num(0.0)), prod);
                        }
                    }
                    acc = next;
                }
                Some(acc)
            }
            _ => None,
        },
        // Function calls/matrices/operators: they can only be a constant term if they contain no var
        // at all
        _ => {
            if has_var(e, var) {
                None
            } else {
                Some(vec![e.clone()])
            }
        }
    }
}

/// Whether it is an "explicit polynomial": the base of every Pow is the variable itself (`x^k`),
/// with none of the `(u-1)^3` kind.
fn is_explicit_poly(e: &Expr, var: &str) -> bool {
    match e {
        Expr::Pow(a, b) => {
            let base_is_var = matches!(a.as_ref(), Expr::Var(n) if n == var);
            let expo_num =
                matches!(b.as_ref(), Expr::Num(n) if *n >= 0.0 && math::fract(*n) == 0.0);
            base_is_var && expo_num
        }
        Expr::Num(_) | Expr::Var(_) | Expr::Indexed { .. } => true,
        Expr::Neg(a) => is_explicit_poly(a, var),
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => {
            is_explicit_poly(a, var) && is_explicit_poly(b, var)
        }
        _ => !has_var(e, var),
    }
}

/// Horner form: `c₀ + c₁x + c₂x² + …` → `(…((cₙx + cₙ₋₁)x + cₙ₋₂)x … + c₀)`.
///
/// Why it is worth it: a degree-n polynomial drops from `2n-1` multiplications to `n`, and the
/// magnitudes of intermediates do not blow up
/// (there is no large power like `x^n`). Both numerical stability and speed improve, at the cost of
/// more parentheses.
pub fn horner_in(e: &Expr, var: &str) -> Option<Expr> {
    // Only apply Horner to expressions "already written in the source as c₀ + c₁x + c₂x² …".
    // Compact forms like `(u-1)^3` are **not expanded** — expanding them is only harder to read (and
    // no faster).
    if !is_explicit_poly(e, var) {
        return None;
    }
    let coeffs = poly_coeffs(e, var)?;
    // Below degree 1 it is pointless (the coefficients are already the simplest form)
    if coeffs.len() <= 2 {
        return None;
    }
    // Drop the highest zero coefficients (like `0*x^3`)
    let mut last = coeffs.len() - 1;
    while last > 0 && matches!(&coeffs[last], Expr::Num(v) if *v == 0.0) {
        last -= 1;
    }
    if last < 2 {
        return None;
    }
    let x = var.to_string();
    let x = Expr::Var(x);
    let mut acc = coeffs[last].clone();
    for k in (0..last).rev() {
        acc = add(mul(acc, x.clone()), coeffs[k].clone());
    }
    Some(simplify(&acc))
}

/// Expand `dot(a, b)` into `Σaᵢbᵢ` (both operands must be concrete and have the same number of
/// elements).
pub fn expand_dot(e: &Expr) -> Option<Expr> {
    let Expr::Call(name, args) = e else {
        return None;
    };
    if name != "dot" || args.len() != 2 {
        return None;
    }
    let (a, b) = (
        flatten_matrix_value(&args[0])?,
        flatten_matrix_value(&args[1])?,
    );
    // Flatten row by row (the row count and each row's length must match)
    if a.len() != b.len() {
        return None;
    }
    let mut acc: Option<Expr> = None;
    for (ra, rb) in a.iter().zip(b.iter()) {
        if ra.len() != rb.len() {
            return None;
        }
        for (x, y) in ra.iter().zip(rb.iter()) {
            let prod = mul(x.clone(), y.clone());
            acc = Some(match acc {
                None => prod,
                Some(p) => add(p, prod),
            });
        }
    }
    acc
}

/// Unified unary/binary operator expansion: matrix operators + linear solving.
///
/// Every lane (evaluation / emission / differentiation) asks it once first, and only takes its own
/// rejection path if it cannot expand.
pub fn expand_call(e: &Expr) -> Option<Expr> {
    let Expr::Call(name, args) = e else {
        return None;
    };
    match name.as_str() {
        "det" | "trace" | "adjoint" | "inv" | "norm" => expand_matrix_call(e),
        "dot" => expand_dot(e),
        "poly_coeffs" => {
            if args.len() != 2 {
                return None;
            }
            let Expr::Var(v) = &args[1] else {
                return None;
            };
            poly_coeffs_vec(&args[0], v).ok()
        }
        "roots" => {
            if args.len() != 2 {
                return None;
            }
            let Expr::Var(v) = &args[1] else {
                return None;
            };
            solve_poly(&args[0], v).ok()
        }
        "solve" => {
            if args.len() != 2 {
                return None;
            }
            // Two forms: single-variable `solve(eq, x)` solves to a scalar; a system
            // `solve([eq…], [x…])` solves to a column vector — both expand here, single
            // mathematical source
            match &args[1] {
                Expr::Var(v) => solve_linear(&args[0], v).ok(),
                Expr::Mat(_) => {
                    let vars = mat_var_names(&args[1])?;
                    let eqs = flat_mat_items(&args[0])?;
                    solve_system(&eqs, &vars).ok()
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether the expression is **statically expandable** into a concrete matrix (decides the return
/// type of the generated function).
pub fn is_matrix_value(e: &Expr) -> bool {
    flatten_matrix_value(e).is_some()
}

/// Emission of a concrete matrix: `vec![vec![a, b], vec![c, d]]` (elements go through the scalar
/// emitter, CSE as usual).
///
/// `Vec<Vec<f64>>` is used rather than fixed-size arrays: the `n` of the generator lane (pattern
/// matrices) is a runtime parameter,
/// and the two lanes must return **the same type**, otherwise downstream would have to write two
/// copies of the code for "literal matrices" and "generated matrices".
fn emit_mat_literal(
    rows: &[Vec<Expr>],
    counts: &HashMap<String, usize>,
    names: &mut HashMap<String, String>,
    out: &mut Vec<String>,
    counter: &mut usize,
    fx: Fx,
) -> String {
    let mut parts = Vec::new();
    for r in rows {
        let mut cols = Vec::new();
        for c in r {
            let (s, _) = emit_node(c, false, counts, names, out, counter, fx);
            cols.push(s);
        }
        parts.push(format!("vec![{}]", cols.join(", ")));
    }
    format!("vec![{}]", parts.join(", "))
}

fn emit_uncached(
    e: &Expr,
    counts: &HashMap<String, usize>,
    names: &mut HashMap<String, String>,
    out: &mut Vec<String>,
    counter: &mut usize,
    fx: Fx,
) -> (String, u8) {
    match e {
        Expr::Num(v) => (fx.lit(*v), 9),
        Expr::Var(n) => (rust_ident(n), 9),
        Expr::Neg(a) => {
            if let Some(m) = flatten_matrix_value(e) {
                return (emit_mat_literal(&m, counts, names, out, counter, fx), 9);
            }
            let (s, p) = emit_node(a, false, counts, names, out, counter, fx);
            (format!("-{}", wrap(s, p, 5)), 4)
        }
        Expr::Add(a, b) => {
            // Element-wise matrix add/subtract: fold into a literal (scalar addition takes the
            // precedence path below as usual)
            if let Some(m) = flatten_matrix_value(e) {
                return (emit_mat_literal(&m, counts, names, out, counter, fx), 9);
            }
            // `x + (-3)` is written as `x - 3`: simplify folds out negative constants, and emitting
            // them as an addition is ugly
            if let Expr::Num(v) = b.as_ref() {
                if *v < 0.0 {
                    let (l, lp) = emit_node(a, false, counts, names, out, counter, fx);
                    return (format!("{} - {}", wrap(l, lp, 2), fx.lit(-*v)), 2);
                }
            }
            let (l, lp) = emit_node(a, false, counts, names, out, counter, fx);
            let (r, rp) = emit_node(b, false, counts, names, out, counter, fx);
            (format!("{} + {}", wrap(l, lp, 2), wrap(r, rp, 3)), 2)
        }
        Expr::Sub(a, b) => {
            // Element-wise matrix add/subtract: fold into a literal (scalar addition takes the
            // precedence path below as usual)
            if let Some(m) = flatten_matrix_value(e) {
                return (emit_mat_literal(&m, counts, names, out, counter, fx), 9);
            }
            let (l, lp) = emit_node(a, false, counts, names, out, counter, fx);
            let (r, rp) = emit_node(b, false, counts, names, out, counter, fx);
            (format!("{} - {}", wrap(l, lp, 2), wrap(r, rp, 3)), 2)
        }
        Expr::Mul(a, b) => {
            // Concrete matrix multiply / broadcast multiply: fold into a 2-D array literal (the
            // direct path at teaching scale;
            // large matrices go through pattern matrices + the loop generation lane)
            if let Some(m) = flatten_matrix_value(e) {
                return (emit_mat_literal(&m, counts, names, out, counter, fx), 9);
            }
            // Strength reduction: `x * -1` → `-x`, `x * 1` → `x`.
            // This is not just cosmetics: `* -1.0` triggers clippy::neg_multiply,
            // and the output must pass `clippy -D warnings`.
            if let Expr::Num(n) = a.as_ref() {
                if *n == -1.0 {
                    let (r, rp) = emit_node(b, false, counts, names, out, counter, fx);
                    return (format!("-{}", wrap(r, rp, 5)), 4);
                }
                if *n == 1.0 {
                    return emit_node(b, false, counts, names, out, counter, fx);
                }
            }
            if let Expr::Num(n) = b.as_ref() {
                if *n == -1.0 {
                    let (l, lp) = emit_node(a, false, counts, names, out, counter, fx);
                    return (format!("-{}", wrap(l, lp, 5)), 4);
                }
                if *n == 1.0 {
                    return emit_node(a, false, counts, names, out, counter, fx);
                }
            }
            let (l, lp) = emit_node(a, false, counts, names, out, counter, fx);
            let (r, rp) = emit_node(b, false, counts, names, out, counter, fx);
            (format!("{} * {}", wrap(l, lp, 3), wrap(r, rp, 4)), 3)
        }
        Expr::Div(a, b) => {
            let (l, lp) = emit_node(a, false, counts, names, out, counter, fx);
            let (r, rp) = emit_node(b, false, counts, names, out, counter, fx);
            (format!("{} / {}", wrap(l, lp, 3), wrap(r, rp, 4)), 3)
        }
        Expr::Pow(a, b) => {
            // Integer matrix powers (1..=8): try folding first — exactly isomorphic to
            // Neg/Add/Sub/Mul/Transpose.
            // The guard used to be placed before the folding, so M², which is supported, got
            // rejected too.
            if let Some(m) = flatten_matrix_value(e) {
                return (emit_mat_literal(&m, counts, names, out, counter, fx), 9);
            }
            // Folding failed but the base is **provably** a matrix (negative/fractional/out-of-range
            // power, non-square):
            // emit compile_error, never `vec![…] * vec![…]` / `(vec![…]).sqrt()`.
            // The test uses the precise predicate — `matrix_ish` is the over-approximation used for
            // broadcast decisions, and would
            // misjudge **scalars** like `det(J)^2` / `norm(v)^2` / `s(i)^2` as matrix powers.
            let base_is_matrix =
                matches!(a.as_ref(), Expr::Mat(_) | Expr::MatPattern(_)) || is_matrix_value(a);
            if base_is_matrix {
                return (
                    "compile_error!(\"矩阵幂请写成显式矩阵乘（生成器支持 0–8 次非负整数幂）\")"
                        .to_string(),
                    9,
                );
            }
            // Decidably non-real at compile time: negative constant base + non-integer exponent. The
            // value layer takes the complex principal branch,
            // while the real lane's `powf`/`sqrt` only give NaN — reject directly, do not emit a
            // silent wrong value.
            if let (Expr::Num(bv), Expr::Num(ev)) = (a.as_ref(), b.as_ref()) {
                if *bv < 0.0 && math::fract(*ev) != 0.0 {
                    return (
                        "compile_error!(\"负数的非整数次幂在实数车道无值（值层为复数）：请改写公式或走复数车道\")"
                            .to_string(),
                        9,
                    );
                }
            }
            let (l, lp) = emit_node(a, false, counts, names, out, counter, fx);
            match b.as_ref() {
                Expr::Num(n) if *n == 2.0 => {
                    let lw = wrap(l.clone(), lp, 4);
                    (format!("{lw} * {lw}"), 3)
                }
                Expr::Num(n) if *n == 0.5 => {
                    // no_std: `f64::sqrt` is from std, so go through libm
                    if fx.is_libm() {
                        (format!("libm::sqrt({})", wrap(l, lp, 3)), 9)
                    } else {
                        (format!("({}).sqrt()", wrap(l, lp, 3)), 9)
                    }
                }
                // Integer power: std goes through `powi`; the no_std lane has no `powi` → `libm::pow`
                Expr::Num(n) if math::fract(*n) == 0.0 && n.abs() < 1e9 => {
                    if fx.is_libm() {
                        (format!("libm::pow({}, {})", wrap(l, lp, 3), fx.lit(*n)), 9)
                    } else {
                        (format!("({}).powi({})", wrap(l, lp, 3), *n as i64), 9)
                    }
                }
                _ => {
                    let (r, rp) = emit_node(b, false, counts, names, out, counter, fx);
                    // no_std: `powf` does not exist → `libm::pow`
                    if fx.is_libm() {
                        (
                            format!("libm::pow({}, {})", wrap(l, lp, 3), wrap(r, rp, 3)),
                            9,
                        )
                    } else {
                        (format!("({}).powf({})", wrap(l, lp, 3), wrap(r, rp, 3)), 9)
                    }
                }
            }
        }
        Expr::Call(n, args) => {
            if matches!(n.as_str(), "__quad_root_1" | "__quad_root_2") {
                if args.len() != 3 {
                    return (
                        "compile_error!(\"内部二次根算子需要三个系数\")".to_string(),
                        9,
                    );
                }
                if let [Expr::Num(a), Expr::Num(b), Expr::Num(c)] = args.as_slice() {
                    if b * b - 4.0 * a * c < 0.0 {
                        return (
                            "compile_error!(\"二次方程的判别式为负：实数车道无实根，请走复数车道\")"
                                .to_string(),
                            9,
                        );
                    }
                }
                let (a, _) = emit_node(&args[0], false, counts, names, out, counter, fx);
                let (b, _) = emit_node(&args[1], false, counts, names, out, counter, fx);
                let (c, _) = emit_node(&args[2], false, counts, names, out, counter, fx);
                let (two, four, half) = (fx.lit(2.0), fx.lit(4.0), fx.lit(0.5));
                let first = n == "__quad_root_1";
                let large = if first {
                    "__q_q / __q_a"
                } else {
                    "__q_c / __q_q"
                };
                let small = if first {
                    "__q_c / __q_q"
                } else {
                    "__q_q / __q_a"
                };
                let text = format!(
                    "{{ let __q_a = {a}; let __q_b = {b}; let __q_c = {c}; \
                     let __q_s = (__q_b * __q_b - {four} * __q_a * __q_c).sqrt(); \
                     let __q_first = __q_b >= 0.0; \
                     let __q_q = if __q_first {{ -{half} * (__q_b + __q_s) }} else {{ -{half} * (__q_b - __q_s) }}; \
                     if __q_q == 0.0 {{ -__q_b / ({two} * __q_a) }} else if __q_first {{ {large} }} else {{ {small} }} }}"
                );
                return (text, 9);
            }
            let Some(arg0) = args.first() else {
                return (
                    "compile_error!(\"函数调用缺参数：本方言的函数都是一元的\")".to_string(),
                    9,
                );
            };
            if n == "sqrt" && matches!(arg0, Expr::Num(v) if *v < 0.0) {
                return (
                    "compile_error!(\"负数开平方在实数车道无值（值层为复数）：请改写公式或走复数车道\")"
                        .to_string(),
                    9,
                );
            }
            let (a, ap) = emit_node(arg0, false, counts, names, out, counter, fx);
            let wrapped = wrap(a, ap, 3);
            // no_std lane: the same name goes through `libm::*` (`fabs` is libm's abs)
            if fx.is_libm() {
                if let Some(m) = libm_name(n) {
                    return (format!("libm::{m}({wrapped})"), 9);
                }
            }
            match n.as_str() {
                "log" | "ln" => (format!("({wrapped}).ln()"), 9),
                "exp" | "sin" | "cos" | "tan" | "sinh" | "cosh" | "tanh" | "sqrt" | "abs" => {
                    (format!("({wrapped}).{n}()"), 9)
                }
                // min/max/clamp: std's min/max (clamping follows the semantics of
                // clamp(v, lo, hi))
                "min" | "max" => {
                    let (b, bp) = emit_node(&args[1], false, counts, names, out, counter, fx);
                    (format!("({wrapped}).{n}({})", wrap(b, bp, 3)), 9)
                }
                "clamp" => {
                    let (lo, lp) = emit_node(&args[1], false, counts, names, out, counter, fx);
                    let (hi, hp) = emit_node(&args[2], false, counts, names, out, counter, fx);
                    (
                        format!(
                            "({wrapped}).max({}).min({})",
                            wrap(lo, lp, 3),
                            wrap(hi, hp, 3)
                        ),
                        9,
                    )
                }
                // Matrix operators: uniformly go through the IR-level expansion (det/trace become
                // scalar expressions, adjoint becomes a matrix literal),
                // then hand the result to the same emitter — no separate string-level expansion
                "det" | "trace" | "adjoint" | "inv" | "solve" | "roots" | "poly_coeffs" | "dot"
                | "norm" => {
                    match expand_call(e) {
                    // Report back the **real** precedence: in `det(J)/2` the expanded det is a
                    // subtraction (p=2),
                    // and if atomicity were claimed falsely here, the parent would not add
                    // parentheses and the division would slip into the last term
                    Some(x) => emit_node(&x, false, counts, names, out, counter, fx),
                    None => (
                        "compile_error!(\"det/trace/adjoint/inv/norm/dot 需要具体矩阵字面量（det 只到 3 阶）\")"
                            .to_string(),
                        9,
                    ),
                    }
                }
                // Over the reals these three are identities: `re(x) = the part other than im` = x,
                // `conj(x) = x`.
                // Complex formulas take the complex lane, so emitting them this way is **exact**,
                // not an approximation.
                "re" | "im" | "conj" => (wrapped, ap),
                // `arg` has no meaningful value over the reals (0 or π depending on the sign); do
                // not guess
                "arg" => (
                    "compile_error!(\"arg 只对复数有定义：实数公式请显式写出符号判断\")"
                        .to_string(),
                    9,
                ),
                // Unregistered function names always fail at compile time (never emit a call to a
                // method that does not exist;
                // that kind of error points at the generated code, making debugging absurdly
                // expensive)
                _ => (
                    "compile_error!(\"未知函数：实数车道支持 exp/ln/log/sin/cos/tan/sinh/cosh/tanh/sqrt/abs/re/im/conj\")"
                        .to_string(),
                    9,
                ),
            }
        }
        Expr::Mat(rows) => (emit_mat_literal(rows, counts, names, out, counter, fx), 9),
        // Symbolic operators (∂/∇/∇²) and indexed symbols: not expressible in inline emission, so
        // point explicitly to the file lane
        Expr::Deriv(..) | Expr::Grad(_) | Expr::Laplacian(_) | Expr::Indexed { .. } => (
            "compile_error!(\"符号算子/索引符号请走 .formula 文件车道（生成器需要上下文）\")"
                .to_string(),
            9,
        ),
        // Σ: with constant bounds, **fully expand** (compile-time summation); otherwise point to
        // the file lane
        Expr::Sum {
            var,
            lower,
            upper,
            body,
        } => {
            if let (Expr::Num(lo), Expr::Num(hi)) = (lower.as_ref(), upper.as_ref()) {
                if math::fract(*lo) == 0.0 && math::fract(*hi) == 0.0 && (hi - lo) < 64.0 {
                    let mut parts = Vec::new();
                    let mut k = *lo;
                    while k <= *hi {
                        let b = subst(body, var, &num(k));
                        let (t, _) = emit_node(&b, false, counts, names, out, counter, fx);
                        parts.push(t);
                        k += 1.0;
                    }
                    // Empty interval (lo > hi) = the zero element: this used to emit an empty
                    // string, producing a `{ }` that does not compile
                    if parts.is_empty() {
                        return ("0.0".to_string(), 9);
                    }
                    return (parts.join(" + "), 2);
                }
            }
            (
                "compile_error!(\"非常量求和界限请走 .formula 文件车道\")".to_string(),
                9,
            )
        }
        // ∫: numerical integration is not inlined (it is a runtime loop), so point to the file
        // lane
        Expr::Integral { .. } => (
            "compile_error!(\"定积分请走 .formula 文件车道（数值积分是运行期循环）\")".to_string(),
            9,
        ),
        // A pattern matrix cannot be emitted directly as a value expression: it must first go
        // through the recognize_pattern →
        // emit_matrix_ctor lane; reaching here means an upper layer skipped the triage.
        Expr::MatPattern(_) => (
            "compile_error!(\"模式矩阵必须经生成器车道，不能内联发射\")".to_string(),
            9,
        ),
        // Transpose: a concrete literal can be folded into a literal (see flatten_matrix above);
        // the shape of a
        // symbolic matrix is unknown at compile time, so point explicitly to "write a concrete
        // matrix first" or the pattern matrix lane.
        Expr::Transpose(a) => {
            if let Some(m) = flatten_matrix_value(e) {
                return (emit_mat_literal(&m, counts, names, out, counter, fx), 9);
            }
            let _ = a;
            (
                "compile_error!(\"转置的被转置项必须是具体矩阵字面量（符号矩阵的形状编译期未知）\")"
                    .to_string(),
                9,
            )
        }
    }
}

// ---------------------------------------------------------------- pattern recognition

/// Recognize a pattern matrix as one of the builtin shapes. Failure to recognize = an error plus a
/// list of the supported shapes.
///
/// This is another case of "rejection is a feature": automatically expanding an arbitrary pattern
/// matrix is a research-level problem,
/// so the first version supports only three **provably correct** shapes and rejects the rest
/// explicitly.
/// Banded detection: every cell with |i-j| ≤ k is the **same expression**, and the rest are blank (0
/// or an ellipsis).
/// Only unambiguous k is accepted (m ≥ 2k+1 is needed to tell "banded" from "full matrix").
fn try_banded(p: &MatPattern, m: usize) -> Option<MatrixKind> {
    let cell = |i: usize, j: usize| -> Option<&PatternCell> { p.rows.get(i)?.get(j) };
    let blank = |i: usize, j: usize| -> bool {
        match cell(i, j) {
            Some(PatternCell::Dots) => true,
            Some(PatternCell::El(Expr::Num(v))) => *v == 0.0,
            _ => false,
        }
    };
    let el = |i: usize, j: usize| -> Option<&Expr> {
        match cell(i, j)? {
            PatternCell::El(e) => Some(e),
            PatternCell::Dots => None,
        }
    };
    // Try k from large to small (prefer the wider bandwidth interpretation)
    for k in (1..=(m / 2)).rev() {
        // Leave at least 2 rows "outside the band", otherwise it is indistinguishable from a full
        // matrix
        if m < 2 * k + 1 {
            continue;
        }
        let mut val: Option<&Expr> = None;
        let mut ok = true;
        let mut concrete = 0usize;
        for i in 0..m {
            for j in 0..m {
                let inside = i.abs_diff(j) <= k;
                if inside {
                    match el(i, j) {
                        Some(e) => {
                            concrete += 1;
                            match val {
                                None => val = Some(e),
                                Some(v) if key(v) == key(e) => {}
                                _ => {
                                    ok = false;
                                }
                            }
                        }
                        None => ok = false,
                    }
                } else if !blank(i, j) {
                    ok = false;
                }
                if !ok {
                    break;
                }
            }
            if !ok {
                break;
            }
        }
        // At least k+1 per row, and the total must match a banded shape (avoid recognizing a weird
        // sparse shape as banded)
        let expect: usize = (0..m)
            .map(|i| {
                let lo = i.saturating_sub(k);
                let hi = (i + k).min(m - 1);
                hi - lo + 1
            })
            .sum();
        if ok && concrete == expect {
            if let Some(v) = val {
                return Some(MatrixKind::Banded {
                    v: v.clone(),
                    half_band: k,
                });
            }
        }
    }
    None
}

/// 4×4 block-diagonal detection: the top-left 2×2 is all P, the bottom-right 2×2 is all Q,
/// and both the top-right and bottom-left 2×2 blocks are all 0 or ellipses (0 and an ellipsis are
/// equivalent — both mean "no coupling outside the block").
fn try_blockdiag(p: &MatPattern) -> Option<MatrixKind> {
    let cell = |i: usize, j: usize| -> Option<&PatternCell> { p.rows.get(i)?.get(j) };
    let el = |i: usize, j: usize| -> Option<&Expr> {
        match cell(i, j)? {
            PatternCell::El(e) => Some(e),
            PatternCell::Dots => None,
        }
    };
    // "Blank" = an ellipsis or a literal 0 (no coupling outside the block / no off-diagonal terms
    // inside it; the two are equivalent)
    let blank = |i: usize, j: usize| -> bool {
        match cell(i, j) {
            Some(PatternCell::Dots) => true,
            Some(PatternCell::El(Expr::Num(v))) => *v == 0.0,
            _ => false,
        }
    };
    // Both 2×2 blocks outside must be entirely blank
    for (i, j) in [
        (0, 2),
        (0, 3),
        (1, 2),
        (1, 3),
        (2, 0),
        (2, 1),
        (3, 0),
        (3, 1),
    ] {
        if !blank(i, j) {
            return None;
        }
    }
    // Shape inside the block = p·I₂: both diagonal cells share an expression, both off-diagonal
    // cells are blank
    let block = |a: (usize, usize), b: (usize, usize), o1: (usize, usize), o2: (usize, usize)| {
        let da = el(a.0, a.1)?;
        let db = el(b.0, b.1)?;
        if key(da) != key(db) {
            return None;
        }
        if !blank(o1.0, o1.1) || !blank(o2.0, o2.1) {
            return None;
        }
        Some(da.clone())
    };
    let pv = block((0, 0), (1, 1), (0, 1), (1, 0))?;
    let qv = block((2, 2), (3, 3), (2, 3), (3, 2))?;
    Some(MatrixKind::BlockDiag2 { p: pv, q: qv })
}

/// 4×4 bidiagonal detection (upper/lower), structure-adaptive: all main-diagonal keys identical +
/// the corresponding side all ellipses.
fn try_bidiag(p: &MatPattern) -> Option<MatrixKind> {
    let el = |i: usize, j: usize| -> Option<&Expr> {
        match p.rows.get(i)?.get(j)? {
            PatternCell::El(e) => Some(e),
            PatternCell::Dots => None,
        }
    };
    let is_dots = |i: usize, j: usize| {
        matches!(
            p.rows.get(i).and_then(|r| r.get(j)),
            Some(PatternCell::Dots)
        )
    };

    let diag: Option<Vec<&Expr>> = (0..4).map(|i| el(i, i)).collect();
    let diag = diag?;
    if key(diag[0]) != key(diag[1]) || key(diag[1]) != key(diag[2]) || key(diag[2]) != key(diag[3])
    {
        return None;
    }

    // Upper bidiagonal: the three entries of the superdiagonal (i,i+1) are concrete, the whole
    // lower triangle is ellipses
    let upper: Option<Vec<&Expr>> = (0..3).map(|i| el(i, i + 1)).collect();
    if let Some(up) = upper {
        let lower_dots = is_dots(1, 0)
            && is_dots(2, 0)
            && is_dots(2, 1)
            && is_dots(3, 0)
            && is_dots(3, 1)
            && is_dots(3, 2);
        let upper_same = key(up[0]) == key(up[1]) && key(up[1]) == key(up[2]);
        if lower_dots && upper_same {
            return Some(MatrixKind::BidiagUpper {
                b: diag[0].clone(),
                c: up[0].clone(),
            });
        }
    }

    // Lower bidiagonal: the three entries of the subdiagonal (i+1,i) are concrete, the whole upper
    // triangle is ellipses
    let lower: Option<Vec<&Expr>> = (0..3).map(|i| el(i + 1, i)).collect();
    if let Some(lo) = lower {
        let upper_dots = is_dots(0, 1)
            && is_dots(0, 2)
            && is_dots(0, 3)
            && is_dots(1, 2)
            && is_dots(1, 3)
            && is_dots(2, 3);
        let lower_same = key(lo[0]) == key(lo[1]) && key(lo[1]) == key(lo[2]);
        if upper_dots && lower_same {
            return Some(MatrixKind::BidiagLower {
                a: lo[0].clone(),
                b: diag[0].clone(),
            });
        }
    }
    None
}

pub fn recognize_pattern(p: &MatPattern) -> Result<MatrixKind, Error> {
    let (m, n) = p.grid().ok_or("模式矩阵各行长度不一致")?;

    let el = |i: usize, j: usize| -> Option<&Expr> {
        match p.rows.get(i)?.get(j)? {
            PatternCell::El(e) => Some(e),
            PatternCell::Dots => None,
        }
    };
    let is_dots = |i: usize, j: usize| {
        matches!(
            p.rows.get(i).and_then(|r| r.get(j)),
            Some(PatternCell::Dots)
        )
    };

    // ---- 3×3 family: identity / diag ----
    if m == 3 && n == 3 {
        // Corners and centre: [[x, ·, z], [·, y, ·], [w, ·, v]]
        let all_dots_off = [(0, 1), (1, 0), (1, 2), (2, 1)]
            .iter()
            .all(|&(i, j)| is_dots(i, j));
        if all_dots_off {
            let get = |i: usize, j: usize| el(i, j).cloned();
            let (x, z, y, w, v) = (get(0, 0), get(0, 2), get(1, 1), get(2, 0), get(2, 2));
            if let (Some(x), Some(z), Some(y), Some(w), Some(v)) = (x, z, y, w, v) {
                let is_zero = |e: &Expr| matches!(e, Expr::Num(v) if *v == 0.0);
                let is_one = |e: &Expr| matches!(e, Expr::Num(v) if *v == 1.0);
                // identity: [[1,·,0],[·,1,·],[0,·,1]]
                if is_one(&x) && is_zero(&z) && is_one(&y) && is_zero(&w) && is_one(&v) {
                    return Ok(MatrixKind::Identity);
                }
                // Indexed diagonals take priority: the general term d_{1}d_{2}d_{3} / d_{i} →
                // vector parameter
                if let Some(name) = indexed_diag(&[&x, &y, &v]) {
                    if is_zero(&z) && is_zero(&w) {
                        return Ok(MatrixKind::DiagVec(name));
                    }
                }
                // diag: the three diagonal entries share an expression, the two anti-diagonal
                // entries are 0
                let same_diag = key(&x) == key(&y) && key(&y) == key(&v);
                if same_diag && is_zero(&z) && is_zero(&w) {
                    return Ok(MatrixKind::Diag(x));
                }
            }
        }
    }

    // ---- banded (k=2 is unambiguous only when m≥5; for 4×4, k=2 degenerates to a full matrix) ----
    if m == n && m >= 5 {
        if let Some(kind) = try_banded(p, m) {
            return Ok(kind);
        }
    }

    // ---- 4×4: bidiag / tridiag ----
    if m == 4 && n == 4 {
        // Block diagonal first: top-left 2×2 = P, bottom-right 2×2 = Q, both off-diagonal blocks
        // all 0/ellipses
        //   P · | 0 0        P 0 | 0 0
        //   · P | 0 0  or   0 P | 0 0
        //   ----+----       -----+-----
        //   0 0 | Q ·        0 0 | Q 0
        //   0 0 | · Q        0 0 | 0 Q
        if let Some(kind) = try_blockdiag(p) {
            return Ok(kind);
        }
        // Bidiagonal first (structure-adaptive, not depending on the concrete cell template of
        // tridiag)
        if let Some(kind) = try_bidiag(p) {
            return Ok(kind);
        }

        // tridiag template (the expected concrete cells; the rest must be Dots):
        //   b c · 0
        //   a b c ·
        //   · a b c
        //   0 · a b
        let concrete = [
            (0, 0),
            (0, 1),
            (0, 3),
            (1, 0),
            (1, 1),
            (1, 2),
            (2, 1),
            (2, 2),
            (2, 3),
            (3, 0),
            (3, 2),
            (3, 3),
        ];
        let dots_at = [(0, 2), (1, 3), (2, 0), (3, 1)];
        let structure_ok = dots_at.iter().all(|&(i, j)| is_dots(i, j))
            && concrete.iter().all(|&(i, j)| el(i, j).is_some());
        if structure_ok {
            let grab = |i: usize, j: usize| el(i, j).cloned().unwrap();
            let is_zero = |e: &Expr| matches!(e, Expr::Num(v) if *v == 0.0);
            let (b1, c1, z1) = (grab(0, 0), grab(0, 1), grab(0, 3));
            let (a1, b2, c2) = (grab(1, 0), grab(1, 1), grab(1, 2));
            let (a2, b3, c3) = (grab(2, 1), grab(2, 2), grab(2, 3));
            let (z2, a3, b4) = (grab(3, 0), grab(3, 2), grab(3, 3));

            // Indexed diagonals take priority: the general term b_{i} has identical keys and would
            // be intercepted by the scalar branch —
            // but the intent of the Indexed form (a vector) is more explicit than "the same
            // expression" (scalar broadcasting).
            if let Some(name) = indexed_diag(&[&b1, &b2, &b3, &b4]) {
                if key(&a1) == key(&a2)
                    && key(&a2) == key(&a3)
                    && key(&c1) == key(&c2)
                    && key(&c2) == key(&c3)
                    && is_zero(&z1)
                    && is_zero(&z2)
                {
                    return Ok(MatrixKind::TridiagVec { a: a1, name, c: c1 });
                }
            }
            let diag_same = key(&b1) == key(&b2) && key(&b2) == key(&b3) && key(&b3) == key(&b4);
            let sub_same = key(&a1) == key(&a2) && key(&a2) == key(&a3);
            let sup_same = key(&c1) == key(&c2) && key(&c2) == key(&c3);
            if diag_same && sub_same && sup_same && is_zero(&z1) && is_zero(&z2) {
                return Ok(MatrixKind::Tridiag {
                    a: a1,
                    b: b1,
                    c: c1,
                });
            }
            return Err(Error::InvalidInput(
            "this 4×4 pattern looks tridiagonal, but the main and off-diagonals are not the same expression, or the far corners are not 0".into(),
        ));
        }
    }

    Err(Error::InvalidInput(format!(
        "模式矩阵（{m}×{n}）不匹配任何内置形状。支持：\n  \
         identity(n)                  [1 ⋯ 0; ⋮ 1 ⋮; 0 ⋯ 1]\n  \
         diag(d, n) / diag_v(d[i], n) 对角同一表达式 / 对角是索引序列\n  \
         tridiag(a,b,c,n) / tridiag_v(...)  三对角（角落必须是省略号）\n  \
         bidiag_u(b,c,n) / bidiag_l(a,b,n)  上/下双对角\n  \
         blockdiag2(p,q,n,m)          分块对角 diag(p·I₂, q·I₂)\n  \
         banded(v,k,n)                对称带状（m≥2k+1 才无歧义）\n\
         注：不带省略号的矩阵草图本来就是**具体矩阵**，会被发射成字面量\n\
         （比生成器更优：常量可折叠、无分配循环）—— 想走生成器车道就写出省略号。"
    )))
}

/// Rust source of the generated matrix constructor (parameterized by shape: n is the runtime size).
/// The signature is derived from the pattern's **free variables** (`k: f64, ..., n: usize`), without
/// hard-coded parameter names.
pub fn emit_matrix_ctor(fname: &str, kind: &MatrixKind) -> String {
    match kind {
        MatrixKind::Identity => format!(
            "/// 单位阵 I_n（由模式矩阵生成）\n#[allow(non_snake_case)]\npub fn {fname}(n: usize) -> Vec<Vec<f64>> {{\n    vec![vec![0.0; n]; n]\n        .into_iter()\n        .enumerate()\n        .map(|(i, mut row)| {{ row[i] = 1.0; row }})\n        .collect()\n}}\n"
        ),
        MatrixKind::Diag(d) => {
            let body = emit_block(d);
            let params = sig_of(core::slice::from_ref(d));
            format!(
                "/// 对角阵 diag(d)·I_n（由模式矩阵生成；d 为标量广播）\n#[allow(non_snake_case)]\npub fn {fname}({params}, n: usize) -> Vec<Vec<f64>> {{\n    let mut m = vec![vec![0.0; n]; n];\n    for __row in 0..n {{ m[__row][__row] = {body}; }}\n    m\n}}\n"
            )
        }
        MatrixKind::DiagVec(name) => {
            let id = rust_ident(name);
            format!(
                "/// 对角阵（由索引模式矩阵生成；d 是**向量**，长度须 ≥ n）\n#[allow(non_snake_case)]\npub fn {fname}({id}: &[f64], n: usize) -> Vec<Vec<f64>> {{\n    let mut m = vec![vec![0.0; n]; n];\n    for __row in 0..n {{\n        m[__row][__row] = {id}[__row];\n    }}\n    m\n}}\n"
            )
        }
        MatrixKind::TridiagVec { a, c, .. } => {
            let ba = emit_block(a);
            let bc = emit_block(c);
            format!(
                "/// 三对角阵（由索引模式矩阵生成；b 是**向量**，长度须 ≥ n）\n#[allow(non_snake_case)]\npub fn {fname}(a: f64, b: &[f64], c: f64, n: usize) -> Vec<Vec<f64>> {{\n    let av = {ba};\n    let cv = {bc};\n    let mut m = vec![vec![0.0; n]; n];\n    for __row in 0..n {{\n        m[__row][__row] = b[__row];\n        let __next = __row + 1;\n        if __next < n {{\n            m[__row][__next] = cv;\n            m[__next][__row] = av;\n        }}\n    }}\n    m\n}}\n"
            )
        }
        MatrixKind::BlockDiag2 { p, q } => {
            let bp = emit_block(p);
            let bq = emit_block(q);
            format!(
                "/// 分块对角阵 diag(P·I₂, Q·I₂)（由模式矩阵生成）\n#[allow(non_snake_case, clippy::needless_range_loop)]\npub fn {fname}(p: f64, q: f64, n: usize) -> Vec<Vec<f64>> {{\n    // n 是**总**维数；两块各占一半（奇数时右下块多一）\n    let pv = {bp};\n    let qv = {bq};\n    let half = n / 2;\n    let mut m = vec![vec![0.0; n]; n];\n    for __row in 0..half {{\n        m[__row][__row] = pv;\n        m[n - 1 - __row][n - 1 - __row] = qv;\n    }}\n    if n % 2 == 1 {{\n        m[half][half] = qv;\n    }}\n    m\n}}\n"
            )
        }
        MatrixKind::Banded { v, half_band } => {
            let bv = emit_block(v);
            let k = *half_band;
            format!(
                "/// 对称带状阵（半带宽 {k}：主对角与 {k} 条副对角同值）\n#[allow(non_snake_case, clippy::needless_range_loop)]\npub fn {fname}(v: f64, n: usize) -> Vec<Vec<f64>> {{\n    let vv = {bv};\n    let mut m = vec![vec![0.0; n]; n];\n    for __row in 0..n {{\n        let __hi = (__row + {k}).min(n - 1);\n        for __col in __row..=__hi {{\n            m[__row][__col] = vv;\n            m[__col][__row] = vv;\n        }}\n    }}\n    m\n}}\n"
            )
        }
        MatrixKind::BidiagUpper { b, c } => {
            let bb = emit_block(b);
            let bc = emit_block(c);
            format!(
                "/// 上双对角阵（主对角 b + 上次对角 c）\n#[allow(non_snake_case, clippy::needless_range_loop)]\npub fn {fname}(b: f64, c: f64, n: usize) -> Vec<Vec<f64>> {{\n    let bv = {bb};\n    let cv = {bc};\n    let mut m = vec![vec![0.0; n]; n];\n    for __row in 0..n {{\n        m[__row][__row] = bv;\n        let __next = __row + 1;\n        if __next < n {{\n            m[__row][__next] = cv;\n        }}\n    }}\n    m\n}}\n"
            )
        }
        MatrixKind::BidiagLower { a, b } => {
            let ba = emit_block(a);
            let bb = emit_block(b);
            format!(
                "/// 下双对角阵（主对角 b + 下次对角 a）\n#[allow(non_snake_case, clippy::needless_range_loop)]\npub fn {fname}(a: f64, b: f64, n: usize) -> Vec<Vec<f64>> {{\n    let av = {ba};\n    let bv = {bb};\n    let mut m = vec![vec![0.0; n]; n];\n    for __row in 0..n {{\n        m[__row][__row] = bv;\n        let __next = __row + 1;\n        if __next < n {{\n            m[__next][__row] = av;\n        }}\n    }}\n    m\n}}\n"
            )
        }
        MatrixKind::Tridiag { a, b, c } => {
            let ba = emit_block(a);
            let bb = emit_block(b);
            let bc = emit_block(c);
            let params = sig_of(&[a.clone(), b.clone(), c.clone()]);
            format!(
                "/// 三对角阵 tridiag(a,b,c)_n（由模式矩阵生成）\n#[allow(non_snake_case, clippy::too_many_arguments)]\npub fn {fname}({params}, n: usize) -> Vec<Vec<f64>> {{\n    let av = {ba};\n    let bv = {bb};\n    let cv = {bc};\n    let mut m = vec![vec![0.0; n]; n];\n    #[allow(clippy::needless_range_loop)]\n    for __row in 0..n {{\n        m[__row][__row] = bv;\n        let __next = __row + 1;\n        if __next < n {{\n            m[__row][__next] = cv;\n            m[__next][__row] = av;\n        }}\n    }}\n    m\n}}\n"
            )
        }
    }
}

/// Generator signature: the union of the pattern's free variables (sorted and deduplicated), e.g.
/// `k: f64`.
fn sig_of(exprs: &[Expr]) -> String {
    let mut names: Vec<String> = exprs.iter().flat_map(vars_of).collect();
    names.sort();
    names.dedup();
    names
        .iter()
        .map(|n| format!("{}: f64", rust_ident(n)))
        .collect::<Vec<_>>()
        .join(", ")
}

// ---------------------------------------------------------------- MathML rendering (shared by the L1 IDE / web)

/// IR → MathML Core (presentation). Pure string generation, zero dependencies — this is the
/// first building block of the rendering assets: the single source of truth distributed to VS Code
/// hover, hikari components and the docs site.
pub fn to_mathml(e: &Expr) -> String {
    format!("<math>{}</math>", mm(e))
}

fn mm(e: &Expr) -> String {
    match e {
        Expr::Num(v) => format!("<mn>{}</mn>", fmt_num(*v)),
        Expr::Var(n) => format!("<mi>{}</mi>", xml_escape(n)),
        Expr::Add(a, b) => format!("<mrow>{}<mo>+</mo>{}</mrow>", mm(a), mm(b)),
        Expr::Sub(a, b) => format!("<mrow>{}<mo>-</mo>{}</mrow>", mm(a), mm(b)),
        Expr::Mul(a, b) => format!("<mrow>{}<mo>\u{2062}</mo>{}</mrow>", mm(a), mm(b)),
        Expr::Div(a, b) => format!("<mfrac>{}{}</mfrac>", mm(a), mm(b)),
        Expr::Pow(a, b) => format!("<msup>{}{}</msup>", mm(a), mm(b)),
        Expr::Neg(a) => format!("<mrow><mo>-</mo>{}</mrow>", mm(a)),
        Expr::Call(name, args) => {
            // Multiple arguments are separated by `<mo>,</mo>` (otherwise `min(a,b)` renders as
            // `min(a b)`).
            let inner: String = args
                .iter()
                .map(mm)
                .collect::<Vec<_>>()
                .join("<mo>,</mo>");
            format!(
                "<mrow><mi>{}</mi><mo>\u{2061}</mo><mo>(</mo>{}<mo>)</mo></mrow>",
                xml_escape(name),
                inner
            )
        }
        Expr::Transpose(a) => format!("<msup>{}<mi>T</mi></msup>", mm(a)),
        Expr::Deriv(u, by) => format!(
            "<mfrac><mrow><mo>\u{2202}</mo>{}</mrow><mrow><mo>\u{2202}</mo><mi>{}</mi></mrow></mfrac>",
            mm(u),
            xml_escape(by)
        ),
        Expr::Grad(u) => format!("<mrow><mo>\u{2207}</mo>{}</mrow>", mm(u)),
        Expr::Laplacian(u) => format!("<mrow><msup><mo>\u{2207}</mo><mn>2</mn></msup>{}</mrow>", mm(u)),
        Expr::Sum { var, lower, upper, body } => format!(
            "<mrow><munderover><mo>\u{2211}</mo><mrow><mi>{}</mi><mo>=</mo>{}</mrow>{}</munderover>{}</mrow>",
            xml_escape(var),
            mm(lower),
            mm(upper),
            mm(body)
        ),
        Expr::Integral { var, lower, upper, body } => format!(
            "<mrow><munderover><mo>\u{222b}</mo>{}{}</munderover>{}<mi>{}d</mi></mrow>",
            mm(lower),
            mm(upper),
            mm(body),
            xml_escape(var)
        ),
        Expr::Indexed { name, indices } => {
            let sub: String = indices
                .iter()
                .map(|i| format!("<mi>{}</mi>", xml_escape(i)))
                .collect();
            format!("<msub><mi>{}</mi><mrow>{}</mrow></msub>", xml_escape(name), sub)
        }
        Expr::Mat(rows) => mat_mathml(&rows.iter().map(|r| r.iter().map(mm_cell).collect::<Vec<_>>()).collect::<Vec<_>>()),
        Expr::MatPattern(p) => mat_mathml(
            &p.rows
                .iter()
                .map(|r| r.iter().map(mm_pattern_cell).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
        ),
    }
}

fn mm_cell(e: &Expr) -> String {
    format!("<mtd>{}</mtd>", mm(e))
}

fn mm_pattern_cell(c: &PatternCell) -> String {
    match c {
        PatternCell::El(e) => format!("<mtd>{}</mtd>", mm(e)),
        PatternCell::Dots => "<mtd><mo>\u{22ef}</mo></mtd>".to_string(),
    }
}

fn mat_mathml(rows: &[Vec<String>]) -> String {
    let rows_xml: String = rows
        .iter()
        .map(|r| format!("<mtr>{}</mtr>", r.concat()))
        .collect();
    format!("<mtable>{rows_xml}</mtable>")
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// ---------------------------------------------------------------- canonical write-back (matlab side)

/// Canonical matlab-side write-back: parenthesized by operator precedence, matrices aligned by
/// column.
/// This is the single source of truth for the formatter (``fmt``) — humans may use any amount of
/// whitespace, the machine output is constant.
pub fn to_matex(e: &Expr) -> String {
    to_matex_raw_matex(e).0
}

fn wrap_matex(e: &Expr, min: u8) -> String {
    let (s, p) = to_matex_raw_matex(e);
    if p < min {
        format!("({s})")
    } else {
        s
    }
}

/// Precedence-aware matex emission: **precedence lives only in this one match** (a second
/// `matex_prec` used to drift from it — Transpose rose to 6 while it still said 9, so
/// transpose-of-transpose
/// lost its parentheses and `x''` could not be read back).
fn to_matex_raw_matex(e: &Expr) -> (String, u8) {
    match e {
        Expr::Num(v) => (fmt_num(*v), 9),
        Expr::Var(n) => (n.clone(), 9),
        Expr::Neg(a) => (format!("-{}", wrap_matex(a, 5)), 4),
        Expr::Add(a, b) => (format!("{} + {}", wrap_matex(a, 2), wrap_matex(b, 3)), 2),
        Expr::Sub(a, b) => (format!("{} - {}", wrap_matex(a, 2), wrap_matex(b, 3)), 2),
        Expr::Mul(a, b) => (format!("{} * {}", wrap_matex(a, 3), wrap_matex(b, 4)), 3),
        Expr::Div(a, b) => (format!("{} / {}", wrap_matex(a, 3), wrap_matex(b, 4)), 3),
        // Exponentiation is **right-associative** and non-stacking: `x ^ 2 ^ 3` re-parses as
        // `(x^2)^3` (x⁸ becomes x⁶),
        // so if the base or the exponent is itself a power/transpose it always gets parentheses
        Expr::Pow(a, b) => (format!("{} ^ {}", wrap_matex(a, 7), wrap_matex(b, 7)), 6),
        Expr::Call(name, args) => (
            format!(
                "{}({})",
                name,
                args.iter().map(to_matex).collect::<Vec<_>>().join(", ")
            ),
            9,
        ),
        Expr::Transpose(a) => (format!("{}'", wrap_matex(a, 7)), 6),
        Expr::Deriv(u, by) => (format!("\u{2202}({}, {})", to_matex(u), by), 9),
        Expr::Grad(u) => (format!("\u{2207}({})", to_matex(u)), 9),
        Expr::Laplacian(u) => (format!("\u{2207}^2({})", to_matex(u)), 9),
        Expr::Sum {
            var,
            lower,
            upper,
            body,
        } => (
            format!(
                "\u{2211}({}, {}, {}, {})",
                var,
                to_matex(lower),
                to_matex(upper),
                to_matex(body)
            ),
            9,
        ),
        Expr::Integral {
            var,
            lower,
            upper,
            body,
        } => (
            format!(
                "\u{222b}({}, {}, {}, {})",
                var,
                to_matex(lower),
                to_matex(upper),
                to_matex(body)
            ),
            9,
        ),
        // The canonical form of an indexed symbol = `a(i, k)`: the same shape as matex's index
        // entry point,
        // so "format → re-parse" closes. It used to be written `a[i, k]`, which the grammar could
        // not read back
        // (the formatting gate defeated itself: after --write, --check is red forever).
        Expr::Indexed { name, indices } => (format!("{}({})", name, indices.join(", ")), 9),
        Expr::Mat(rows) => (format_matex_rows(&rows_payload(rows)), 9),
        Expr::MatPattern(p) => {
            let cells: Vec<Vec<Option<String>>> = p
                .rows
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|c| match c {
                            PatternCell::El(e) => Some(to_matex(e)),
                            PatternCell::Dots => None,
                        })
                        .collect()
                })
                .collect();
            (format_matex_rows(&cells), 9)
        }
    }
}

fn rows_payload(rows: &[Vec<Expr>]) -> Vec<Vec<Option<String>>> {
    rows.iter()
        .map(|r| r.iter().map(|c| Some(to_matex(c))).collect())
        .collect()
}

/// Canonical matrix layout: each column's width = the widest content in that column + at least 1
/// space;
/// every row ends with `;` (except the last row); `...` is aligned with the content in its column.
#[allow(clippy::needless_range_loop)] // the column index j is layout semantics (a width-table index), not an iteration index
pub fn format_matex_rows(cells: &[Vec<Option<String>>]) -> String {
    let cols = cells.iter().map(|r| r.len()).max().unwrap_or(0);
    let dots = "...".to_string();
    let mut widths = vec![0usize; cols];
    for r in cells {
        for (j, c) in r.iter().enumerate() {
            let w = c.as_ref().unwrap_or(&dots).chars().count();
            widths[j] = widths[j].max(w);
        }
    }
    // Column separator = comma: the canonical form must be unambiguous (hand-written code may omit
    // commas and use spaces,
    // but a space before `-k` is swallowed by binary subtraction — the round-trip would break).
    let mut lines = Vec::new();
    for (i, r) in cells.iter().enumerate() {
        let mut line = String::new();
        for j in 0..cols {
            let cell = r
                .get(j)
                .and_then(|x| x.clone())
                .unwrap_or_else(|| dots.clone());
            line.push_str(&cell);
            if j + 1 < cols {
                line.push(',');
            }
            let pad = widths[j] + 1 - cell.chars().count();
            line.push_str(&" ".repeat(pad));
        }
        let mut line = line.trim_end().to_string();
        if i + 1 < cells.len() {
            line.push_str(" ;");
        }
        lines.push(line);
    }
    format!("[ {} ]", lines.join("\n  "))
}

/// Canonical text of the whole formula set (declarations on multiple lines + formulas one by one +
/// matrix alignment).
pub fn format_set(set: &FormulaSet) -> String {
    let mut out = String::new();
    // `use` must come first: canonicalization **must never drop dependency declarations**
    // (dropping them raises no error, it only says "undeclared symbol" on the next build — the
    // hardest kind to trace)
    for u in &set.uses {
        out.push_str(&format!("use \"{u}\"\n"));
    }
    if !set.uses.is_empty() {
        out.push('\n');
    }
    if !set.decls.is_empty() {
        out.push_str("decl {\n");
        for d in &set.decls {
            match &d.unit {
                Some(u) => out.push_str(&format!("    {} : {} [{}];\n", d.name, d.quantity, u)),
                None => out.push_str(&format!("    {} : {};\n", d.name, d.quantity)),
            }
        }
        out.push_str("}\n");
    }
    for item in &set.items {
        let body = to_matex(&item.expr);
        match &item.name {
            Some(n) => out.push_str(&format!("{n} = {body}\n")),
            None => out.push_str(&format!("{body}\n")),
        }
    }
    out
}

// ---------------------------------------------------------------- unit tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units() {
        assert_eq!(
            parse_unit("A/m^2").unwrap(),
            Dim::from_exponents([0, -2, 0, 1, 0, 0, 0])
        );
        assert_eq!(
            parse_unit("V").unwrap(),
            Dim::from_exponents([1, 2, -3, -1, 0, 0, 0])
        );
        assert_eq!(parse_unit("kg*m/s^2").unwrap(), parse_unit("N").unwrap());
        assert!(parse_unit("A/m^x").is_err());
    }

    #[test]
    fn units_fractional_exponents() {
        // Ω·s^(-1/2): the textbook unit of the Warburg coefficient. The parentheses are required —
        // without them the `/` in the exponent is confused with the unit string's division sign.
        let w = parse_unit("V/A*s^(-1/2)").unwrap();
        assert_eq!(w.display(), "kg·m^2·s^(-7/2)·A^-2");
        assert_eq!(
            w,
            parse_unit("V/A")
                .unwrap()
                .mul_dim(parse_unit("s").unwrap().scale_ratio(-1, 2).unwrap())
        );
        // Equivalent writing: the root and the power come from the same source
        assert_eq!(
            parse_unit("s^(1/2)").unwrap().scale_ratio(2, 1).unwrap(),
            parse_unit("s").unwrap()
        );
        // Anything outside the half-lattice algebra is rejected: m^(1/3) does not exist in these
        // dimensions
        assert!(parse_unit("m^(1/3)").is_err());
        // Unbalanced parentheses / zero denominator
        assert!(parse_unit("s^(-1/2").is_err());
        assert!(parse_unit("s^(1/0)").is_err());
        // A fractional exponent without parentheses is taken as the unit string's division sign
        // (a deliberate, explainable failure)
        assert!(parse_unit("V/A*s^-1/2").is_err());
    }

    #[test]
    fn complex_arithmetic_basics() {
        let z = C::new(0.7, -1.3);
        // sqrt² = itself (principal branch)
        let s = z.sqrt();
        let back = s.mul(s);
        assert!((back.re - z.re).abs() < 1e-15 && (back.im - z.im).abs() < 1e-15);
        // exp∘ln = itself
        let e = z.ln().unwrap().exp();
        assert!((e.re - z.re).abs() < 1e-15 && (e.im - z.im).abs() < 1e-15);
        // An integer power of a negative real must **stay exactly on the real axis** (polar
        // coordinates would give a trace of a fake imaginary part)
        let sq = C::real(-1.3).pow_real(2.0);
        assert!(sq.is_real(), "负底整数幂必须落在实轴上：{sq:?}");
        assert!((sq.re - 1.69).abs() < 1e-15, "{}", sq.re);
        // A half-integer power of a negative real takes the principal branch: √(-4) = 2j
        let i2 = C::real(-4.0).sqrt();
        assert!(i2.re.abs() < 1e-15 && (i2.im - 2.0).abs() < 1e-15);
        // Division by zero is an error, not NaN
        assert!(C::ONE.div(C::ZERO).is_err());
        assert!(C::ZERO.ln().is_err());
        // Complex power: i^i = e^(-π/2) (a hard check against the analytic form)
        let i = C::I.pow(C::I).unwrap();
        assert!((i.re - (-core::f64::consts::FRAC_PI_2).exp()).abs() < 1e-15);
        assert!(i.im.abs() < 1e-15);
    }

    #[test]
    fn si_prefixes_and_conversion() {
        // Lookup order: full name first, then prefix splitting (`m` is metre, `mol` is mole, `cd` is
        // candela)
        assert_eq!(parse_unit("mm").unwrap(), parse_unit("m").unwrap());
        assert_eq!(parse_unit("mol").unwrap(), parse_unit("mol").unwrap());
        assert_eq!(parse_unit("GHz").unwrap(), parse_unit("Hz").unwrap());
        assert!((parse_unit_spec("nF").unwrap().scale - 1e-9).abs() < 1e-20);
        assert!((parse_unit_spec("mm").unwrap().scale - 1e-3).abs() < 1e-18);
        assert!((parse_unit_spec("km").unwrap().scale - 1e3).abs() < 1e-9);
        // Prefixes scale by the exponent: (mm)^2 = 1e-6
        assert!((parse_unit_spec("mm^2").unwrap().scale - 1e-6).abs() < 1e-20);
        // Composite units: the conversion factor of mV/mm is 1 (1e-3 / 1e-3)
        assert!((parse_unit_spec("mV/mm").unwrap().scale - 1.0).abs() < 1e-18);
        // Longer prefixes win: daN is ten newtons, not "deci-attonewton"
        assert!((parse_unit_spec("daN").unwrap().scale - 10.0).abs() < 1e-12);
        // Conversion: the dimensions must agree
        assert!((convert(1.0, "nF", "F").unwrap() - 1e-9).abs() < 1e-24);
        assert!((convert(1500.0, "mV", "V").unwrap() - 1.5).abs() < 1e-12);
        assert!((convert(1.0, "kHz", "Hz").unwrap() - 1000.0).abs() < 1e-9);
        // The SI base unit of mass is kg: g and every prefixed gram must be computed from
        // 1 g = 1e-3 kg.
        assert_eq!(convert(1.0, "g", "kg").unwrap(), 1e-3);
        assert_eq!(convert(1.0, "kg", "g").unwrap(), 1000.0);
        assert_eq!(convert(1.0, "mg", "kg").unwrap(), 1e-6);
        assert_eq!(convert(1.0, "Mg", "kg").unwrap(), 1000.0);
        assert_eq!(convert(1.0, "mg", "g").unwrap(), 1e-3);
        assert!(matches!(
            convert(1.0, "nF", "V").unwrap_err(),
            Error::IncompatibleUnits {
                ref from,
                ref to,
                ..
            } if from == "nF" && to == "V"
        ));
        // CGPM 2022 prefixes (R/Q large, r/q small).
        assert_eq!(convert(1.0, "Qm", "m").unwrap(), 1e30);
        assert_eq!(convert(1.0, "Rm", "m").unwrap(), 1e27);
        assert_eq!(convert(1.0, "rm", "m").unwrap(), 1e-27);
        assert_eq!(convert(1.0, "qm", "m").unwrap(), 1e-30);
        // Non-SI units with exact definitions.
        assert_eq!(convert(1.0, "min", "s").unwrap(), 60.0);
        assert_eq!(convert(2.0, "h", "min").unwrap(), 120.0);
        assert_eq!(convert(1.0, "d", "h").unwrap(), 24.0);
        assert_eq!(convert(1.0, "L", "m^3").unwrap(), 1e-3);
        assert_eq!(convert(1.0, "mL", "L").unwrap(), 1e-3);
        assert_eq!(convert(1.0, "bar", "Pa").unwrap(), 1e5);
        assert_eq!(convert(1.0, "atm", "Pa").unwrap(), 101_325.0);
        assert_eq!(convert(1.0, "tonne", "kg").unwrap(), 1000.0);
        assert_eq!(convert(1.0, "nmi", "m").unwrap(), 1852.0);
        assert_eq!(convert(1.0, "ha", "m^2").unwrap(), 1e4);
        assert_eq!(convert(1.0, "acre", "m^2").unwrap(), 4_046.856_422_4);
        assert!((convert(1.0, "knot", "m/s").unwrap() - 1852.0 / 3600.0).abs() < 1e-15);
        assert!(
            (convert(1.0, "deg", "rad").unwrap() - core::f64::consts::PI / 180.0).abs() < 1e-15
        );
        assert!((convert(1.0, "eV", "J").unwrap() - 1.602_176_634e-19).abs() < 1e-34);
        assert_eq!(convert(1.0, "Wh", "J").unwrap(), 3600.0);
        // Ambiguous or angle-dependent symbols are refused, not guessed.
        let Error::AmbiguousUnit { why, remedy, .. } = parse_unit("cal").unwrap_err() else {
            panic!("cal must be AmbiguousUnit")
        };
        assert!(why.contains("4.184") && why.contains("4.1868"), "{why}");
        assert!(remedy.contains("J"), "{remedy}");
        let Error::AmbiguousUnit { why, remedy, .. } = parse_unit("rpm").unwrap_err() else {
            panic!("rpm must be AmbiguousUnit")
        };
        assert!(why.contains("rad/s"), "{why}");
        assert!(remedy.contains("Hz"), "{remedy}");
        // Both spellings of the micro sign are accepted; the ASCII `u` is not guessed
        assert!((parse_unit_spec("\u{00b5}F").unwrap().scale - 1e-6).abs() < 1e-20);
        assert!((parse_unit_spec("\u{03bc}F").unwrap().scale - 1e-6).abs() < 1e-20);
        assert!(parse_unit("uF").is_err());
        // Offset / logarithmic units: rejected explicitly (silently treating them as K /
        // dimensionless would compute wrong values)
        assert!(matches!(
            parse_unit("degC").unwrap_err(),
            Error::OffsetUnit(_)
        ));
        assert!(matches!(parse_unit("dBm").unwrap_err(), Error::LogUnit(_)));
        // Solid angle and radian are dimensionless
        assert!(parse_unit("sr").unwrap().is_zero());
    }

    #[test]
    fn formula_names_must_respect_declared_dimensions_and_be_unique() {
        let set = FormulaSet {
            decls: vec![
                Decl {
                    name: "F".into(),
                    quantity: "force".into(),
                    unit: Some("N".into()),
                },
                Decl {
                    name: "d".into(),
                    quantity: "distance".into(),
                    unit: Some("m".into()),
                },
                Decl {
                    name: "s".into(),
                    quantity: "stress".into(),
                    unit: Some("Pa".into()),
                },
            ],
            items: vec![FormulaItem {
                name: Some("s".into()),
                expr: div(var("F"), var("d")),
                line: 7,
            }],
            uses: vec![],
        };
        let err = dims_of_set(&set).unwrap_err();
        // Stable variant, not a sentence: name, both dimensions and the line are all payload.
        let Error::ResultDimensionMismatch {
            name,
            got,
            declared,
            line,
        } = err
        else {
            panic!("expected ResultDimensionMismatch: {err}")
        };
        assert_eq!(name, "s");
        assert_eq!(got, "kg·s^-2");
        assert_eq!(declared, "kg·m^-1·s^-2");
        assert_eq!(line, 7);

        // The same name is itself legal: the result of a = F/m exactly honors the acceleration
        // dimension in the decl.
        let mut compatible = set.clone();
        compatible.decls[2].unit = Some("N/m".into());
        // Once the same name is accepted, later entries reference the formula binding (s=F/d), not
        // an unexplainable dummy symbol.
        compatible.items.push(FormulaItem {
            name: Some("t".into()),
            expr: mul(var("s"), var("d")),
            line: 8,
        });
        let dims = dims_of_set(&compatible).unwrap();
        assert_eq!(dims["s"], parse_unit("N/m").unwrap());
        assert_eq!(dims["t"], parse_unit("N").unwrap());

        let duplicate = FormulaSet {
            decls: vec![],
            items: vec![
                FormulaItem {
                    name: Some("q".into()),
                    expr: num(1.0),
                    line: 1,
                },
                FormulaItem {
                    name: Some("q".into()),
                    expr: num(2.0),
                    line: 2,
                },
            ],
            uses: vec![],
        };
        let err = dims_of_set(&duplicate).unwrap_err();
        assert_eq!(err.code(), "duplicate-declaration", "{err}");
    }

    #[test]
    fn complex_division_survives_extremes() {
        // Regression: the naive `d = re²+im²` overflows or underflows when |b| is large/small,
        // so `1e300/1e10` became inf and `1e-200/1e-200` was falsely reported as division by
        // zero.
        assert_eq!(
            C::div(C::real(1e300), C::real(1e10)).unwrap().re,
            1e300 / 1e10
        );
        assert!((C::div(C::real(1e200), C::real(1e200)).unwrap().re - 1.0).abs() < 1e-15);
        assert!((C::div(C::real(1e-200), C::real(1e-200)).unwrap().re - 1.0).abs() < 1e-15);
        assert!(C::div(C::ONE, C::ZERO).is_err());
        // Complex: Smith's algorithm must not degenerate to NaN at extreme magnitudes either
        let big = C::new(1e300, 1e300);
        let q = C::div(big, big).unwrap();
        assert!((q.re - 1.0).abs() < 1e-12 && q.im.abs() < 1e-12, "{q:?}");
        // Real divided by real stays exact (does not take the complex formula)
        assert_eq!(C::div(C::real(1.0), C::real(4.0)).unwrap().re, 0.25);
    }

    #[test]
    fn unit_exponent_overflow_is_rejected_not_wrapped() {
        // Regression: i32 multiplication overflow (panic in debug, wrapping into a **wrong
        // dimension** in release)
        assert!(parse_unit("m^(20000)").is_err());
        assert!(parse_unit("m^(2000000000)").is_err());
        assert!(parse_unit("V^(1000000000)").is_err());
        // Within the legal range, as before
        assert_eq!(
            parse_unit("m^2").unwrap(),
            Dim::from_exponents([0, 2, 0, 0, 0, 0, 0])
        );
    }

    #[test]
    fn dim_display_round_trips_through_parse() {
        // Regression: `Dim::display` separates with `·` while parse_unit used to reject non-ASCII,
        // so "write the display back into a decl" was a broken path
        // Note that `J/(mol*K)` is not this dialect's notation: parentheses are used only for
        // **exponents**,
        // and grouping in a unit string relies on `/` being left-associative (`J/mol/K` =
        // J/(mol·K))
        for u in ["V/A*s^(-1/2)", "kg/m^3", "Pa*s", "J/mol/K", "m/s^2"] {
            let d = parse_unit(u).unwrap();
            let printed = d.display();
            let back = parse_unit(&printed).unwrap_or_else(|e| panic!("{u} → {printed}: {e}"));
            assert_eq!(d, back, "{u} 的回写 {printed} 不一致");
        }
    }

    #[test]
    fn nondifferentiable_operators_are_named() {
        // arg is not differentiable over the reals: `diff` gives NaN, but the caller must be able to
        // know **which** one is not differentiable
        let e = Expr::Call("arg".into(), vec![var("x")]);
        assert!(contains_nan_expr(&diff(&e, "x")));
        assert_eq!(nondiffable(&e), vec!["arg".to_string()]);
        // abs / conj / re / im are now symbolic-differentiable (real abs is differentiable)
        for f in ["abs", "conj", "re", "im"] {
            let e = Expr::Call(f.into(), vec![pow(var("x"), num(2.0))]);
            let d = diff(&e, "x");
            assert!(!contains_nan_expr(&d), "`{f}` 的导数不该是 NaN：{d:?}");
        }
        // d|u|/dx = u/|u| · u': at x=1 it should be 2 (u = x², u' = 2x)
        let e = Expr::Call("abs".into(), vec![pow(var("x"), num(2.0))]);
        let d = simplify(&diff(&e, "x"));
        let mut env = HashMap::new();
        env.insert("x".to_string(), 1.0);
        assert!((eval(&d, &env).unwrap() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn display_and_precedence_edge_cases() {
        // Power of a power: both sides must add parentheses, otherwise the round-trip changes the
        // association order
        let nested = pow(pow(var("x"), num(2.0)), num(3.0));
        assert!(
            to_latex(&nested).contains("(x^{2})^{3}"),
            "{}",
            to_latex(&nested)
        );
        assert!(
            to_matex(&nested).contains("(x ^ 2) ^ 3"),
            "{}",
            to_matex(&nested)
        );
        // The exponent is a power: `x^(2^3)`
        let e = pow(var("x"), pow(num(2.0), num(3.0)));
        assert!(to_matex(&e).contains("x ^ (2 ^ 3)"), "{}", to_matex(&e));
        // ∇ is a prefix operator: it must get parentheses as a base, otherwise (∇u)² and ∇(u²) look
        // the same
        let g = pow(grad(var("u")), num(2.0));
        assert!(to_latex(&g).contains("(\\nabla u)^{2}"), "{}", to_latex(&g));
        // Transpose as the base
        let t = pow(transpose(var("a")), num(2.0));
        assert!(to_latex(&t).contains("(a^{T})^{2}"), "{}", to_latex(&t));
    }

    #[test]
    fn matrix_division_is_rejected() {
        let a = Expr::Mat(vec![vec![num(2.0), num(4.0)]]);
        let b = Expr::Mat(vec![vec![num(2.0), num(2.0)]]);
        // Matrix / matrix = inversion, semantically confusing → reject
        let err = eval_value(&div(a.clone(), b), &HashMap::new()).unwrap_err();
        assert_eq!(err.code(), "matrix-division-unsupported", "{err}");
        assert!(err.render(crate::i18n::Locale::En).contains("inv"), "{err}");
        // Matrix / scalar = element-wise division: consistent with the emission lane (the Div branch
        // of `flatten` divides cell by cell)
        match eval_value(&div(a.clone(), num(2.0)), &HashMap::new()).unwrap() {
            Value::Matrix(m) => {
                assert_eq!(m.len(), 1);
                assert_eq!((m[0][0].re, m[0][1].re), (1.0, 2.0), "逐元素除");
            }
            other => panic!("矩阵除以标量应得矩阵，得到 {other:?}"),
        }
        // Scalar / matrix is undefined
        let err = eval_value(&div(num(2.0), a), &HashMap::new()).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("inv"), "{text}");
    }

    #[test]
    fn empty_sum_is_the_zero_element() {
        // An empty interval (lo > hi) is zero in all three places: the value layer, the real lane and
        // the complex lane
        let s = sum("i", num(5.0), num(3.0), var("i"));
        assert_eq!(eval(&s, &HashMap::new()).unwrap(), 0.0);
        assert!(emit_block(&s).contains("0.0"));
        assert!(emit_block_complex(&s).contains("(0.0, 0.0)"));
    }

    #[test]
    fn declared_j_keeps_its_parameter_slot() {
        // `j` is a reserved constant only when undeclared; if the user declared it, it must appear
        // in the signature
        let decls = vec![Decl {
            name: "j".into(),
            quantity: "current_density".into(),
            unit: Some("A/m^2".into()),
        }];
        let e = mul(var("j"), num(2.0));
        assert_eq!(params_in_decl_order(&decls, &e), vec!["j".to_string()]);
        // When undeclared it is excluded as before
        let r = vec![Decl {
            name: "r".into(),
            quantity: "resistance".into(),
            unit: Some("V/A".into()),
        }];
        assert!(params_in_decl_order(&r, &add(var("r"), var("j"))).contains(&"r".to_string()));
    }

    #[test]
    fn error_render_points_at_the_column() {
        let src = "decl { x : d; }\nx + undeclared_symbol";
        let e = FrontError::new(2, 5, "未声明的符号 `undeclared_symbol`");
        let text = e.render_with_source("demo.formula", src);
        // Three lines: header / source line (with a line-number gutter) / caret line
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 4, "{text}");
        assert!(lines[0].contains("demo.formula:2:5"), "{text}");
        assert!(lines[2].contains("x + undeclared_symbol"), "{text}");
        // The caret lands exactly on column 5 (4 spaces for zero-based + `^`)
        assert!(lines[3].ends_with("    ^"), "{text:?}");
        // With no position (line = 0) it falls back to the plain header
        let plain = FrontError::plain("语义错误").render_with_source("demo", src);
        assert_eq!(plain, "demo: 语义错误");
    }

    #[test]
    fn hostile_sources_are_rejected_not_crashed() {
        // 1) Invisible characters: zero-width spaces / bidirectional overrides / NUL are always
        // rejected (not as vague as "unknown character")
        for (src, payload) in [
            (
                "decl { x : d; } x\u{200b} + 1",
                "200B (zero-width character)",
            ),
            (
                "decl { x : d; } x + 1\u{202e}",
                "202E (bidirectional override)",
            ),
            ("decl { x : d; } x + 1\u{0}", "0000 (NUL)"),
            ("decl { x : d; } x + 1\u{feff}", "FEFF (byte order mark)"),
        ] {
            // Assert the stable code and the typed payload (code point + class) — not the
            // rendered sentence, whose wording is per-locale.
            let err = scan_source(src).unwrap_err();
            assert_eq!(err.code.code(), "invisible-character", "{src:?}");
            match &*err.code {
                i18n::Error::InvisibleCharacter(p) => assert_eq!(p, payload, "{src:?}"),
                other => panic!("{src:?} → {other:?}"),
            }
        }
        // Normal tabs and newlines must pass
        assert!(scan_source("decl {\n\tx : d;\n}\nx + 1").is_ok());

        // 2) Depth limit: excessively deep nesting gives a **numeric error**, not a stack overflow
        // (the same guard at the parse layer is covered by the latex front-end unit tests — the
        // dependency direction can only be latex → core)
        let mut deep = var("x");
        for _ in 0..MAX_DEPTH + 5 {
            deep = neg(deep);
        }
        let err = check_depth(&deep).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("nesting too deep"), "{text}");
        assert!(depth_of(&var("x")) == 1);
        let mut ok = var("x");
        for _ in 0..50 {
            ok = neg(ok);
        }
        assert!(check_depth(&ok).is_ok(), "50 层应放行");
    }

    #[test]
    fn sum_step_cap_prevents_dos() {
        // Σ(i, 1, 1e9) used to "run all night"; now it is an explicit error
        let s = sum("i", num(1.0), num(1.0e9), var("i"));
        let err = eval_value(&s, &HashMap::new()).unwrap_err();
        assert_eq!(err.code(), "invalid-input", "{err}");
        assert!(
            err.render(crate::i18n::Locale::En).contains("limit"),
            "{err}"
        );
        // Small intervals as usual
        let small = sum("i", num(1.0), num(10.0), var("i"));
        assert_eq!(eval(&small, &HashMap::new()).unwrap(), 55.0);
    }

    #[test]
    fn scalar_valued_matrix_ops_work_as_exponents_and_det_args() {
        // Regression: the exponent side used matrix_ish (an over-approximation), so `2 ^ det(J)` was
        // wrongly rejected —
        // while the emission lane got the same expression right. Both sides now use the same
        // (precise) predicate.
        let j = Expr::Mat(vec![vec![num(1.0), num(2.0)], vec![num(3.0), num(4.0)]]);
        let env = HashMap::new();
        // det(J) = -2 → 2^det = 0.25
        let e = pow(num(2.0), Expr::Call("det".into(), vec![j.clone()]));
        let v = eval_value(&e, &env).unwrap().as_scalar().unwrap();
        assert!((v.re - 0.25).abs() < 1e-12, "{v:?}");
        // norm(v) as the exponent
        let vv = Expr::Mat(vec![vec![num(3.0), num(4.0)]]);
        let e = pow(num(2.0), Expr::Call("norm".into(), vec![vv]));
        let v = eval_value(&e, &env).unwrap().as_scalar().unwrap();
        assert!((v.re - 32.0).abs() < 1e-12, "2^5 = 32：{v:?}");
        // A genuine matrix as the exponent → still rejected
        let bad = pow(num(2.0), j.clone());
        let err = eval_value(&bad, &env).unwrap_err();
        assert_eq!(err.code(), "matrix-scalar-mix", "{err}");
        assert!(err
            .render(crate::i18n::Locale::En)
            .contains("cannot be an exponent"));
        // Dimension penetration of det(inv(J)): dim(inv(J)) = J⁻¹, det = J⁻²
        let decls = vec![Decl {
            name: "c".into(),
            quantity: "cell".into(),
            unit: Some("m".into()),
        }];
        let dims = dims_of(&decls).unwrap();
        let cells = Expr::Mat(vec![vec![var("c"), var("c")], vec![var("c"), var("c")]]);
        let inner = Expr::Call("inv".into(), vec![cells]);
        let d = dim_check(&Expr::Call("det".into(), vec![inner]), &dims).unwrap();
        assert_eq!(d, parse_unit("m^-2").unwrap(), "det(inv(J)) 量纲 = m^-2");
    }

    #[test]
    fn canonical_form_keeps_use_directives() {
        // Regression: canonicalization used to drop the whole `use "..."` line — raising no error,
        // it only said "undeclared symbol" on the next build, the hardest kind of accident to
        // trace.
        let mut set = FormulaSet::default();
        set.uses.push("lib/atmosphere.formula".into());
        set.decls.push(Decl {
            name: "x".into(),
            quantity: "d".into(),
            unit: None,
        });
        set.items.push(FormulaItem {
            name: Some("y".into()),
            expr: var("x"),
            line: 2,
        });
        let text = format_set(&set);
        let use_pos = text
            .find("use \"lib/atmosphere.formula\"")
            .expect("use 必须保留");
        let decl_pos = text.find("decl {").expect("decl 必须在");
        assert!(use_pos < decl_pos, "`use` 必须排在声明之前：\n{text}");
    }

    #[test]
    fn linear_solve_works_and_rejects_nonlinearity() {
        // a·x + b = 0 → x = -b/a
        // Check the solution **numerically** (simplify rearranges the tree, so keys cannot compare
        // equivalent expressions)
        let mut env = HashMap::new();
        env.insert("a".to_string(), 2.0);
        env.insert("b".to_string(), -6.0);
        env.insert("k1".to_string(), 3.0);
        env.insert("k2".to_string(), 4.0);
        env.insert("x".to_string(), 0.0);
        let e = add(mul(var("a"), var("x")), var("b"));
        let x = solve_linear(&e, "x").unwrap();
        // a·x + b = 0 → x = -b/a = 3
        assert!((eval(&x, &env).unwrap() - 3.0).abs() < 1e-12, "{x:?}");
        // k1·a − k2·x = 0 → x = k1·a/k2 = 1.5
        let e2 = sub(mul(var("k1"), var("a")), mul(var("k2"), var("x")));
        let x2 = solve_linear(&e2, "x").unwrap();
        assert!((eval(&x2, &env).unwrap() - 1.5).abs() < 1e-12, "{x2:?}");
        // Substituting back into the original must give zero (the real criterion for "solved
        // correctly")
        env.insert("x".to_string(), eval(&x2, &env).unwrap());
        assert!(eval(&e2, &env).unwrap().abs() < 1e-12);

        // Non-linearity: must error and state clearly which kind it is
        for (src, want) in [
            (mul(var("x"), var("x")), "twice in one product"),
            (div(num(1.0), var("x")), "denominator"),
            (pow(var("x"), num(2.0)), "power"),
            (Expr::Call("exp".into(), vec![var("x")]), "inside `exp`"),
        ] {
            // Every one of these is the **same** catalogued variant; the differing part is its
            // `why` payload, which is what this loop asserts.
            let err = solve_linear(&src, "x").unwrap_err();
            assert_eq!(err.code(), "nonlinear-equation", "{err}");
            assert!(
                err.render(i18n::Locale::En).contains(want),
                "expected `{want}` in {err}"
            );
        }
        // The equation has no first-degree term in x: it must be rejected (not solved), and the
        // message must name the variable it could not solve for. (Staging: this site is not yet
        // a catalogue variant — pinned by payload, not by "is still staging".)
        let err = solve_linear(&add(var("a"), var("b")), "x").unwrap_err();
        assert!(
            err.render(i18n::Locale::En).contains('x'),
            "the rejection must name the variable: {err}"
        );
    }

    #[test]
    fn solve_system_solves_and_roundtrips_to_zero() {
        // Two-mass spring: (k1+k2)x1 − k2·x2 = F1; −k2·x1 + (k2+k3)·x2 = F2
        let k1 = var("k1");
        let k2 = var("k2");
        let k3 = var("k3");
        let f1 = var("F1");
        let f2 = var("F2");
        let (x, y) = (var("x1"), var("x2"));
        let eq1 = sub(
            sub(
                mul(k1.clone(), x.clone()),
                mul(k2.clone(), sub(y.clone(), x.clone())),
            ),
            f1.clone(),
        );
        let eq2 = sub(
            add(
                mul(k2.clone(), sub(y.clone(), x.clone())),
                mul(k3.clone(), y.clone()),
            ),
            f2.clone(),
        );
        let sol = solve_system(&[eq1.clone(), eq2.clone()], &["x1".into(), "x2".into()]).unwrap();
        let Expr::Mat(rows) = &sol else {
            panic!("解必须是列向量矩阵，得到 {sol:?}");
        };
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.len() == 1), "列向量形状 2×1");

        // The solved entries must not still contain unknowns (otherwise it is "asking for it while
        // requiring it as input")
        for r in rows {
            for name in vars_of(&r[0]) {
                assert!(
                    !["x1", "x2"].contains(&name.as_str()),
                    "解里还含未知数 `{name}`"
                );
            }
        }

        // Numeric substitution: plugging the solution into the original equations must give ≈ 0 (the
        // real criterion for "solved correctly")
        let mut env = HashMap::new();
        env.insert("k1".to_string(), 1.0);
        env.insert("k2".to_string(), 2.0);
        env.insert("k3".to_string(), 3.0);
        env.insert("F1".to_string(), 10.0);
        env.insert("F2".to_string(), 20.0);
        env.insert("x1".to_string(), eval(&rows[0][0], &env).unwrap());
        env.insert("x2".to_string(), eval(&rows[1][0], &env).unwrap());
        assert!(
            eval(&eq1, &env).unwrap().abs() < 1e-12,
            "eq1 残差 {}",
            eval(&eq1, &env).unwrap()
        );
        assert!(
            eval(&eq2, &env).unwrap().abs() < 1e-12,
            "eq2 残差 {}",
            eval(&eq2, &env).unwrap()
        );

        // Consistent with a hand calculation by Cramer's rule:
        // det = (k1+k2)(k2+k3) − k2² = 3·5 − 4 = 11
        // x1 = [F1(k2+k3) + F2·k2]/det = (50 + 40)/11 = 90/11
        // x2 = [F2(k1+k2) + F1·k2]/det = (60 + 20)/11 = 80/11
        let det = 3.0 * 5.0 - 2.0 * 2.0;
        assert!((env["x1"] - 90.0 / det).abs() < 1e-12, "x1 = {}", env["x1"]);
        assert!((env["x2"] - 80.0 / det).abs() < 1e-12, "x2 = {}", env["x2"]);
    }

    #[test]
    fn roots_solves_linear_and_quadratic_exactly() {
        let mut env = HashMap::new();
        for (k, v) in [("a", 2.0), ("b", -6.0), ("k", 3.0), ("F", 12.0)] {
            env.insert(k.to_string(), v);
        }
        // Degree 1: 2x − 6 = 0 → [3] (a 1×1 column vector)
        let lin = solve_poly(&sub(mul(var("a"), var("x")), num(6.0)), "x").unwrap();
        let Expr::Mat(r) = &lin else {
            panic!("必须是矩阵")
        };
        assert_eq!((r.len(), r[0].len()), (1, 1));
        assert!((eval(&r[0][0], &env).unwrap() - 3.0).abs() < 1e-12);

        // Degree 2: x² − 5x + 6 = 0 → 2 and 3 (order: −b−√D first)
        let quad = solve_poly(
            &add(
                sub(pow(var("x"), num(2.0)), mul(num(5.0), var("x"))),
                num(6.0),
            ),
            "x",
        )
        .unwrap();
        let Expr::Mat(r) = &quad else {
            panic!("必须是矩阵")
        };
        assert_eq!((r.len(), r[0].len()), (2, 1), "两个根，列向量");
        let mut got: Vec<f64> = r.iter().map(|row| eval(&row[0], &env).unwrap()).collect();
        got.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((got[0] - 2.0).abs() < 1e-12, "{got:?}");
        assert!((got[1] - 3.0).abs() < 1e-12, "{got:?}");

        // Plugging the roots back into the **original polynomial** must give zero (independent of
        // the correctness of the root formula itself)
        for g in &got {
            let mut e2 = env.clone();
            e2.insert("x".to_string(), *g);
            let orig = add(
                sub(pow(var("x"), num(2.0)), mul(num(5.0), var("x"))),
                num(6.0),
            );
            assert!(eval(&orig, &e2).unwrap().abs() < 1e-12, "代回残差");
        }

        // Multiple root: x² − 2x + 1 = 0 → two 1s (repeated by multiplicity)
        let dbl = solve_poly(
            &add(
                sub(pow(var("x"), num(2.0)), mul(num(2.0), var("x"))),
                num(1.0),
            ),
            "x",
        )
        .unwrap();
        let Expr::Mat(r) = &dbl else { panic!() };
        for row in r {
            let v = eval(&row[0], &env).unwrap();
            assert!((v - 1.0).abs() < 1e-12, "{v}");
        }

        // High-degree zero coefficients must be peeled off: x³ − x³ + x − 1 is still degree 1
        let cancel = sub(
            add(
                sub(pow(var("x"), num(3.0)), pow(var("x"), num(3.0))),
                var("x"),
            ),
            num(1.0),
        );
        let Expr::Mat(r) = &solve_poly(&cancel, "x").unwrap() else {
            panic!("必须是矩阵")
        };
        assert_eq!(r.len(), 1, "高阶项抵消后是 1 次");
        assert!((eval(&r[0][0], &env).unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn quadratic_roots_are_stable_and_preserve_the_documented_order() {
        let poly = add(
            add(
                mul(var("a"), pow(var("x"), num(2.0))),
                mul(var("b"), var("x")),
            ),
            var("c"),
        );
        let roots = solve_poly(&poly, "x").unwrap();
        let Expr::Mat(rows) = roots else {
            panic!("roots must be a column vector")
        };
        let eval_at = |a: f64, b: f64, c: f64, index: usize| {
            let env = HashMap::from([
                ("a".to_string(), a),
                ("b".to_string(), b),
                ("c".to_string(), c),
            ]);
            eval_value(&rows[index][0], &env)
                .unwrap()
                .as_scalar()
                .unwrap()
        };
        // −b−√D first: with b<0 the small root comes first, with b>0 the large root comes first. The
        // order is not swapped by the stabilization.
        let small = eval_at(1.0, -1.0e8, 1.0, 0);
        assert!((small.re - 1.0e-8).abs() / 1.0e-8 < 1.0e-12, "{small:?}");
        assert!(small.im == 0.0);
        assert_eq!(eval_at(1.0, -1.0e8, 1.0, 1).re, 1.0e8);
        assert!((eval_at(1.0, 1.0e8, 1.0, 0).re + 1.0e8).abs() / 1.0e8 < 1.0e-15);
        assert!((eval_at(1.0, 1.0e8, 1.0, 1).re + 1.0e-8).abs() / 1.0e-8 < 1.0e-12);
        // Neither a zero double root nor an ordinary double root may take the 0/0 branch of c/q.
        assert_eq!(eval_at(1.0, 0.0, 0.0, 0), C::ZERO);
        assert_eq!(eval_at(1.0, 0.0, 0.0, 1), C::ZERO);
        assert_eq!(eval_at(1.0, 2.0, 1.0, 0).re, -1.0);
        assert_eq!(eval_at(1.0, 2.0, 1.0, 1).re, -1.0);
        // x²+1: the complex value layer still gives −j, +j (consistent with the original formula
        // order).
        assert!(eval_at(1.0, 0.0, 1.0, 0).im < 0.0);
        assert!(eval_at(1.0, 0.0, 1.0, 1).im > 0.0);
    }

    #[test]
    fn numeric_roots_finds_all_roots_including_complex() {
        // (x−1)(x−2)(x−3) = x³ − 6x² + 11x − 6
        let r = numeric_roots(&[-6.0, 11.0, -6.0, 1.0]).unwrap();
        assert_eq!(r.len(), 3, "三次必须有三个根（含重数）");
        let mut re: Vec<f64> = r.iter().map(|c| c.re).collect();
        re.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for (got, want) in re.iter().zip([1.0, 2.0, 3.0]) {
            assert!((got - want).abs() < 1e-10, "实根 {got} ≠ {want}");
        }
        assert!(r.iter().all(|c| c.im.abs() < 1e-10), "应当全是实根");

        // x² + 1 → ±j (purely imaginary roots must really be computed)
        let r2 = numeric_roots(&[1.0, 0.0, 1.0]).unwrap();
        assert_eq!(r2.len(), 2);
        let ims: Vec<f64> = r2.iter().map(|c| c.im).collect();
        assert!(ims.iter().any(|v| (*v - 1.0).abs() < 1e-12), "{ims:?}");
        assert!(ims.iter().any(|v| (*v + 1.0).abs() < 1e-12), "{ims:?}");
        assert!(r2.iter().all(|c| c.re.abs() < 1e-12));

        // x³ + 1 → one real root −1 and a pair of complex roots (angles ±60°)
        let r3 = numeric_roots(&[1.0, 0.0, 0.0, 1.0]).unwrap();
        assert_eq!(r3.len(), 3);
        assert!(r3
            .iter()
            .any(|c| (c.re + 1.0).abs() < 1e-10 && c.im.abs() < 1e-10));
        // Substitute back: |z³ + 1| must be zero
        for z in &r3 {
            let z2 = z.mul(*z);
            let z3 = z2.mul(*z);
            let res = z3.add(C::ONE);
            assert!(res.abs() < 1e-8, "z³+1 残差 {res:?}");
        }

        // Multiple root: (x−2)² = x² − 4x + 4 (numerical root finding on multiple roots is a
        // well-known difficulty, so a looser criterion is allowed)
        let r4 = numeric_roots(&[4.0, -4.0, 1.0]).unwrap();
        assert_eq!(r4.len(), 2);
        for z in &r4 {
            assert!((z.re - 2.0).abs() < 1e-6, "重根 {z:?}");
        }

        // Boundaries: zero polynomial / constant / linear
        assert!(numeric_roots(&[0.0, 0.0]).is_err());
        assert!(numeric_roots(&[5.0]).is_err());
        let lin = numeric_roots(&[3.0, 2.0]).unwrap();
        assert!((lin[0].re + 1.5).abs() < 1e-15);
        // High-degree zero coefficients are peeled off
        assert_eq!(numeric_roots(&[3.0, 2.0, 0.0, 0.0]).unwrap().len(), 1);
        // Non-finite coefficients
        assert!(numeric_roots(&[f64::NAN, 1.0]).is_err());
    }

    #[test]
    fn numeric_roots_never_reports_garbage() {
        // Four counterexamples from an independent review; all must "either give the right numbers or
        // report an error":
        // ① after overflow, f64::max swallowed the NaN, once returning Ok(all NaN)
        // ② noise zeroing once zeroed the **real part** too, wiping a real small root to 0
        // ③ with the Cauchy bound as the initial radius, healthy polynomials were misreported as
        // "ill-conditioned"
        // ④ the behavior of the zeroing criterion on small purely imaginary roots

        // ① Wilkinson type: ∏(x−k), k=1..20 (coefficients reach 20!, with wildly different
        // magnitudes after making it monic)
        let mut coeffs = vec![1.0f64];
        for k in 1..=20 {
            let mut next = vec![0.0; coeffs.len() + 1];
            for (i, c) in coeffs.iter().enumerate() {
                next[i] += -(k as f64) * c;
                next[i + 1] += c;
            }
            coeffs = next;
        }
        match numeric_roots(&coeffs) {
            Ok(rs) => {
                // If results are given, every root must be finite and the substitution residual
                // controllable
                for z in &rs {
                    assert!(
                        z.re.is_finite() && z.im.is_finite(),
                        "根必须是有限数：{z:?}"
                    );
                    let mut acc = C::new(coeffs[coeffs.len() - 1], 0.0);
                    for c in coeffs.iter().rev().skip(1) {
                        acc = acc.mul(*z).add(C::new(*c, 0.0));
                    }
                    assert!(acc.abs() < 1e-4, "根 {z:?} 代回残差 {}", acc.abs());
                }
                assert_eq!(rs.len(), 20);
            }
            // An error is acceptable (degree 20 is a deliberately hard case), but it **must not
            // return NaN**
            Err(e) => {
                assert!(
                    matches!(e, Error::RootFindingFailed { .. }),
                    "a failed run must be catalogued: {e:?}"
                );
                let text = e.render(i18n::Locale::En);
                assert!(text.contains("root finding failed"), "{text}")
            }
        }

        // ② A real small root must not be wiped to 0: the roots of x² − 1e-26 are ±1e-13
        let small = numeric_roots(&[-1e-26, 0.0, 1.0]).unwrap();
        assert_eq!(small.len(), 2);
        for z in &small {
            // The criterion is "not wiped to 0 and of the right magnitude", without demanding the
            // last bit (at this scale the relative error
            // is of order 1e-4 anyway)
            assert!(
                (z.re.abs() - 1e-13).abs() / 1e-13 < 1e-2,
                "小根被抹平了：{z:?}（应为 ±1e-13 量级）"
            );
        }

        // ③ Healthy polynomials must not be judged ill-conditioned: ∏(x−k), k=1..8
        let mut c8 = vec![1.0f64];
        for k in 1..=8 {
            let mut next = vec![0.0; c8.len() + 1];
            for (i, c) in c8.iter().enumerate() {
                next[i] += -(k as f64) * c;
                next[i + 1] += c;
            }
            c8 = next;
        }
        let r8 = numeric_roots(&c8).expect("∏(x−k), k=1..8 是健康多项式，不该报病态");
        let mut re8: Vec<f64> = r8.iter().map(|z| z.re).collect();
        re8.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for (k, got) in re8.iter().enumerate() {
            assert!((got - (k as f64 + 1.0)).abs() < 1e-6, "第 {k} 个根 {got}");
        }

        // ④ Small purely imaginary roots: x² + 1e-26 → ±1e-13j (the imaginary part is a real value
        // and must not be cleared as residue)
        let imag = numeric_roots(&[1e-26, 0.0, 1.0]).unwrap();
        for z in &imag {
            assert!(
                (z.im.abs() - 1e-13).abs() / 1e-13 < 1e-2,
                "纯虚小根被清掉了：{z:?}"
            );
        }
    }

    #[test]
    fn numeric_roots_handles_tiny_roots_exactly_or_refuses() {
        // S0 from the third review round: the denominator of the residual acceptance was wrong (it
        // stopped at |a_n|), and together with the
        // absolute lower bound on the initial radius (geo > 1e-8), small-root polynomials were
        // **silently returned with wrong roots**
        // — `x²−1e-100` once returned 4.03e-25 (off by 4e25×) without reporting an error.
        for (eps, want) in [
            (1e-26f64, 1e-13f64),
            (1e-45, 3.1622776601683793e-23),
            (1e-50, 1e-25),
            (1e-60, 1e-30),
            (1e-100, 1e-50),
            (1e-300, 1e-150),
        ] {
            let r = match numeric_roots(&[-eps, 0.0, 1.0]) {
                Ok(r) => r,
                Err(_) => continue, // an error is acceptable; silently giving a wrong value is not
            };
            for z in &r {
                let rel = (z.re.abs() - want).abs() / want;
                assert!(rel < 1e-6, "eps={eps:e}：根 {z:?} 与真值 {want:e} 不符");
            }
        }
        // Larger genuine roots must be right as well
        let r = numeric_roots(&[-1e-12, 0.0, 1.0]).unwrap();
        for z in &r {
            assert!((z.re.abs() - 1e-6).abs() / 1e-6 < 1e-9, "{z:?}");
        }
        // A linear equation overflow must error (it once returned Ok(-inf))
        let err = numeric_roots(&[1e300, 1e-300]).unwrap_err();
        assert_eq!(err.code(), "non-finite-value", "{err}");
        let en = err.render(i18n::Locale::En);
        assert!(en.contains("overflows") && en.contains("rescale"), "{en}");
    }

    #[test]
    fn poly_coeffs_extracts_low_to_high_and_round_trips_with_roots() {
        let mut env = HashMap::new();
        env.insert("a".to_string(), 2.0);
        env.insert("b".to_string(), 3.0);
        env.insert("c".to_string(), 1.0);
        // a·x² − b·x + c = 2x² − 3x + 1 → coefficients [1, −3, 2] (low degree to high degree)
        let poly = add(
            sub(
                mul(var("a"), pow(var("x"), num(2.0))),
                mul(var("b"), var("x")),
            ),
            var("c"),
        );
        let Expr::Mat(rows) = poly_coeffs_vec(&poly, "x").unwrap() else {
            panic!("系数应是 1×k 行向量")
        };
        assert_eq!(rows.len(), 1, "行向量");
        let got: Vec<f64> = rows[0].iter().map(|e| eval(e, &env).unwrap()).collect();
        assert_eq!(got, vec![1.0, -3.0, 2.0], "低次到高次");

        // Complementary to roots: coefficients → find roots → substitute back to get zero
        let r = solve_poly(&poly, "x").unwrap();
        let Expr::Mat(rr) = &r else { panic!() };
        let mut roots: Vec<f64> = rr.iter().map(|row| eval(&row[0], &env).unwrap()).collect();
        roots.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((roots[0] - 0.5).abs() < 1e-12, "{roots:?}");
        assert!((roots[1] - 1.0).abs() < 1e-12, "{roots:?}");

        // High-degree zero coefficients must be peeled off: x³ − x³ − 2x + 1 → [1, −2]
        let cancel = add(
            sub(
                sub(pow(var("x"), num(3.0)), pow(var("x"), num(3.0))),
                mul(num(2.0), var("x")),
            ),
            num(1.0),
        );
        let Expr::Mat(rows) = poly_coeffs_vec(&cancel, "x").unwrap() else {
            panic!()
        };
        let got: Vec<f64> = rows[0]
            .iter()
            .map(|e| eval(e, &HashMap::new()).unwrap())
            .collect();
        assert_eq!(got, vec![1.0, -2.0], "高次抵消后是 1 次");

        // Non-polynomials / the zero polynomial must be rejected
        for bad in [
            Expr::Call("sin".into(), vec![var("x")]),
            div(num(1.0), var("x")),
        ] {
            let err = poly_coeffs_vec(&bad, "x").unwrap_err();
            let text = err.to_string();
            assert!(text.contains("一元多项式"), "{text}");
        }
        assert!(poly_coeffs_vec(&mul(num(0.0), var("x")), "x").is_err());
    }

    #[test]
    fn numeric_roots_keeps_imaginary_parts_of_small_complex_roots() {
        // S0 from the fourth review round: zeroing the imaginary part used an **absolute** threshold
        // `1e-12·(1+|re|)`,
        // and for complex roots with modulus below 1e-12 the imaginary part is all the information
        // the root has, so it was wiped entirely:
        // `x³−1e-60` once returned three real roots (the latter two with relative error 0.87) without
        // reporting an error.
        let r = numeric_roots(&[-1e-60, 0.0, 0.0, 1.0]).expect("三次应可解");
        assert_eq!(r.len(), 3);
        let real_count = r.iter().filter(|c| c.im == 0.0).count();
        assert_eq!(real_count, 1, "x³−1e-60 只有一个实根：{r:?}");
        // All three roots must satisfy z³ = 1e-60 (relative criterion)
        for z in &r {
            let z3 = z.mul(*z).mul(*z);
            let rel = z3.sub(C::new(1e-60, 0.0)).abs() / 1e-60;
            assert!(rel < 1e-6, "根 {z:?} 代回相对残差 {rel}");
        }
        // Likewise for degree five: x⁵−1e-100 has 4 complex roots
        let r5 = numeric_roots(&[-1e-100, 0.0, 0.0, 0.0, 0.0, 1.0]).expect("五次应可解");
        let re5 = r5.iter().filter(|c| c.im == 0.0).count();
        assert_eq!(re5, 1, "x⁵−1e-100 只有一个实根：{r5:?}");
        for z in &r5 {
            let mut acc = *z;
            for _ in 0..4 {
                acc = acc.mul(*z);
            }
            let rel = acc.sub(C::new(1e-100, 0.0)).abs() / 1e-100;
            assert!(rel < 1e-5, "根 {z:?} 代回相对残差 {rel}");
        }
        // Ordinary real roots must still be cleaned completely (switching to a relative criterion
        // must not leave 1e-16 noise behind)
        let r2 = numeric_roots(&[-6.0, 5.0, 1.0]).unwrap(); // x²+5x−6 → 1 与 −6
        assert!(r2.iter().all(|c| c.im == 0.0), "{r2:?}");
    }

    #[test]
    fn roots_rejects_high_degree_and_non_polynomial() {
        // Degree 3: rejected explicitly and the degree reported (no Cardano approximation)
        let cubic = add(pow(var("x"), num(3.0)), num(1.0));
        let err = solve_poly(&cubic, "x").unwrap_err();
        assert_eq!(err.code(), "invalid-input", "{err}");
        let en = err.render(i18n::Locale::En);
        assert!(en.contains("degree 3") && en.contains("degree 1–2"), "{en}");
        // Non-polynomial: x inside a function argument / in the denominator / at a non-integer
        // power
        for bad in [
            Expr::Call("sin".into(), vec![var("x")]),
            div(num(1.0), var("x")),
            pow(var("x"), num(-1.0)),
        ] {
            let err = solve_poly(&bad, "x").unwrap_err();
            let text = err.to_string();
            assert!(text.contains("多项式"), "{text}");
        }
        // The equation has no x
        let err = solve_poly(&add(var("a"), var("b")), "x").unwrap_err();
        assert_eq!(err.code(), "nonlinear-equation", "{err}");
        assert!(
            err.render(i18n::Locale::En).contains("does not contain"),
            "{err}"
        );
    }

    #[test]
    fn solve_system_rejects_nonuniq_and_nonlinear() {
        let (x, y) = (var("x"), var("y"));
        // Underdetermined: 1 equation, 2 unknowns
        let err =
            solve_system(&[sub(x.clone(), y.clone())], &["x".into(), "y".into()]).unwrap_err();
        let text = err.to_string();
        assert_eq!(err.code(), "underdetermined", "{text}");
        // Contradiction: x − 1 and x − 2 holding at the same time
        let err = solve_system(
            &[sub(x.clone(), num(1.0)), sub(x.clone(), num(2.0))],
            &["x".into()],
        )
        .unwrap_err();
        assert_eq!(err.code(), "inconsistent", "{err}");
        // Non-linear: x·y appears on both sides of a multiplication
        let err = solve_system(
            &[sub(mul(x.clone(), y.clone()), num(1.0))],
            &["x".into(), "y".into()],
        )
        .unwrap_err();
        assert_eq!(err.code(), "nonlinear-equation", "{err}");
        // Duplicate unknown names
        let err = solve_system(&[sub(x.clone(), num(1.0))], &["x".into(), "x".into()]).unwrap_err();
        assert_eq!(err.code(), "invalid-input", "{err}");
        assert!(err.render(i18n::Locale::En).contains("twice"), "{err}");
        // Empty list
        let err = solve_system(&[sub(x.clone(), num(1.0))], &[]).unwrap_err();
        assert_eq!(err.code(), "invalid-input", "{err}");
        assert!(
            err.render(i18n::Locale::En).contains("at least one"),
            "{err}"
        );
        // A numerically redundant equation (a third one of the form 2·eq1): after elimination to
        // zero it should still give a unique solution
        let eq1 = sub(add(x.clone(), y.clone()), num(3.0));
        let eq2 = sub(x.clone(), y.clone());
        let eq3 = sub(mul(num(2.0), add(x.clone(), y.clone())), num(6.0));
        let sol = solve_system(&[eq1, eq2, eq3], &["x".into(), "y".into()]).unwrap();
        let Expr::Mat(rows) = &sol else {
            panic!("解必须是矩阵");
        };
        let mut env = HashMap::new();
        env.insert("x".to_string(), eval(&rows[0][0], &env).unwrap());
        env.insert("y".to_string(), eval(&rows[1][0], &env).unwrap());
        // x + y = 3, x − y = 0 → (1.5, 1.5)
        assert!((env["x"] - 1.5).abs() < 1e-12 && (env["y"] - 1.5).abs() < 1e-12);
    }

    #[test]
    fn solve_rejects_hidden_unknown_in_exponent() {
        // Review S0-1: the x in `2^x` used to be taken as a constant term — as long as that unknown
        // also appeared as a
        // linear term in the same equation, the whole term ended up in the "solution":
        // solve([x + 2^x - 3], [x]) once gave 3−2^x,
        // which once generated became a legal function "taking the solved-for quantity as an input"
        // that silently returned wrong values.
        let (x, y) = (var("x"), var("y"));
        // The system lane
        let err = solve_system(
            &[sub(add(x.clone(), pow(num(2.0), x.clone())), num(3.0))],
            &["x".into()],
        )
        .unwrap_err();
        match &err {
            // The multi-variable lane identifies the equation, not one variable, so only the
            // reason is pinned here.
            Error::NonlinearEquation { why, .. } => {
                assert!(why.contains("exponent"), "{why}");
            }
            other => panic!("{other:?}"),
        }
        // The single-variable lane has the same disease (a pre-existing defect, fixed in the same
        // place)
        let err = solve_linear(
            &sub(add(x.clone(), pow(num(2.0), x.clone())), num(3.0)),
            "x",
        )
        .unwrap_err();
        assert!(
            matches!(
                &err,
                Error::NonlinearEquation { why, .. } if why.contains("exponent")
            ),
            "{err}"
        );
        // But `2^y` contains no x: solving for x is itself legal (x = 2^y is the true solution)
        let ok = solve_system(&[sub(x.clone(), pow(num(2.0), y.clone()))], &["x".into()])
            .expect("2^y 不含 x，应可解");
        let Expr::Mat(m) = &ok else {
            panic!("应是列向量")
        };
        let mut env = HashMap::new();
        env.insert("y".to_string(), 3.0);
        assert!((eval(&m[0][0], &env).unwrap() - 8.0).abs() < 1e-12);
    }

    #[test]
    fn solve_system_detects_structurally_zero_coefficients() {
        // Review S0-2: `simplify` does not combine like terms, so `k2 − k2` cannot be folded, and
        // rows with zero coefficients were taken as pivots → 0/0 (an error at the evaluation layer /
        // NaN at the emission layer) or an underdetermined system taken as unique.
        let (x, y) = (var("x"), var("y"));
        let k2 = var("k2");
        // A: the redundant row `(k2−k2)·y + x − 1` is identical to `x − 1` → only x is pinned, y is
        // free → underdetermined
        let err = solve_system(
            &[
                sub(x.clone(), num(1.0)),
                sub(
                    add(mul(sub(k2.clone(), k2.clone()), y.clone()), x.clone()),
                    num(1.0),
                ),
            ],
            &["x".into(), "y".into()],
        )
        .unwrap_err();
        let text = err.to_string();
        assert_eq!(err.code(), "underdetermined", "{text}");
        // B: the second = the first × k → equally underdetermined (not 0/0)
        let k = var("k");
        let err = solve_system(
            &[
                sub(add(x.clone(), y.clone()), num(1.0)),
                sub(mul(k.clone(), add(x.clone(), y.clone())), k.clone()),
            ],
            &["x".into(), "y".into()],
        )
        .unwrap_err();
        let text = err.to_string();
        assert_eq!(err.code(), "underdetermined", "{text}");
        // The structural zero test itself: several forms of combining like terms
        assert!(is_structurally_zero(&sub(k2.clone(), k2.clone())), "k−k");
        assert!(
            is_structurally_zero(&sub(mul(k.clone(), x.clone()), mul(x.clone(), k.clone()))),
            "k·x − x·k"
        );
        assert!(
            is_structurally_zero(&sub(sub(mul(num(2.0), x.clone()), x.clone()), x.clone())),
            "2x − x − x"
        );
        assert!(
            is_structurally_zero(&sub(
                mul(x.clone(), pow(x.clone(), num(2.0))),
                pow(x.clone(), num(3.0))
            )),
            "x·x² − x³"
        );
        // A power differing by one degree must **never** be judged zero: `a²−a = a(a−1)` is
        // genuinely non-zero.
        // (The power branch of the first `term_factors` pushed only n−1 factors, so both of these
        // were judged identically zero,
        //   and the solver then silently gave solutions that do not satisfy the original equations —
        // an S0 caught by the independent review.)
        for (bad, why) in [
            (sub(pow(k2.clone(), num(2.0)), k2.clone()), "a²−a"),
            (
                sub(
                    pow(mul(num(2.0), k2.clone()), num(2.0)),
                    mul(num(2.0), k2.clone()),
                ),
                "(2a)²−2a",
            ),
            (
                sub(
                    pow(mul(x.clone(), k2.clone()), num(2.0)),
                    mul(x.clone(), k2.clone()),
                ),
                "(xy)²−xy",
            ),
            (pow(k2.clone(), num(3.0)), "a³（单项非零）"),
        ] {
            assert!(!is_structurally_zero(&bad), "{why} 被误判为恒零");
        }
        // Equal powers really are zero (a positive case, so that we do not switch the whole power
        // path off)
        assert!(
            is_structurally_zero(&sub(pow(k2.clone(), num(2.0)), mul(k2.clone(), k2.clone()))),
            "a² − a·a"
        );
        assert!(
            is_structurally_zero(&sub(
                pow(mul(num(2.0), k2.clone()), num(2.0)),
                mul(num(4.0), pow(k2.clone(), num(2.0)))
            )),
            "(2a)² − 4a²"
        );
        // Non-zero must not be misjudged as zero
        assert!(!is_structurally_zero(&sub(k2.clone(), var("k3"))));
        assert!(!is_structurally_zero(&sub(x.clone(), num(1.0))));
        // Identical subexpressions can be proven zero (coefficients sharing a key add up)
        assert!(is_structurally_zero(&sub(
            Expr::Call("exp".into(), vec![x.clone()]),
            Expr::Call("exp".into(), vec![x.clone()])
        )));
        // Identical subexpressions (including a symbolic divisor) are of course zero: the whole term
        // is one atom, and keys add up
        let kv = var("k");
        assert!(is_structurally_zero(&sub(
            div(x.clone(), kv.clone()),
            div(x.clone(), kv.clone())
        )));
        // But an identity that needs **algebraic expansion** to prove cannot be proven → return
        // false conservatively (no guessing)
        let (a, b) = (var("a"), var("b"));
        let expandable = sub(
            pow(add(a.clone(), b.clone()), num(2.0)),
            add(
                add(
                    pow(a.clone(), num(2.0)),
                    mul(num(2.0), mul(a.clone(), b.clone())),
                ),
                pow(b.clone(), num(2.0)),
            ),
        );
        assert!(
            !is_structurally_zero(&expandable),
            "(a+b)² − (a²+2ab+b²) 需要展开，证不出就该返回 false"
        );
    }

    #[test]
    fn solve_system_allows_equations_with_different_dimensions() {
        // Review S1-1: going through the generic `Mat` check, `dim_check` would require **all
        // equations to share a dimension**,
        // wrongly rejecting a completely legal system such as "force balance (N) + geometric
        // constraint (m)".
        let dims = dims_of(&[
            Decl {
                name: "k".into(),
                quantity: "stiffness".into(),
                unit: Some("N/m".into()),
            },
            Decl {
                name: "F".into(),
                quantity: "load".into(),
                unit: Some("N".into()),
            },
            Decl {
                name: "x".into(),
                quantity: "d1".into(),
                unit: Some("m".into()),
            },
            Decl {
                name: "L".into(),
                quantity: "len".into(),
                unit: Some("m".into()),
            },
            Decl {
                name: "y".into(),
                quantity: "d2".into(),
                unit: Some("m".into()),
            },
        ])
        .unwrap();
        // k·(x − y) − F = 0 (N) and x − L = 0 (m): two legal equations of different dimensions
        let sys = Expr::Call(
            "solve".into(),
            vec![
                Expr::Mat(vec![
                    vec![sub(mul(var("k"), sub(var("x"), var("y"))), var("F"))],
                    vec![sub(var("x"), var("L"))],
                ]),
                Expr::Mat(vec![vec![var("x")], vec![var("y")]]),
            ],
        );
        let d = dim_check(&sys, &dims).expect("量纲不同的方程应被接受");
        assert_eq!(d, parse_unit("m").unwrap(), "解的每个分量都是位移");
        // But a **single equation** that is not homogeneous must still be rejected
        let bad = Expr::Call(
            "solve".into(),
            vec![
                Expr::Mat(vec![
                    vec![sub(var("x"), var("F"))], // m vs N
                ]),
                Expr::Mat(vec![vec![var("x")]]),
            ],
        );
        let err = dim_check(&bad, &dims).unwrap_err();
        assert_eq!(err.code(), "dimension-mismatch", "{err}");
    }

    #[test]
    fn matrix_and_scalar_cannot_be_added() {
        // Review counterexample: `3 + roots(...)` / `3 + poly_coeffs(...)` were once emitted as
        // `3.0 + vec![vec![…]]` (E0277), while `--diagnose` reported it clean.
        let mat = Expr::Mat(vec![vec![num(1.0), num(2.0)], vec![num(3.0), num(4.0)]]);
        for bad in [
            add(num(3.0), mat.clone()),
            sub(mat.clone(), num(3.0)),
            add(mat.clone(), var("x")),
        ] {
            let err = shape_check(&bad).unwrap_err();
            // Stable code, not a sentence: the wording is per-locale.
            assert_eq!(err.code(), "matrix-scalar-mix", "{err}");
        }
        // Zero-element exception: `M + 0` is the additive identity and must not be judged wrong
        assert!(shape_check(&add(mat.clone(), num(0.0))).is_ok());
        assert!(shape_check(&sub(mat.clone(), num(0.0))).is_ok());
        // Matrix + matrix (same shape) is still legal
        assert!(shape_check(&add(mat.clone(), mat.clone())).is_ok());
        // Inconsistent shapes must still be reported
        let other = Expr::Mat(vec![vec![num(1.0), num(2.0), num(3.0)]]);
        let err = shape_check(&add(mat, other)).unwrap_err();
        assert_eq!(err.code(), "shape-mismatch", "{err}");
        // A matrix product taking part in an addition: `is_matrix_value` recognizes it, so it must
        // not be misjudged as a scalar
        let prod = mul(
            Expr::Mat(vec![vec![num(1.0), num(0.0)], vec![num(0.0), num(1.0)]]),
            Expr::Mat(vec![vec![num(1.0), num(2.0)], vec![num(3.0), num(4.0)]]),
        );
        assert!(
            shape_check(&add(prod.clone(), num(0.0))).is_ok(),
            "矩阵乘积 + 0 应合法"
        );
        let err = shape_check(&add(prod, num(1.0))).unwrap_err();
        assert_eq!(err.code(), "matrix-scalar-mix", "{err}");
    }

    #[test]
    fn matrix_shapes_are_checked_even_when_only_flatten_knows_them() {
        // Review counterexample: the shapes of `roots(...) + roots(...)` and
        // `poly_coeffs(...) * roots(...)`
        // are known only to `flatten_matrix_value` (`shape_of` returns None for Call),
        // so the whole check was skipped → emitting `vec![…] + vec![…]` / `vec![…] * vec![…]` (which
        // does not compile),
        // while `--diagnose` reported it clean.
        let x = var("x");
        // 1×1 + 2×1: inconsistent shapes, must be rejected
        let r1 = Expr::Call("roots".into(), vec![sub(x.clone(), num(1.0)), x.clone()]);
        let r2 = Expr::Call(
            "roots".into(),
            vec![sub(pow(x.clone(), num(2.0)), num(1.0)), x.clone()],
        );
        let err = shape_check(&add(r1.clone(), r2.clone())).unwrap_err();
        assert_eq!(err.code(), "shape-mismatch", "{err}");
        // 1×(n+1) coefficients × n×1 roots: the same polynomial never matches, so it must be
        // rejected (no vec*vec emission)
        let coeffs = Expr::Call(
            "poly_coeffs".into(),
            vec![add(pow(x.clone(), num(2.0)), x.clone()), x.clone()],
        );
        // Matrix×matrix with mismatched inner dims: the sentence names both factors, so assert
        // the payload rather than merely "something failed".
        let err = shape_check(&mul(coeffs.clone(), r2.clone())).unwrap_err();
        match &err {
            Error::ShapeMismatch(why) => assert!(
                why.contains("cannot multiply") && why.contains("1, 3"),
                "the sentence must name both factors: {why}"
            ),
            other => panic!("{other:?}"),
        }
        // Legal shapes still pass: 2×2 times 2×1
        let m = Expr::Mat(vec![vec![num(1.0), num(2.0)], vec![num(3.0), num(4.0)]]);
        let col = Expr::Mat(vec![vec![num(1.0)], vec![num(2.0)]]);
        assert!(shape_check(&mul(m.clone(), col.clone())).is_ok());
        // 2×2 + 2×2 passes
        assert!(shape_check(&add(m.clone(), m.clone())).is_ok());
        // Adding 2×2 and 2×1 must be rejected
        let err = shape_check(&add(m, col)).unwrap_err();
        assert_eq!(err.code(), "shape-mismatch", "{err}");
    }

    #[test]
    fn solve_system_rejects_matrix_coefficients() {
        // Review S1-2: the system lane used to let matrix coefficients through, emitting
        // `f64 / Vec<Vec<f64>>`
        let mat = Expr::Mat(vec![vec![num(1.0), num(2.0)], vec![num(3.0), num(4.0)]]);
        let x = var("x");
        let err =
            solve_system(&[sub(mul(mat.clone(), x.clone()), var("b"))], &["x".into()]).unwrap_err();
        assert_eq!(err.code(), "matrix-coefficient", "{err}");
        // A matrix-valued constant term is rejected likewise
        let err = solve_system(&[sub(x.clone(), mat)], &["x".into()]).unwrap_err();
        assert_eq!(err.code(), "matrix-coefficient", "{err}");
    }

    #[test]
    fn expand_deep_reaches_nested_operators() {
        // Review S1-3: `expand_call` only looks at the root node, so in `2 * solve(...)` the
        // solved-for unknown
        // stays in the parameter list (generating a function "asking the caller to pass the
        // solved-for quantity").
        let x = var("x");
        let a = var("a");
        let c = var("c");
        let nested = mul(
            num(2.0),
            Expr::Call(
                "solve".into(),
                vec![sub(mul(a.clone(), x.clone()), c.clone()), x.clone()],
            ),
        );
        // The root is a Mul, so top-level expand_call leaves it alone — exactly the gap the review
        // pointed out
        assert!(expand_call(&nested).is_none(), "顶层展开不该命中嵌套算子");
        let deep = expand_deep(&nested);
        assert!(
            !vars_of(&deep).contains(&"x".to_string()),
            "深度展开后不该再有被求解的 x：{deep:?}"
        );
        // det nested inside an addition can be expanded too
        let det_nested = add(
            Expr::Call(
                "det".into(),
                vec![Expr::Mat(vec![
                    vec![num(1.0), num(2.0)],
                    vec![num(3.0), num(4.0)],
                ])],
            ),
            num(1.0),
        );
        let d = simplify(&expand_deep(&det_nested));
        assert!(matches!(d, Expr::Num(_)), "det(A)+1 应折成常数，得到 {d:?}");
        assert!((eval(&d, &HashMap::new()).unwrap() - (-1.0)).abs() < 1e-12);
    }

    #[test]
    fn solve_system_routes_through_expand_and_flatten() {
        let (x, y) = (var("x"), var("y"));
        let sys = Expr::Call(
            "solve".into(),
            vec![
                Expr::Mat(vec![
                    vec![sub(x.clone(), num(4.0))],
                    vec![sub(y.clone(), num(1.0))],
                ]),
                Expr::Mat(vec![vec![x.clone(), y.clone()]]),
            ],
        );
        // expand_call: the system form → a column vector; the single-variable form → a scalar
        let expanded = expand_call(&sys).expect("方程组应可展开");
        let Expr::Mat(rows) = &expanded else {
            panic!("展开结果应是矩阵，得到 {expanded:?}");
        };
        assert_eq!(rows.len(), 2);
        assert!((eval(&rows[0][0], &HashMap::new()).unwrap() - 4.0).abs() < 1e-12);
        // Matrix-value determination: the system is a true matrix; the single-variable solve is
        // not
        assert!(is_matrix_value(&sys), "方程组 solve 应是矩阵值");
        let single = Expr::Call(
            "solve".into(),
            vec![sub(mul(var("a"), var("x")), num(6.0)), var("x")],
        );
        assert!(!is_matrix_value(&single), "单变量 solve 是标量值");
        // A single-variable solve with matrix coefficients must be rejected (the scalar lane would
        // generate a vec! division)
        let mat_coeff = Expr::Call(
            "solve".into(),
            vec![
                sub(
                    mul(
                        Expr::Mat(vec![vec![num(1.0), num(0.0)], vec![num(0.0), num(1.0)]]),
                        x.clone(),
                    ),
                    y.clone(),
                ),
                x,
            ],
        );
        let Expr::Call(_, mat_args) = &mat_coeff else {
            panic!("构造的应是 Call");
        };
        let err = solve_linear(&mat_args[0], "x").unwrap_err();
        assert_eq!(err.code(), "matrix-coefficient", "{err}");
    }

    #[test]
    fn typo_suggestions_and_multiarg_mathml() {
        // A mistyped function name → suggest the closest name (edit distance ≤ 2)
        assert_eq!(closest_func("sqrtt"), Some("sqrt"));
        // Short names are naturally confusable (`mn` is close to both ln and min): the assertions
        // here use **long names**,
        // and preserve the semantics "do not guess when it really is ambiguous"
        assert_eq!(closest_func("clampx"), Some("clamp"));
        assert_eq!(closest_func("adjiont"), Some("adjoint"));
        assert_eq!(closest_func("dett"), Some("det"));
        // Completely unrelated names get no suggestion
        assert_eq!(closest_func("foobar"), None);
        let e = Expr::Call("sqrtt".into(), vec![var("x")]);
        let err = check_calls(&e).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("sqrt"), "{text}");

        // Multi-argument MathML must be comma-separated (otherwise the preview panel renders
        // `min(a b)`).
        let m = Expr::Call("min".into(), vec![var("a"), var("b")]);
        let xml = to_mathml(&m);
        assert!(xml.contains("<mo>,</mo>"), "{xml}");
        assert!(xml.contains("<mi>min</mi>"), "{xml}");
    }

    #[test]
    fn horner_matches_power_form_and_only_touches_explicit_polynomials() {
        // Explicit polynomial: a·x³ + b·x → Horner
        // (the source is `a*x^3 + b*x`; every base is x itself)
        let e = add(
            mul(var("a"), pow(var("x"), num(3.0))),
            mul(var("b"), var("x")),
        );
        let h = horner_in(&e, "x").expect("显式多项式应可 Horner");
        assert!(!matches!(h, Expr::Num(_)));
        // Pointwise equal values (Horner only changes the association order, not the math)
        let case = [
            (0.0f64, 2.0f64, 3.0f64),
            (-1.5, 0.3, -2.0),
            (7.25, 1.0, 0.5),
        ];
        for (x, a, b) in case {
            let mut env = HashMap::new();
            env.insert("x".to_string(), x);
            env.insert("a".to_string(), a);
            env.insert("b".to_string(), b);
            let want = eval(&e, &env).unwrap();
            let got = eval(&h, &env).unwrap();
            assert!(
                (got - want).abs() <= 1e-12 * (1.0 + want.abs()),
                "x={x} Horner {got} vs 幂次 {want}"
            );
        }
        // Fewer multiplications: no third power should remain in the Horner form
        assert!(!contains_pow_of(&h, "x", 3.0), "Horner 后不该有 x^3：{h:?}");

        // The compact form `(u-1)^3` is **not** expanded (expansion is harder to read and no faster)
        let compact = pow(sub(var("u"), num(1.0)), num(3.0));
        assert!(horner_in(&compact, "u").is_none());

        // Below degree 1 it is pointless
        let lin = add(mul(var("a"), var("x")), var("b"));
        assert!(horner_in(&lin, "x").is_none());
    }

    #[test]
    fn libm_lane_uses_libm_calls_and_rejects_alloc() {
        // x^0.5 → libm::sqrt (not f64::sqrt — it is not in core)
        let e = pow(var("x"), num(0.5));
        let w = emit_block_libm(&e);
        assert!(w.contains("libm::sqrt"), "{w}");
        assert!(!w.contains(".sqrt()"), "{w}");
        // x^2.5 → libm::pow (there is no powf)
        let e = pow(var("x"), num(2.5));
        let w = emit_block_libm(&e);
        assert!(w.contains("libm::pow"), "{w}");
        // Integer powers also go through libm::pow (powi is from std)
        let e = pow(var("x"), num(3.0));
        let w = emit_block_libm(&e);
        assert!(w.contains("libm::pow"), "{w}");
        // Transcendental functions → libm::exp
        let e = Expr::Call("exp".into(), vec![var("x")]);
        let w = emit_block_libm(&e);
        assert!(w.contains("libm::exp"), "{w}");
        // abs → libm::fabs
        let e = Expr::Call("abs".into(), vec![var("x")]);
        let w = emit_block_libm(&e);
        assert!(w.contains("libm::fabs"), "{w}");
        // Matrix-valued formulas are rejected as a whole block (vec! needs alloc)
        let m = Expr::Mat(vec![vec![num(1.0), num(2.0)]]);
        let w = emit_block_libm(&m);
        assert!(w.contains("no_std"), "{w}");
        // clamp **works** in no_std: min/max are core methods (not std-only)
        let e = Expr::Call("clamp".into(), vec![var("x"), num(0.0), num(1.0)]);
        let w = emit_block_libm(&e);
        assert!(w.contains(".max(0.0).min(1.0)"), "{w}");
        // Genuinely outside the libm whitelist: arg (complex-only, no counterpart in core)
        let e = Expr::Call("arg".into(), vec![var("x")]);
        let w = emit_block_libm(&e);
        assert!(w.contains("compile_error!"), "{w}");
    }

    #[test]
    fn negative_base_fractional_power_is_not_folded_to_nan() {
        // Regression: fold_const used to fold (-8)^(1/3) to NaN while the value layer gives a
        // complex number —
        // two lanes, two answers. Now it is not folded, leaving it to the evaluation layer.
        let e = pow(num(-8.0), div(num(1.0), num(3.0)));
        let folded = simplify(&e);
        assert!(
            !matches!(folded, Expr::Num(v) if v.is_nan()),
            "不该折叠成 NaN：{folded:?}"
        );
        // Evaluation takes the complex principal branch: (-8)^(1/3) = 1 + 1.732j
        let v = eval_value(&folded, &HashMap::new())
            .unwrap()
            .as_scalar()
            .unwrap();
        assert!((v.re - 1.0).abs() < 1e-12, "{v:?}");
        assert!((v.im - 3.0f64.sqrt()).abs() < 1e-12, "{v:?}");
        // Positive bases / integer exponents are folded as before
        assert!(matches!(simplify(&pow(num(4.0), num(0.5))), Expr::Num(v) if v == 2.0));
        assert!(matches!(simplify(&pow(num(-2.0), num(3.0))), Expr::Num(v) if v == -8.0));
    }

    #[test]
    fn negative_literal_sqrt_is_rejected_by_real_emission_not_deferred_to_nan() {
        let e = Expr::Call("sqrt".into(), vec![num(-4.0)]);
        let real = emit_block(&e);
        assert!(real.contains("compile_error!"), "{real}");
        // The same mathematical object is still the principal value 2j in the complex lane; it must
        // not be caught by the real guard.
        let complex = emit_block_complex(&e);
        assert!(
            complex.contains("__c_sqrt") && !complex.contains("compile_error!"),
            "{complex}"
        );
        let value = eval_value(&e, &HashMap::new())
            .unwrap()
            .as_scalar()
            .unwrap();
        assert_eq!(value, C::new(0.0, 2.0));
    }

    #[test]
    fn matrix_power_folds_to_explicit_products_or_rejects() {
        // M² = M·M (the flatten layer does repeated matrix multiplication)
        let m = Expr::Mat(vec![vec![num(1.0), num(2.0)], vec![num(3.0), num(4.0)]]);
        let sq = Expr::Pow(Box::new(m.clone()), Box::new(num(2.0)));
        assert!(is_matrix_value(&sq), "整数幂的矩阵应可折叠");
        // Numeric check: [[1,2],[3,4]]² = [[7,10],[15,22]]
        // **Both** the value layer and the generator support it (the same folding semantics; himari-eval
        // is no longer behind)
        let want = [[7.0f64, 10.0], [15.0, 22.0]];
        match eval_value(&sq, &HashMap::new()).unwrap() {
            Value::Matrix(mm) => {
                for r in 0..2 {
                    for c in 0..2 {
                        assert_eq!(mm[r][c].re, want[r][c], "值层 [{r}][{c}]");
                    }
                }
            }
            other => panic!("{other:?}"),
        }
        let flat = flatten_matrix_value(&sq).unwrap();
        match eval_value(&Expr::Mat(flat), &HashMap::new()).unwrap() {
            Value::Matrix(mm) => {
                for r in 0..2 {
                    for c in 0..2 {
                        assert_eq!(mm[r][c].re, want[r][c], "折叠 [{r}][{c}]");
                    }
                }
            }
            other => panic!("{other:?}"),
        }
        // Fractional power / non-square → not folded (is_matrix_value = false)
        assert!(!is_matrix_value(&Expr::Pow(
            Box::new(m.clone()),
            Box::new(num(2.5))
        )));
        let row = Expr::Mat(vec![vec![num(1.0), num(2.0)]]);
        assert!(!is_matrix_value(&Expr::Pow(
            Box::new(row),
            Box::new(num(2.0))
        )));
        // For a rejected matrix power the emitter emits compile_error (not a vec!*vec! compile
        // accident)
        let code = emit_block(&Expr::Pow(Box::new(m.clone()), Box::new(num(2.5))));
        assert!(code.contains("compile_error!"), "{code}");
    }

    #[test]
    fn matrix_zero_power_is_identity_not_the_scalar_one() {
        let m = Expr::Mat(vec![vec![num(1.0), num(2.0)], vec![num(3.0), num(4.0)]]);
        let e = pow(m, num(0.0));
        let folded = simplify(&e);
        assert!(is_matrix_value(&folded), "M^0 必须保持矩阵类型：{folded:?}");
        let want = Value::Matrix(vec![vec![C::ONE, C::ZERO], vec![C::ZERO, C::ONE]]);
        assert_eq!(eval_value(&folded, &HashMap::new()).unwrap(), want);
        assert_eq!(
            eval_value(
                &Expr::Mat(flatten_matrix_value(&folded).unwrap()),
                &HashMap::new()
            )
            .unwrap(),
            want
        );
        let code = emit_block(&folded);
        assert!(
            code.contains("vec![vec![1.0, 0.0], vec![0.0, 1.0]]"),
            "{code}"
        );
    }

    #[test]
    fn dot_and_norm_are_expanded_from_literals() {
        // dot([1,2],[3,4]) = 11
        let a = Expr::Mat(vec![vec![num(1.0), num(2.0)]]);
        let b = Expr::Mat(vec![vec![num(3.0), num(4.0)]]);
        let d = Expr::Call("dot".into(), vec![a.clone(), b.clone()]);
        let v = eval_value(&d, &HashMap::new())
            .unwrap()
            .as_scalar()
            .map(|c| c.re);
        assert_eq!(v, Some(11.0));
        // norm([3,4]) = 5 (3-4-5)
        let n = Expr::Call("norm".into(), vec![a.clone()]);
        let v = eval_value(&n, &HashMap::new())
            .unwrap()
            .as_scalar()
            .map(|c| c.re);
        assert_eq!(v, Some(5.0f64.sqrt()));
        // Dimensions: dot is the square of the dimension, norm matches the elements' dimension
        let decls = vec![Decl {
            name: "v".into(),
            quantity: "velocity".into(),
            unit: Some("m/s".into()),
        }];
        let dims = dims_of(&decls).unwrap();
        let ve = Expr::Mat(vec![vec![var("v"), var("v")]]);
        let dot = Expr::Call("dot".into(), vec![ve.clone(), ve.clone()]);
        assert_eq!(
            dim_check(&dot, &dims).unwrap(),
            parse_unit("m^2/s^2").unwrap(),
            "dot 的量纲是平方"
        );
        let norm = Expr::Call("norm".into(), vec![ve]);
        assert_eq!(
            dim_check(&norm, &dims).unwrap(),
            parse_unit("m/s").unwrap(),
            "norm 与元素同量纲"
        );
        // Inconsistent shapes → expansion fails (no silent computation)
        let c = Expr::Mat(vec![vec![num(1.0), num(2.0), num(3.0)]]);
        assert!(expand_dot(&Expr::Call("dot".into(), vec![a, c])).is_none());
        // The generated code contains an ordinary scalar expression
        let code = emit_block(&Expr::Call(
            "dot".into(),
            vec![
                Expr::Mat(vec![vec![num(1.0), num(2.0)]]),
                Expr::Mat(vec![vec![var("x"), var("y")]]),
            ],
        ));
        assert!(code.contains("x") && code.contains("y"), "{code}");
    }

    #[test]
    fn determinant_and_inverse_reach_the_documented_orders() {
        // Both the order-2 and order-3 inverse can be expanded (2D/3D isoparametric transforms)
        let m2 = Expr::Mat(vec![vec![var("a"), var("b")], vec![var("c"), var("d")]]);
        let inv2 = Expr::Call("inv".into(), vec![m2.clone()]);
        assert_eq!(flatten_matrix_value(&inv2).expect("2 阶逆").len(), 2);
        let m3 = Expr::Mat(vec![
            vec![var("a"), var("b"), var("c")],
            vec![var("d"), var("e"), var("f")],
            vec![var("g"), var("h"), var("i")],
        ]);
        let inv3 = Expr::Call("inv".into(), vec![m3.clone()]);
        let flat3 = flatten_matrix_value(&inv3).expect("3 阶逆");
        assert_eq!(flat3.len(), 3);
        // The order-4 inverse is explicitly not done (16 3×3 minors ≈ 100 terms; the generated code
        // explodes)
        let m4 = Expr::Mat(vec![
            vec![num(1.0), num(2.0), num(3.0), num(4.0)],
            vec![num(5.0), num(6.0), num(7.0), num(8.0)],
            vec![num(9.0), num(10.0), num(11.0), num(12.0)],
            vec![num(13.0), num(14.0), num(15.0), num(16.0)],
        ]);
        assert!(flatten_matrix_value(&Expr::Call("inv".into(), vec![m4.clone()])).is_none());

        // det up to order 4: an upper-triangular 4×4 → det = 1·2·3·4 = 24
        let tri = Expr::Mat(vec![
            vec![num(1.0), num(5.0), num(6.0), num(7.0)],
            vec![num(0.0), num(2.0), num(8.0), num(9.0)],
            vec![num(0.0), num(0.0), num(3.0), num(10.0)],
            vec![num(0.0), num(0.0), num(0.0), num(4.0)],
        ]);
        let d4 = eval_value(&Expr::Call("det".into(), vec![tri]), &HashMap::new())
            .unwrap()
            .as_scalar()
            .map(|c| c.re);
        assert_eq!(d4, Some(24.0));
        // Order 5 is rejected
        let five: Vec<Vec<Expr>> = (0..5)
            .map(|i| (0..5).map(|j| num((i * 5 + j) as f64)).collect())
            .collect();
        let err = eval_value(
            &Expr::Call("det".into(), vec![Expr::Mat(five)]),
            &HashMap::new(),
        )
        .unwrap_err();
        assert_eq!(err.code(), "invalid-input", "{err}");
        assert!(err.to_string().contains("1–4"), "{err}");

        // Numeric check of the order-3 inverse: M·M⁻¹ = I
        let mut env = HashMap::new();
        for (k, v) in [
            ("a", 2.0),
            ("b", -1.0),
            ("c", 0.0),
            ("d", -1.0),
            ("e", 2.0),
            ("f", -1.0),
            ("g", 0.0),
            ("h", -1.0),
            ("i", 2.0),
        ] {
            env.insert(k.to_string(), v);
        }
        let flat = flatten_matrix_value(&inv3).unwrap();
        let m_val = match eval_value(&m3, &env).unwrap() {
            Value::Matrix(m) => m,
            other => panic!("{other:?}"),
        };
        let inv_val = match eval_value(&Expr::Mat(flat), &env).unwrap() {
            Value::Matrix(m) => m,
            other => panic!("{other:?}"),
        };
        // All three are **matrix index semantics** (i/j/k traverse rows/columns/the inner dimension
        // respectively), not set iteration
        #[allow(clippy::needless_range_loop)]
        for r in 0..3 {
            #[allow(clippy::needless_range_loop)]
            for c in 0..3 {
                let mut acc = C::ZERO;
                for k in 0..3 {
                    acc = acc.add(m_val[r][k].mul(inv_val[k][c]));
                }
                let want = if r == c { 1.0 } else { 0.0 };
                assert!((acc.re - want).abs() < 1e-12, "M·M⁻¹[{r}][{c}] = {acc:?}");
            }
        }
        // Evaluating a singular matrix errors (not inf)
        let mut sing = HashMap::new();
        for (k, v) in [("a", 1.0), ("b", 1.0), ("c", 1.0), ("d", 1.0)] {
            sing.insert(k.to_string(), v);
        }
        // After IR expansion, a singular matrix reports division by zero at the "divide by det" step
        // (still an explicit error, not inf)
        let err = eval_value(&inv2, &sing).unwrap_err();
        assert_eq!(err.code(), "division-by-zero", "{err}");
        assert!(matches!(err, Error::DivisionByZero), "{err}");
    }

    #[test]
    fn buckingham_pi_recovers_reynolds() {
        let vars = vec![
            ("rho".to_string(), parse_unit("kg/m^3").unwrap()),
            ("v".to_string(), parse_unit("m/s").unwrap()),
            ("L".to_string(), parse_unit("m").unwrap()),
            ("mu".to_string(), parse_unit("Pa*s").unwrap()),
        ];
        let g = dimensionless_groups(&vars).unwrap();
        assert_eq!(g.len(), 1, "4 变量 / 秩 3 → 1 个无量纲群");
        let names: Vec<String> = vars.iter().map(|(n, _)| n.clone()).collect();
        assert_eq!(pi_text(&names, &g[0]), "rho·v·L·mu^-1");
        // Verify term by term: the dimension after weighting by the exponents must be 1
        let mut acc = Dim::ZERO;
        for ((_, d), e) in vars.iter().zip(&g[0]) {
            acc = acc.mul_dim(d.scale_ratio((e * 2.0) as i32, 2).unwrap());
        }
        assert!(acc.is_zero(), "Π 必须无量纲：{}", acc.display());
    }

    #[test]
    fn buckingham_pi_group_count_matches_theorem() {
        // Five base quantities → rank 5 → 0 groups
        let mut base: Vec<(String, Dim)> = ["kg", "m", "s", "A", "K"]
            .iter()
            .map(|u| (u.to_string(), parse_unit(u).unwrap()))
            .collect();
        assert_eq!(dimensionless_groups(&base).unwrap().len(), 0);
        // Adding a derived quantity (force = kg·m/s²) does not increase the rank: 6 variables /
        // rank 5 → 1 group
        // (this is exactly what the Π theorem says: the number of groups = number of variables −
        // rank)
        base.push(("F".to_string(), parse_unit("N").unwrap()));
        let g1 = dimensionless_groups(&base).unwrap();
        assert_eq!(g1.len(), 1, "6 变量 / 秩 5 → 1 个群");
        assert_eq!(
            pi_text(
                &base.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>(),
                &g1[0]
            ),
            "kg·m·s^-2·F^-1",
            "F/(kg·m/s²) 的那个群"
        );
        // Adding energy too (J = kg·m²/s²): the rank is unchanged → 7 variables / rank 5 → 2
        // groups
        let mut v = base.clone();
        v.push(("E".to_string(), parse_unit("J").unwrap()));
        assert_eq!(
            dimensionless_groups(&v).unwrap().len(),
            2,
            "7 变量 / 秩 5 → 2 个群"
        );
    }

    #[test]
    fn dim_solve_finds_target_dimension() {
        let vars = vec![
            ("mass".to_string(), parse_unit("kg").unwrap()),
            ("acc".to_string(), parse_unit("m/s^2").unwrap()),
        ];
        // F = m·a: exponents of 1 each
        assert_eq!(
            dim_solve(&vars, parse_unit("N").unwrap()).unwrap(),
            vec![1.0, 1.0]
        );
        // A dimension that cannot be formed must error, and say so clearly
        let volt = parse_unit("V").unwrap();
        let err = dim_solve(&vars, volt).unwrap_err();
        // Catalogued: the stable code, and the payload carries the requested dimension
        // (in its base-dimension display form).
        assert_eq!(err.code(), "target-dimension-unreachable", "{err}");
        match &err {
            Error::TargetDimensionUnreachable { target, .. } => {
                assert_eq!(*target, volt.display().to_string());
            }
            other => panic!("{other:?}"),
        }
        assert!(
            err.to_string().contains(&volt.display().to_string()),
            "{err}"
        );
        // A dimensionless target gives the all-zero solution
        let zero = dim_solve(&vars, Dim::ZERO).unwrap();
        assert_eq!(zero, vec![0.0, 0.0]);
    }

    #[test]
    fn matrix_expression_codegen_matches_value_layer() {
        // Differential check: evaluating the **folded literal** must equal evaluating the **original
        // expression**.
        // The emitter's folding (including transpose/add/subtract/scalar broadcast/matrix multiply)
        // and the value layer are two independent implementations,
        // and it only counts as really correct if they agree.
        let a = Expr::Mat(vec![vec![num(1.0), num(2.0)], vec![num(3.0), num(4.0)]]);
        let b = Expr::Mat(vec![vec![num(5.0), num(6.0)], vec![num(7.0), num(8.0)]]);
        let row = Expr::Mat(vec![vec![num(1.0), num(2.0), num(3.0)]]);
        let cases = vec![
            transpose(row.clone()),
            add(a.clone(), b.clone()),
            sub(a.clone(), b.clone()),
            mul(a.clone(), b.clone()),
            mul(var("k"), a.clone()),
            neg(a.clone()),
            transpose(mul(a.clone(), b.clone())),
        ];
        let mut env = HashMap::new();
        env.insert("k".to_string(), 2.5);
        for e in &cases {
            let folded = flatten_matrix_value(e).unwrap_or_else(|| panic!("应可折叠：{e:?}"));
            let v1 = eval_value(e, &env).unwrap();
            let v2 = eval_value(&Expr::Mat(folded), &env).unwrap();
            assert_eq!(v1, v2, "折叠与值层不同值：{e:?}");
            assert!(is_matrix_value(e));
            assert!(
                emit_block(e).contains("vec!["),
                "矩阵值必须发射成 vec![...]"
            );
        }
        // The transposed 1×3 becomes a 3×1 literal
        assert_eq!(
            emit_block(&transpose(row)).trim(),
            "{\n        vec![vec![1.0], vec![2.0], vec![3.0]]\n    }"
        );
        // Cases that are not folded: inconsistent shapes / symbolic matrices of unknown shape /
        // scalar expressions
        let jag = Expr::Mat(vec![vec![num(1.0)], vec![num(2.0)]]);
        let small = Expr::Mat(vec![vec![num(1.0)]]);
        assert!(!is_matrix_value(&add(small.clone(), jag)));
        assert!(!is_matrix_value(&transpose(var("A"))));
        assert!(!is_matrix_value(&add(num(1.0), var("k"))));
        // A bare symbol is a **scalar** (a matrix can only come from a literal or the pattern
        // generator):
        // so `A * [1, 2]` is scalar broadcasting, not a matrix product
        assert!(is_matrix_value(&mul(var("A"), small)));
    }

    #[test]
    fn reserved_j_is_not_a_parameter() {
        // `j` is the reserved imaginary unit: it does not enter the parameter list (otherwise the
        // signature would become "please pass the imaginary unit")
        let decls = vec![Decl {
            name: "r".into(),
            quantity: "resistance".into(),
            unit: Some("V/A".into()),
        }];
        let e = add(var("r"), mul(var("j"), num(2.0)));
        assert_eq!(params_in_decl_order(&decls, &e), vec!["r".to_string()]);
        assert!(uses_complex(&e));
        let code = emit_block_complex(&e);
        assert!(
            code.contains("__c_add") && code.contains("(0.0, 1.0)"),
            "{code}"
        );
        assert!(!code.contains("__c_sqrt"), "未用到的算子不该被发射：{code}");
        // A purely real expression does not take the complex lane
        assert!(!uses_complex(&add(var("r"), num(2.0))));
    }

    #[test]
    fn diff_matches_finite_difference() {
        // f = exp(2x) * sin(x) / (1 + x^2)
        let f = div(
            mul(
                Expr::Call("exp".into(), vec![mul(num(2.0), var("x"))]),
                Expr::Call("sin".into(), vec![var("x")]),
            ),
            add(num(1.0), pow(var("x"), num(2.0))),
        );
        let df = simplify(&diff(&f, "x"));
        let mut env = HashMap::new();
        for x in [-1.3, -0.2, 0.4, 1.7] {
            env.insert("x".to_string(), x);
            let h = 1e-6;
            env.insert("x".to_string(), x + h);
            let fp = eval(&f, &env).unwrap();
            env.insert("x".to_string(), x - h);
            let fm = eval(&f, &env).unwrap();
            env.insert("x".to_string(), x);
            let analytic = eval(&df, &env).unwrap();
            let numeric = (fp - fm) / (2.0 * h);
            assert!(
                (analytic - numeric).abs() < 1e-5 * (1.0 + numeric.abs()),
                "x={x} analytic={analytic} numeric={numeric}"
            );
        }
    }

    #[test]
    fn dim_check_catches_mismatch() {
        let decls = vec![
            Decl {
                name: "a".into(),
                quantity: "length".into(),
                unit: Some("m".into()),
            },
            Decl {
                name: "t".into(),
                quantity: "time".into(),
                unit: Some("s".into()),
            },
        ];
        let dims = dims_of(&decls).unwrap();
        assert!(dim_check(&div(var("a"), var("t")), &dims).is_ok());
        assert!(dim_check(&add(var("a"), var("t")), &dims).is_err());
        assert!(dim_check(&var("nope"), &dims).is_err());
    }

    // ---------------- matrix patterns ----------------

    fn pat(rows: Vec<Vec<PatternCell>>) -> MatPattern {
        MatPattern { rows }
    }
    fn el(e: Expr) -> PatternCell {
        PatternCell::El(e)
    }
    const D: PatternCell = PatternCell::Dots;

    #[test]
    fn recognize_identity() {
        // [[1, ·, 0], [·, 1, ·], [0, ·, 1]]
        let p = pat(vec![
            vec![el(num(1.0)), D, el(num(0.0))],
            vec![D, el(num(1.0)), D],
            vec![el(num(0.0)), D, el(num(1.0))],
        ]);
        assert_eq!(recognize_pattern(&p).unwrap(), MatrixKind::Identity);
    }

    #[test]
    fn recognize_diag() {
        let p = pat(vec![
            vec![el(var("d")), D, el(num(0.0))],
            vec![D, el(var("d")), D],
            vec![el(num(0.0)), D, el(var("d"))],
        ]);
        assert!(matches!(
            recognize_pattern(&p).unwrap(),
            MatrixKind::Diag(_)
        ));
        // The diagonal is not the same expression → reject
        let bad = pat(vec![
            vec![el(var("d")), D, el(num(0.0))],
            vec![D, el(var("e")), D],
            vec![el(num(0.0)), D, el(var("d"))],
        ]);
        assert!(recognize_pattern(&bad).is_err());
    }

    #[test]
    fn recognize_tridiag() {
        let p = pat(vec![
            vec![el(var("b")), el(var("c")), D, el(num(0.0))],
            vec![el(var("a")), el(var("b")), el(var("c")), D],
            vec![D, el(var("a")), el(var("b")), el(var("c"))],
            vec![el(num(0.0)), D, el(var("a")), el(var("b"))],
        ]);
        match recognize_pattern(&p).unwrap() {
            MatrixKind::Tridiag { .. } => {}
            other => panic!("期望 Tridiag，得到 {other:?}"),
        }
        // Different diagonals → explicit error (not a silent downgrade)
        let mut bad = p;
        if let PatternCell::El(e) = &mut bad.rows[1][1] {
            *e = var("z");
        }
        assert!(matches!(
            recognize_pattern(&bad).unwrap_err(),
            Error::InvalidInput(ref why) if why.contains("tridiagonal")
        ));
    }

    #[test]
    fn half_integer_dimensions() {
        // √(m²) = m: integer powers are still correct
        let decls = vec![Decl {
            name: "a".into(),
            quantity: "area".into(),
            unit: Some("m^2".into()),
        }];
        let dims = dims_of(&decls).unwrap();
        let root = Expr::Call("sqrt".into(), vec![var("a")]);
        assert_eq!(dim_check(&root, &dims).unwrap(), parse_unit("m").unwrap());

        // √(s) = s^1/2: half-integer exponents (previously rejected)
        let decls2 = vec![Decl {
            name: "t".into(),
            quantity: "time".into(),
            unit: Some("s".into()),
        }];
        let dims2 = dims_of(&decls2).unwrap();
        let root2 = Expr::Call("sqrt".into(), vec![var("t")]);
        let half = dim_check(&root2, &dims2).unwrap();
        assert_eq!(half.display(), "s^(1/2)");
        // Ω·s^-1/2: σ is V/A, multiplied by s^-1/2
        let sigma = var("sigma");
        let d_sigma = dim_check(
            &sigma,
            &dims_of(&[Decl {
                name: "sigma".into(),
                quantity: "warburg_coefficient".into(),
                unit: Some("V/A".into()),
            }])
            .unwrap(),
        )
        .unwrap();
        // Then multiply by s^-1/2 (expressed as 1/√s)
        let inv_sqrt_s = div(num(1.0), root2.clone());
        let d_inv = dim_check(&inv_sqrt_s, &dims2).unwrap();
        let combined = d_sigma.mul_dim(d_inv);
        // V/A = kg·m²·s⁻³·A⁻², then multiplied by s^(-1/2)
        assert_eq!(
            combined.display(),
            "kg·m^2·s^(-7/2)·A^-2",
            "Ω·s^(-1/2) 量纲"
        );
    }

    #[test]
    fn complex_impedance_arithmetic() {
        // Z = R + j*X (j is the reserved imaginary unit, dimensionless)
        let z = add(var("R"), mul(var("j"), var("X")));
        let mut env = HashMap::new();
        env.insert("R".to_string(), 3.0);
        env.insert("X".to_string(), -4.0);
        match eval_value(&z, &env).unwrap() {
            Value::Scalar(c) => {
                assert_eq!((c.re, c.im), (3.0, -4.0));
            }
            other => panic!("期望标量，得到 {other:?}"),
        }
        // |Z| = √(R²+X²) = 5
        let mag = Expr::Call(
            "sqrt".into(),
            vec![add(pow(var("R"), num(2.0)), pow(var("X"), num(2.0)))],
        );
        assert_eq!(eval(&mag, &env).unwrap(), 5.0);

        // Dimensions: j is dimensionless, so both sides of Z = R + jX share a dimension (Ω)
        let decls = vec![
            Decl {
                name: "R".into(),
                quantity: "resistance".into(),
                unit: Some("V/A".into()),
            },
            Decl {
                name: "X".into(),
                quantity: "reactance".into(),
                unit: Some("V/A".into()),
            },
        ];
        let dims = dims_of(&decls).unwrap();
        assert_eq!(
            dim_check(&z, &dims).unwrap(),
            parse_unit("V/A").unwrap(),
            "复阻抗量纲应仍是 Ω"
        );

        // Complex division: Z1/Z2, with both real and imaginary parts correct
        let q = div(
            add(num(1.0), mul(var("j"), num(1.0))),
            add(num(1.0), mul(var("j"), num(-1.0))),
        );
        match eval_value(&q, &HashMap::new()).unwrap() {
            Value::Scalar(c) => {
                assert!(c.re.abs() < 1e-15, "re={}", c.re);
                assert!((c.im - 1.0).abs() < 1e-15, "im={}", c.im);
            }
            other => panic!("期望标量，得到 {other:?}"),
        }
    }

    #[test]
    fn recognize_banded_k2() {
        // 5×5 k=2: every cell with |i-j|<=2 is v, everything outside the band is 0
        let mk = |blank_at: &[(usize, usize)]| {
            let mut rows = Vec::new();
            for i in 0..5usize {
                let mut r = Vec::new();
                for j in 0..5usize {
                    r.push(if i.abs_diff(j) <= 2 {
                        el(var("v"))
                    } else if blank_at.contains(&(i, j)) {
                        D
                    } else {
                        el(num(0.0))
                    });
                }
                rows.push(r);
            }
            pat(rows)
        };
        match recognize_pattern(&mk(&[])).unwrap() {
            MatrixKind::Banded { half_band, .. } => assert_eq!(half_band, 2),
            other => panic!("期望 Banded k=2，得到 {other:?}"),
        }
        // Ellipses standing in for 0 are recognized too
        match recognize_pattern(&mk(&[(0, 3), (0, 4), (3, 0), (4, 0)])).unwrap() {
            MatrixKind::Banded { half_band, .. } => assert_eq!(half_band, 2),
            other => panic!("省略号形态应为 Banded，得到 {other:?}"),
        }
        // A concrete non-zero placed outside the band → not banded
        let mut bad = mk(&[]);
        bad.rows[0][4] = el(var("w"));
        assert!(!matches!(
            recognize_pattern(&bad),
            Ok(MatrixKind::Banded { .. })
        ));
        // For 4×4, k=2 degenerates to a full matrix → not handled by the banded branch (only
        // attempted when m≥5)
    }

    #[test]
    fn recognize_blockdiag2() {
        // Block diagonal = diag(p·I₂, q·I₂): the block diagonal is p, the block off-diagonal is
        // blank, everything outside the blocks is blank
        let bd = pat(vec![
            vec![el(var("p")), D, el(num(0.0)), el(num(0.0))],
            vec![D, el(var("p")), el(num(0.0)), el(num(0.0))],
            vec![el(num(0.0)), el(num(0.0)), el(var("q")), D],
            vec![el(num(0.0)), el(num(0.0)), D, el(var("q"))],
        ]);
        match recognize_pattern(&bd).unwrap() {
            MatrixKind::BlockDiag2 { .. } => {}
            other => panic!("期望 BlockDiag2，得到 {other:?}"),
        }
        // The all-ellipsis form (both inside and outside the blocks denoted by ⋯) is equivalent
        let bd2 = pat(vec![
            vec![el(var("p")), D, D, D],
            vec![D, el(var("p")), D, D],
            vec![D, D, el(var("q")), D],
            vec![D, D, D, el(var("q"))],
        ]);
        match recognize_pattern(&bd2).unwrap() {
            MatrixKind::BlockDiag2 { .. } => {}
            other => panic!("省略号形态应为 BlockDiag2，得到 {other:?}"),
        }
        // Coupling outside the blocks → not block diagonal
        let bad = pat(vec![
            vec![el(var("p")), D, el(var("c")), el(num(0.0))],
            vec![D, el(var("p")), el(num(0.0)), el(num(0.0))],
            vec![el(num(0.0)), el(num(0.0)), el(var("q")), D],
            vec![el(num(0.0)), el(num(0.0)), D, el(var("q"))],
        ]);
        assert!(!matches!(
            recognize_pattern(&bad),
            Ok(MatrixKind::BlockDiag2 { .. })
        ));
    }

    #[test]
    fn recognize_bidiag_both_orientations() {
        // Upper bidiagonal: main diagonal b + superdiagonal c, the whole lower triangle is
        // ellipses
        let up = pat(vec![
            vec![el(var("b")), el(var("c")), D, D],
            vec![D, el(var("b")), el(var("c")), D],
            vec![D, D, el(var("b")), el(var("c"))],
            vec![D, D, D, el(var("b"))],
        ]);
        match recognize_pattern(&up).unwrap() {
            MatrixKind::BidiagUpper { .. } => {}
            other => panic!("期望 BidiagUpper，得到 {other:?}"),
        }
        // Lower bidiagonal: main diagonal b + subdiagonal a, the whole upper triangle is
        // ellipses
        let lo = pat(vec![
            vec![el(var("b")), D, D, D],
            vec![el(var("a")), el(var("b")), D, D],
            vec![D, el(var("a")), el(var("b")), D],
            vec![D, D, el(var("a")), el(var("b"))],
        ]);
        match recognize_pattern(&lo).unwrap() {
            MatrixKind::BidiagLower { .. } => {}
            other => panic!("期望 BidiagLower，得到 {other:?}"),
        }
        // Generated source: both orientations land in the right positions
        let su = emit_matrix_ctor(
            "BU",
            &MatrixKind::BidiagUpper {
                b: var("b"),
                c: var("c"),
            },
        );
        assert!(
            su.contains("m[__row][__next] = cv;") && !su.contains("m[__next]"),
            "{su}"
        );
        let sl = emit_matrix_ctor(
            "BL",
            &MatrixKind::BidiagLower {
                a: var("a"),
                b: var("b"),
            },
        );
        assert!(sl.contains("m[__next][__row] = av;"), "{sl}");
    }

    #[test]
    fn recognize_unknown_shape_rejected_with_guidance() {
        // 2×2 has no meaningful ellipses; 3×3 but with a jumbled diagonal → an error listing the
        // three supported shapes
        let p = pat(vec![
            vec![el(var("x")), D, el(var("y"))],
            vec![D, el(var("q")), D],
            vec![el(var("z")), D, el(var("w"))],
        ]);
        let text = recognize_pattern(&p).unwrap_err().render(i18n::Locale::En);
        assert!(
            text.contains("identity") && text.contains("diag") && text.contains("tridiag"),
            "{text}"
        );
    }

    #[test]
    fn to_latex_dots_directions() {
        let p = Expr::MatPattern(pat(vec![
            vec![el(var("a")), D, el(var("b"))],
            vec![D, D, D],
            vec![el(var("c")), D, el(var("d"))],
        ]));
        let tex = to_latex(&p);
        assert!(tex.contains("\\cdots"), "{tex}");
        assert!(tex.contains("\\vdots"), "{tex}");
        assert!(tex.contains("\\ddots"), "{tex}");
        assert!(tex.contains("bmatrix"), "{tex}");
    }

    #[test]
    fn solve_system_dimensions_are_checked_on_both_sides() {
        // An equation is legal only if it is **dimensionally homogeneous** (both sides of the
        // addition/subtraction share a dimension): this is where "dimension is a promise"
        // lands in the solving lane. Previously it was only a by-product of dim_check checking arg0
        // first,
        // and here it is pinned down as a contract — if someone changes the order of the Call
        // branch, this turns red.
        let dims = dims_of(&[
            Decl {
                name: "k".into(),
                quantity: "stiffness".into(),
                unit: Some("N/m".into()),
            },
            Decl {
                name: "F".into(),
                quantity: "load".into(),
                unit: Some("N".into()),
            },
            Decl {
                name: "x".into(),
                quantity: "displacement".into(),
                unit: Some("m".into()),
            },
        ])
        .unwrap();
        // k·x − F = 0: both sides are N → legal, and the solved x has dimension N/(N/m) = m
        let good = Expr::Call(
            "solve".into(),
            vec![
                Expr::Mat(vec![vec![sub(mul(var("k"), var("x")), var("F"))]]),
                Expr::Mat(vec![vec![var("x")]]),
            ],
        );
        assert_eq!(
            dim_check(&good, &dims).unwrap(),
            parse_unit("m").unwrap(),
            "解的列向量元素量纲应是位移"
        );
        // x − F = 0: m vs N → dimensions not homogeneous, must be rejected (not "make do by solving
        // for an F")
        let bad = Expr::Call(
            "solve".into(),
            vec![
                Expr::Mat(vec![vec![sub(var("x"), var("F"))]]),
                Expr::Mat(vec![vec![var("x")]]),
            ],
        );
        let err = dim_check(&bad, &dims).unwrap_err();
        assert_eq!(err.code(), "dimension-mismatch", "{err}");
    }

    #[test]
    fn roots_dimension_equals_variable_dimension() {
        // The roots' dimension must be **the variable's own** dimension — no extra derivation is
        // needed: the equation must be dimensionally
        // homogeneous (every term of Σ aᵢxⁱ shares a dimension), and homogeneity directly implies
        // dim(roots) = dim(x).
        // This test pins that reasoning down.
        let dims = dims_of(&[
            Decl {
                name: "a".into(),
                quantity: "inv_len".into(),
                unit: Some("1/m".into()),
            },
            Decl {
                name: "b".into(),
                quantity: "one".into(),
                unit: Some("1".into()),
            },
            Decl {
                name: "c".into(),
                quantity: "len".into(),
                unit: Some("m".into()),
            },
            Decl {
                name: "x".into(),
                quantity: "position".into(),
                unit: Some("m".into()),
            },
        ])
        .unwrap();
        // a·x² + b·x + c (every term is m) → dim(roots) = m
        let poly = add(
            sub(
                mul(var("a"), pow(var("x"), num(2.0))),
                mul(var("b"), var("x")),
            ),
            var("c"),
        );
        let call = Expr::Call("roots".into(), vec![poly, var("x")]);
        assert_eq!(
            dim_check(&call, &dims).unwrap(),
            parse_unit("m").unwrap(),
            "二次方程的根应是长度"
        );
        // Wrong variable dimension: if x is declared as s, the same polynomial is no longer
        // homogeneous → must be rejected
        let mut bad = dims.clone();
        bad.insert("x".to_string(), parse_unit("s").unwrap());
        let err = dim_check(&call, &bad).unwrap_err();
        assert_eq!(err.code(), "dimension-mismatch", "{err}");
    }

    #[test]
    fn poly_coeffs_requires_a_dimensionless_variable() {
        // The coefficients of a homogeneous polynomial have different dimensions per degree
        // (c_k = dim(poly)/dim(x)^k),
        // while a matrix requires elements of the same dimension — so coefficients can only be
        // extracted for a dimensionless variable.
        // This check must be **explained at the parse/diagnosis stage**; downstream must not report
        // "inconsistent matrix element dimensions".
        let dims = dims_of(&[
            Decl {
                name: "k".into(),
                quantity: "stiffness_quad".into(),
                // k·x² and the constant term F share a dimension (N): the polynomial itself is
                // homogeneous, but the **coefficients** have different dimensions
                unit: Some("N/m^2".into()),
            },
            Decl {
                name: "F".into(),
                quantity: "load".into(),
                unit: Some("N".into()),
            },
            Decl {
                name: "x".into(),
                quantity: "len".into(),
                unit: Some("m".into()),
            },
            Decl {
                name: "s".into(),
                quantity: "norm".into(),
                unit: Some("1".into()),
            },
        ])
        .unwrap();
        // A dimensioned variable → reject, with an actionable message
        let poly = add(mul(var("k"), pow(var("x"), num(2.0))), var("F"));
        let call = Expr::Call("poly_coeffs".into(), vec![poly, var("x")]);
        let err = dim_check(&call, &dims).unwrap_err();
        assert_eq!(err.code(), "invalid-input", "{err}");
        let en = err.render(crate::i18n::Locale::En);
        assert!(
            en.contains("dimension") && en.contains("dimensionless"),
            "{en}"
        );
        // A dimensionless variable → pass, and the result is a dimensionless matrix
        let ok = Expr::Call(
            "poly_coeffs".into(),
            vec![
                add(
                    mul(num(2.0), pow(var("s"), num(2.0))),
                    mul(num(3.0), var("s")),
                ),
                var("s"),
            ],
        );
        assert_eq!(dim_check(&ok, &dims).unwrap(), Dim::ZERO);
    }

    #[test]
    fn dim_check_matrix_family() {
        let decls = vec![
            Decl {
                name: "a".into(),
                quantity: "l".into(),
                unit: Some("m".into()),
            },
            Decl {
                name: "b".into(),
                quantity: "l".into(),
                unit: Some("m".into()),
            },
            Decl {
                name: "t".into(),
                quantity: "t".into(),
                unit: Some("s".into()),
            },
        ];
        let dims = dims_of(&decls).unwrap();
        // Concrete matrix: elements share a dimension → OK
        let m = Expr::Mat(vec![vec![var("a"), var("b")], vec![var("b"), var("a")]]);
        assert_eq!(dim_check(&m, &dims).unwrap(), parse_unit("m").unwrap());
        // Inconsistent element dimensions → reject
        let bad = Expr::Mat(vec![vec![var("a")], vec![var("t")]]);
        assert!(dim_check(&bad, &dims).is_err());
        // Pattern matrix: the concrete elements agree → OK; Dots do not participate
        let pm = Expr::MatPattern(pat(vec![
            vec![el(var("a")), D, el(num(0.0))],
            vec![D, el(var("a")), D],
            vec![el(num(0.0)), D, el(var("a"))],
        ]));
        assert!(dim_check(&pm, &dims).is_ok());
        let pbad = Expr::MatPattern(pat(vec![
            vec![el(var("a")), D, el(num(0.0))],
            vec![D, el(var("t")), D],
            vec![el(num(0.0)), D, el(var("a"))],
        ]));
        assert!(dim_check(&pbad, &dims).is_err());
    }

    #[test]
    fn mathml_shapes() {
        // Fraction
        let x = to_mathml(&div(var("a"), var("b")));
        assert!(x.contains("<mfrac>"), "{x}");
        // Superscript
        let p = to_mathml(&pow(var("x"), num(2.0)));
        assert!(x.contains("<mn>2</mn>") || p.contains("<msup>"), "{p}");
        // Indexed symbol
        let ix = to_mathml(&indexed("a", vec!["i".into(), "j".into()]));
        assert!(ix.contains("<msub>") && ix.contains("<mi>j</mi>"), "{ix}");
        // Summation (bounds)
        let sm = to_mathml(&sum("i", num(1.0), num(3.0), pow(var("i"), num(2.0))));
        assert!(sm.contains("<munderover>"), "{sm}");
        // Escaping
        let esc = to_mathml(&var("a<b"));
        assert!(esc.contains("&lt;"), "{esc}");
    }

    #[test]
    fn matrix_mul_eval_and_shapes() {
        use Value::Matrix;
        // [1 2; 3 4] × [5 6; 7 8] = [19 22; 43 50]
        let a = Expr::Mat(vec![vec![num(1.0), num(2.0)], vec![num(3.0), num(4.0)]]);
        let b = Expr::Mat(vec![vec![num(5.0), num(6.0)], vec![num(7.0), num(8.0)]]);
        let prod = mul(a.clone(), b.clone());
        shape_check(&prod).unwrap();
        match eval_value(&prod, &HashMap::new()).unwrap() {
            Matrix(m) => {
                let got: Vec<Vec<f64>> =
                    m.iter().map(|r| r.iter().map(|c| c.re).collect()).collect();
                assert_eq!(got, vec![vec![19.0, 22.0], vec![43.0, 50.0]]);
                assert!(
                    m.iter().flatten().all(|c| c.is_real()),
                    "实数矩阵不应带虚部"
                );
            }
            _ => panic!("期望矩阵"),
        }
        // Transposed shape
        assert_eq!(shape_of(&transpose(a.clone())).unwrap(), Some((2, 2)));
        let row = Expr::Mat(vec![vec![num(1.0), num(2.0), num(3.0)]]);
        assert_eq!(shape_of(&transpose(row)).unwrap(), Some((3, 1)));
    }

    #[test]
    fn matrix_shape_violations_rejected() {
        // 2×3 × 2×3 is not multipliable
        let a = Expr::Mat(vec![
            vec![num(1.0), num(2.0), num(3.0)],
            vec![num(4.0), num(5.0), num(6.0)],
        ]);
        let b = a.clone();
        let err = shape_check(&mul(a, b)).unwrap_err();
        assert_eq!(err.code(), "shape-mismatch", "{err}");
        // The payload names both factors, so assert it rather than merely the code.
        match &err {
            Error::ShapeMismatch(why) => {
                assert!(
                    why.contains("cannot multiply") && why.contains("2"),
                    "{why}"
                );
            }
            other => panic!("{other:?}"),
        }
        // Inconsistent shapes for addition
        let r2 = Expr::Mat(vec![vec![num(1.0)], vec![num(2.0)]]);
        let r3 = Expr::Mat(vec![vec![num(1.0)], vec![num(2.0)], vec![num(3.0)]]);
        assert_eq!(
            shape_check(&add(r2, r3)).unwrap_err().code(),
            "shape-mismatch"
        );
        // Jagged matrix
        let jag = Expr::Mat(vec![vec![num(1.0), num(2.0)], vec![num(3.0)]]);
        assert!(shape_of(&jag).is_err());
        // Matrix division is rejected explicitly
        let m = Expr::Mat(vec![vec![num(1.0)]]);
        let s = Expr::Mat(vec![vec![num(2.0)]]);
        assert_eq!(
            shape_check(&div(m, s)).unwrap_err().code(),
            "matrix-division-unsupported"
        );
    }

    #[test]
    fn federate_inline_substitutes_in_order() {
        // a = x + 1; b = a * 2; c = b - a → substituted entry by entry
        let set = FormulaSet {
            decls: vec![Decl {
                name: "x".into(),
                quantity: "d".into(),
                unit: None,
            }],
            uses: vec![],
            items: vec![
                FormulaItem {
                    name: Some("a".into()),
                    expr: add(var("x"), num(1.0)),
                    line: 0,
                },
                FormulaItem {
                    name: Some("b".into()),
                    expr: mul(var("a"), num(2.0)),
                    line: 0,
                },
                FormulaItem {
                    name: Some("c".into()),
                    expr: sub(var("b"), var("a")),
                    line: 0,
                },
            ],
        };
        let fed = federate_inline(&set).unwrap();
        let mut env = HashMap::new();
        env.insert("x".to_string(), 3.0);
        // c = (x+1)*2 - (x+1) = 8 - 4 = 4
        assert_eq!(eval(&fed[2].expr, &env).unwrap(), 4.0);
        // The expression of b no longer references a
        assert!(!vars_of(&fed[1].expr).contains(&"a".to_string()));
    }

    #[test]
    fn matrix_ctors_compile_shape() {
        let src = emit_matrix_ctor(
            "M",
            &MatrixKind::Tridiag {
                a: var("a"),
                b: var("b"),
                c: var("c"),
            },
        );
        assert!(
            src.contains("pub fn M(a: f64, b: f64, c: f64, n: usize)"),
            "{src}"
        );
        assert!(src.contains("m[__row][__next] = cv;"), "{src}");
        let src_i = emit_matrix_ctor("I", &MatrixKind::Identity);
        assert!(src_i.contains("pub fn I(n: usize)"), "{src_i}");
    }

    #[test]
    fn cse_emits_shared_lets() {
        let shared = add(var("a"), var("b"));
        let e = add(mul(shared.clone(), shared.clone()), shared.clone());
        let code = emit_block(&e);
        assert_eq!(code.matches("let __t0 = a + b;").count(), 1, "{code}");
        // 1 definition + 3 uses
        assert_eq!(code.matches("__t0").count(), 4, "{code}");
    }
}
