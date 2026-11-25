//! Uncatalogued-error budget gate.
//!
//! Counts *constructions* of staging errors in production code — `Error::from(..)`,
//! `Error::Message(..)`, `FrontError::plain("…")`, `FrontError::plain(format!(..))`,
//! `FrontError::new(l, c, "…")`, `FrontError::new(l, c, format!(..))` — by walking the Rust
//! AST with `syn`, so the verdict never depends on line layout, whitespace, comments, or
//! whether a pattern *looks* like a construction. Items compiled only under test are skipped —
//! any item kind gated by a `#[cfg(test)]`-entailing predicate, plus `#[test]` functions (the
//! only item kind rustc accepts `#[test]` on); the catalogue itself
//! (`i18n.rs`) is excluded. This is the *string-staging* budget: a catalogued variant built
//! with a string payload (e.g. `Error::SyntaxError(text)`) is NOT staging and is not counted.
//!
//! The ceiling lives in `tools/staging-budget/src/ceiling.txt` and may only go down.
//! `--against-git` reads that file from `HEAD` and fails if the working-tree ceiling is higher;
//! an unreadable ceiling or an unavailable baseline is a hard error, never a silent downgrade.
//!
//! `--list` additionally prints every counted site as `<path>:<line>  <bucket>  <source line>`
//! (path relative to the repository root, line 1-based) before the usual summary, so a batch
//! that retires sites can be reviewed line by line and the before/after lists diffed. The list
//! is produced by the same walk that produces the count, so `list.len()` is always `total`.
//! Any other argument is a usage error (exit 2): a mistyped flag must never silently
//! disable a check.

use std::path::{Path, PathBuf};
use std::process::Command;

use syn::spanned::Spanned;
use syn::{visit::Visit, Expr, ExprPath, Item};

fn parse_ceiling(text: &str) -> Option<usize> {
    let t = text.trim().trim_end_matches(';').trim();
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// The ceiling for THIS tree. No fallback: an unreadable or malformed ceiling would silently
/// disarm the budget, so it is reported as an error and the caller exits 2.
fn ceiling_for(repo: &Path) -> Result<usize, String> {
    let path = repo.join("tools/staging-budget/src/ceiling.txt");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    parse_ceiling(&text)
        .ok_or_else(|| format!("{} is not a usize literal: {text:?}", path.display()))
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct Counts {
    pub error_from: usize,
    pub error_message: usize,
    pub plain_str: usize,
    pub plain_format: usize,
    pub new_str: usize,
    pub new_format: usize,
}

impl Counts {
    fn total(&self) -> usize {
        self.error_from
            + self.error_message
            + self.plain_str
            + self.plain_format
            + self.new_str
            + self.new_format
    }
}

/// One counted construction, as reported by `--list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    /// Path relative to the repository root, with `/` separators.
    pub path: String,
    /// 1-based line of the construction's first token.
    pub line: usize,
    /// Bucket name, identical to the summary's label: `error_from`, `error_message`,
    /// `plain_str`, `plain_format`, `new_str`, `new_format`.
    pub bucket: &'static str,
    /// The source line, trimmed (the construction may span several lines; this is the first).
    pub source: String,
}

impl Site {
    /// `<path>:<line>  <bucket>  <source>` — the exact `--list` line.
    pub fn render(&self) -> String {
        format!(
            "{}:{}  {:<13}  {}",
            self.path, self.line, self.bucket, self.source
        )
    }
}

/// The result of walking a repository: what was counted and where.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Scan {
    pub counts: Counts,
    /// Sorted by `(path, line)`; always `counts.total()` long.
    pub sites: Vec<Site>,
}

/// True only when a `#[cfg(..)]` attribute's predicate is test-only (the item is compiled
/// ONLY under test). `#[cfg_attr(test, ..)]` is NOT a gate — it only adds an attribute under
/// test while the item itself still compiles in production. `#[test]` is handled separately
/// by [`is_test_fn`] because rustc accepts it only on functions.
fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg")
            && matches!(&a.meta, syn::Meta::List(l) if cfg_predicate_is_test(&l.tokens))
    })
}

/// `#[test]` is a test gate only on *free* functions — rustc rejects the attribute on every
/// other item kind, including associated (`impl`/`trait`) methods, with a hard error. Treating
/// it as a gate elsewhere would skip code that can never compile and would pin the wrong
/// semantics in tests. Impl/trait method gating therefore also consults this, so a `#[test]`
/// on such a method (which rustc would reject) is skipped consistently rather than counted.
fn is_test_fn(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("test"))
}

/// Is an item with this `cfg(...)` predicate compiled ONLY under test?
///
/// This is an *entailment* question, not "mentions test": the item is test-only iff the
/// predicate cannot be true unless `test` is. Evaluated over the three-valued lattice
/// `{True, False, DependsOnTest}` where unknown flags (`std`, `unix`, `feature = ".."`,
/// `target_..`) are *not* test-dependent (they may be either, but never require test):
/// `all(..)` is test-only iff ANY operand is test-only; `any(..)` iff ALL operands are;
/// `not(p)` flips only a pure value and never turns a production predicate into a test gate;
/// bare `test` is test-only; `ident = "lit"` is never test-only.
fn cfg_predicate_is_test(tokens: &proc_macro2::TokenStream) -> bool {
    let tts: Vec<proc_macro2::TokenTree> = tokens.clone().into_iter().collect();
    let mut pieces: Vec<Vec<proc_macro2::TokenTree>> = vec![Vec::new()];
    for tt in tts {
        if matches!(&tt, proc_macro2::TokenTree::Punct(p) if p.as_char() == ',') {
            pieces.push(Vec::new());
        } else {
            pieces.last_mut().unwrap().push(tt);
        }
    }
    // Top-level commas mean `all`.
    pieces.iter().any(|piece| predicate_is_test_only(piece))
}

fn predicate_is_test_only(piece: &[proc_macro2::TokenTree]) -> bool {
    let Some(first) = piece.first() else {
        return false;
    };
    let name = match first {
        proc_macro2::TokenTree::Ident(i) => i.to_string(),
        _ => return false,
    };
    match piece.get(1) {
        // `ident = "lit"` — a value predicate; never test-only.
        Some(proc_macro2::TokenTree::Punct(p)) if p.as_char() == '=' => false,
        Some(proc_macro2::TokenTree::Group(g)) => {
            let inner = g.stream();
            let parts: Vec<Vec<proc_macro2::TokenTree>> = {
                let mut v: Vec<Vec<proc_macro2::TokenTree>> = vec![Vec::new()];
                for tt in inner {
                    if matches!(&tt, proc_macro2::TokenTree::Punct(p) if p.as_char() == ',') {
                        v.push(Vec::new());
                    } else {
                        v.last_mut().unwrap().push(tt);
                    }
                }
                v
            };
            match name.as_str() {
                // all(..) needs every operand; it is test-only as soon as one is.
                "all" => parts.iter().any(|p| predicate_is_test_only(p)),
                // any(..) is test-only only if every branch is.
                "any" => parts.iter().all(|p| predicate_is_test_only(p)),
                // not(test) is production-only; not(X) for other X stays non-test.
                "not" => false,
                _ => false,
            }
        }
        // Bare flag.
        _ => name == "test",
    }
}

/// A path whose last segment is `name` and whose qualifier is the type `before` — spelled
/// either as the type name (`himari::Error::X`, `crate::Error::X`, `Error::X`) or as `Self::X`
/// inside an `impl Error` / `impl FrontError` block. An unrelated type's method of the same
/// name (`Foo::Message`, `Baz::new`) is NOT counted.
fn path_ends_with(p: &syn::Path, before: &str, name: &str) -> bool {
    let segs = &p.segments;
    if segs.len() < 2 {
        return false;
    }
    let qualifier = &segs[segs.len() - 2].ident;
    segs[segs.len() - 1].ident == name && (qualifier == before || qualifier == "Self")
}

fn lit_str(e: &Expr) -> bool {
    matches!(e, Expr::Lit(l) if matches!(l.lit, syn::Lit::Str(_)))
}

fn is_format_macro(e: &Expr) -> bool {
    matches!(e, Expr::Macro(m) if m.mac.path.is_ident("format"))
}

#[derive(Default)]
struct Counter {
    c: Counts,
    sites: Vec<Site>,
    test_depth: usize,
    /// Repository-relative path and the file's lines, for `Site` reporting.
    path: String,
    lines: Vec<String>,
}

impl Counter {
    /// Record one counted construction at the expression's first token.
    fn record(&mut self, bucket: &'static str, e: &Expr) {
        let line = e.span().start().line;
        let source = self
            .lines
            .get(line.saturating_sub(1))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        self.sites.push(Site {
            path: self.path.clone(),
            line,
            bucket,
            source,
        });
    }
}

impl<'ast> Visit<'ast> for Counter {
    fn visit_item(&mut self, i: &'ast Item) {
        // Any item kind can carry `#[cfg(test)]` (mod, fn, impl, const, static, trait, use,
        // extern crate, …); gate on whichever attrs it has. `#[test]` additionally gates a
        // free function.
        let gated = item_attrs(i).is_some_and(is_cfg_test)
            || matches!(i, Item::Fn(f) if is_test_fn(&f.attrs));
        if gated {
            self.test_depth += 1;
            syn::visit::visit_item(self, i);
            self.test_depth -= 1;
            return;
        }
        syn::visit::visit_item(self, i);
    }

    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        if is_cfg_test(&f.attrs) || is_test_fn(&f.attrs) {
            self.test_depth += 1;
            syn::visit::visit_impl_item_fn(self, f);
            self.test_depth -= 1;
            return;
        }
        syn::visit::visit_impl_item_fn(self, f);
    }

    fn visit_trait_item_fn(&mut self, f: &'ast syn::TraitItemFn) {
        if is_cfg_test(&f.attrs) || is_test_fn(&f.attrs) {
            self.test_depth += 1;
            syn::visit::visit_trait_item_fn(self, f);
            self.test_depth -= 1;
            return;
        }
        syn::visit::visit_trait_item_fn(self, f);
    }

    fn visit_expr(&mut self, e: &'ast Expr) {
        if self.test_depth == 0 {
            self.classify(e);
        }
        syn::visit::visit_expr(self, e);
    }
}

/// Count the staging constructions in one parsed file, and record where each one is.
/// `path` is repository-relative; `src` is the text `file` was parsed from.
fn count_file(file: &syn::File, path: &str, src: &str) -> (Counts, Vec<Site>) {
    let mut c = Counter {
        path: path.to_string(),
        lines: src.lines().map(str::to_owned).collect(),
        ..Counter::default()
    };
    c.visit_file(file);
    (c.c, c.sites)
}

fn item_attrs(i: &Item) -> Option<&[syn::Attribute]> {
    Some(match i {
        Item::Mod(x) => &x.attrs,
        Item::Fn(x) => &x.attrs,
        Item::Use(x) => &x.attrs,
        Item::ExternCrate(x) => &x.attrs,
        Item::Impl(x) => &x.attrs,
        Item::Const(x) => &x.attrs,
        Item::Static(x) => &x.attrs,
        Item::Trait(x) => &x.attrs,
        Item::TraitAlias(x) => &x.attrs,
        Item::Type(x) => &x.attrs,
        Item::Struct(x) => &x.attrs,
        Item::Enum(x) => &x.attrs,
        Item::Union(x) => &x.attrs,
        Item::Macro(x) => &x.attrs,
        _ => return None,
    })
}

impl Counter {
    fn classify(&mut self, e: &Expr) {
        let Expr::Call(call) = e else { return };
        let Expr::Path(ExprPath { path, .. }) = &*call.func else {
            return;
        };
        let args = &call.args;
        // Error::from(..)
        if path_ends_with(path, "Error", "from") {
            self.c.error_from += 1;
            self.record("error_from", e);
            return;
        }
        // Error::Message(..) — a *call* is a construction by definition; patterns are not Exprs.
        if path_ends_with(path, "Error", "Message") {
            self.c.error_message += 1;
            self.record("error_message", e);
            return;
        }
        // FrontError::plain("…") / plain(format!(..))
        if path_ends_with(path, "FrontError", "plain") {
            if let Some(a) = args.first() {
                if lit_str(a) {
                    self.c.plain_str += 1;
                    self.record("plain_str", e);
                } else if is_format_macro(a) {
                    self.c.plain_format += 1;
                    self.record("plain_format", e);
                }
            }
            return;
        }
        // FrontError::new(line, col, "…" | format!(..))
        if path_ends_with(path, "FrontError", "new") {
            if let Some(a) = args.iter().nth(2) {
                if lit_str(a) {
                    self.c.new_str += 1;
                    self.record("new_str", e);
                } else if is_format_macro(a) {
                    self.c.new_format += 1;
                    self.record("new_format", e);
                }
            }
        }
    }
}

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let p = entry.path();
        // Exclusions are path-component matches on the path RELATIVE TO THE SCAN ROOT, never
        // substrings and never on the absolute path: a checkout that lives under a directory
        // called `target` must still be scanned, while a package's own build `target/` is not.
        // `fixtures/` holds deliberately-invalid negative probes (bare LaTeX that must fail to
        // lex); they are not production code and are not parseable by design.
        let rel = p
            .strip_prefix(root)
            .expect("walkdir yields paths under its root");
        let component = |name: &str| rel.components().any(|c| c.as_os_str() == name);
        if !p.extension().is_some_and(|e| e == "rs")
            || component("target")
            || component("tests")
            || component("fixtures")
        {
            continue;
        }
        let name = p.file_name().unwrap_or_default().to_string_lossy();
        if name.ends_with("_test.rs") || name.starts_with("test_") || name == "i18n.rs" {
            continue;
        }
        out.push(p.to_path_buf());
    }
    out.sort();
    out
}

fn baseline_ceiling(repo: &Path) -> Result<usize, String> {
    let out = Command::new("git")
        .args(["show", "HEAD:tools/staging-budget/src/ceiling.txt"])
        .current_dir(repo)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git show HEAD:tools/staging-budget/src/ceiling.txt failed ({}); a shallow/absent HEAD cannot vouch for monotonicity",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    parse_ceiling(&text)
        .ok_or_else(|| format!("HEAD's ceiling.txt is not a usize literal: {text:?}"))
}

/// The gate's verdict, with the process exit code it maps to.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Within budget (and monotone when requested).
    Clean { counts: Counts, ceiling: usize },
    /// Over budget or ceiling raised: exit 1.
    OverBudget(String),
    /// Cannot vouch (no roots, parse error, unreadable ceiling, unavailable baseline): exit 2.
    CannotVouch(String),
}

impl Verdict {
    pub fn exit_code(&self) -> i32 {
        match self {
            Verdict::Clean { .. } => 0,
            Verdict::OverBudget(_) => 1,
            Verdict::CannotVouch(_) => 2,
        }
    }
}

/// Walk every production Rust file under `packages/`, `examples/` and `tools/` and return what
/// was counted and where. Errors are the reasons the gate cannot vouch (no roots, no files,
/// unparseable sources) — the caller maps them onto `Verdict::CannotVouch`.
pub fn scan(repo: &Path) -> Result<Scan, String> {
    // A root that has none of the production trees cannot vouch for anything.
    let roots: Vec<PathBuf> = ["packages", "examples", "tools"]
        .iter()
        .map(|r| repo.join(r))
        .filter(|r| r.is_dir())
        .collect();
    if roots.is_empty() {
        return Err(format!(
            "no packages/, examples/ or tools/ under {} — refusing a vacuous zero",
            repo.display()
        ));
    }
    let mut files = Vec::new();
    for r in &roots {
        files.extend(rust_files(r));
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        return Err(format!(
            "no Rust files under {roots:?} — refusing a vacuous zero"
        ));
    }

    let mut out = Scan::default();
    let mut parse_errors: Vec<String> = Vec::new();
    for f in &files {
        let src = std::fs::read_to_string(f).unwrap_or_default();
        let file: syn::File = match syn::parse_file(&src) {
            Ok(f) => f,
            Err(e) => {
                parse_errors.push(format!("cannot parse {}: {e}", f.display()));
                continue;
            }
        };
        let rel = f
            .strip_prefix(repo)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        let (c, sites) = count_file(&file, &rel, &src);
        out.counts.error_from += c.error_from;
        out.counts.error_message += c.error_message;
        out.counts.plain_str += c.plain_str;
        out.counts.plain_format += c.plain_format;
        out.counts.new_str += c.new_str;
        out.counts.new_format += c.new_format;
        out.sites.extend(sites);
    }
    if !parse_errors.is_empty() {
        return Err(format!(
            "{} file(s) failed to parse — the gate cannot vouch for them: {}",
            parse_errors.len(),
            parse_errors.join("; ")
        ));
    }
    // Sites are already in `(path, line)` order: files are walked in sorted path order and
    // `syn` visits items and expressions in source order, so no explicit sort is needed. The
    // ordering test pins this; change the walk and it goes red.
    Ok(out)
}

/// Runs the gate against `repo` and returns the verdict — no process exits, no stdout, so the
/// whole contract (exit codes included) is unit-testable. `main` only maps it onto the process.
pub fn run(repo: &Path, against_git: bool) -> Verdict {
    let total_counts = match scan(repo) {
        Ok(s) => s.counts,
        Err(why) => return Verdict::CannotVouch(why),
    };

    // No fallback: an unreadable or malformed ceiling would silently disarm the budget, so it
    // is a hard error exactly like an unavailable baseline.
    let ceiling = match ceiling_for(repo) {
        Ok(c) => c,
        Err(why) => return Verdict::CannotVouch(why),
    };
    if total_counts.total() > ceiling {
        return Verdict::OverBudget(format!(
            "{} > {ceiling} (retire sites, then lower the ceiling)",
            total_counts.total()
        ));
    }
    if against_git {
        // Never silently downgrade to count-only: an unverifiable baseline is a hard error.
        match baseline_ceiling(repo) {
            Err(why) => {
                return Verdict::CannotVouch(format!(
                    "--against-git requested but the baseline is unavailable: {why}"
                ))
            }
            Ok(base) if ceiling > base => {
                return Verdict::OverBudget(format!(
                    "ceiling raised {base} -> {ceiling}; it may only go down"
                ))
            }
            Ok(_) => {}
        }
    }
    Verdict::Clean {
        counts: total_counts,
        ceiling,
    }
}

/// The command line, with stdout injected so the `--list` path is unit-testable. `args` excludes
/// the program name; `cwd` is where `--root` is resolved against. Returns the process exit code
/// (0 clean, 1 over budget, 2 cannot vouch); failures still go to stderr like the real CLI.
fn cli(args: &[String], cwd: &Path, ci_env: bool, out: &mut dyn std::io::Write) -> i32 {
    let mut repo = cwd.to_path_buf();
    let mut against_git = false;
    let mut list = false;
    // Every argument must be a known flag. A typo fails loudly with exit 2 — it must never
    // silently disable a check (`--against-gi` in CI would otherwise skip the monotonicity
    // gate), and nothing but the flags below may influence the verdict.
    let mut it = args.iter().enumerate();
    while let Some((i, a)) = it.next() {
        match a.as_str() {
            "--against-git" => against_git = true,
            "--list" => list = true,
            "--root" => match args.get(i + 1) {
                Some(v) => {
                    repo = PathBuf::from(v);
                    it.next(); // the value is consumed, never treated as a flag
                }
                None => {
                    eprintln!("staging-budget: --root requires a directory argument");
                    return 2;
                }
            },
            other => {
                eprintln!(
                    "staging-budget: unknown argument `{other}` (expected --against-git, --list or --root <dir>)"
                );
                return 2;
            }
        }
    }
    // Under CI the monotonicity check is implied, exactly as before.
    if ci_env {
        against_git = true;
    }
    // `--list`: one line per counted site, before the summary. Printed for over-budget trees too
    // (that is when the list is most wanted); when the walk itself fails, `run` reports why.
    if list {
        if let Ok(s) = scan(&repo) {
            for site in &s.sites {
                let _ = writeln!(out, "{}", site.render());
            }
        }
    }
    match run(&repo, against_git) {
        Verdict::Clean { counts: c, ceiling } => {
            let _ = writeln!(out, "error_from     {}", c.error_from);
            let _ = writeln!(out, "error_message  {}", c.error_message);
            let _ = writeln!(out, "plain_str      {}", c.plain_str);
            let _ = writeln!(out, "plain_format   {}", c.plain_format);
            let _ = writeln!(out, "new_str        {}", c.new_str);
            let _ = writeln!(out, "new_format     {}", c.new_format);
            let _ = writeln!(out, "total          {}  (ceiling {ceiling})", c.total());
            let _ = writeln!(out, "staging-budget: clean");
            0
        }
        Verdict::OverBudget(why) => {
            eprintln!("staging-budget: FAIL — {why}");
            1
        }
        Verdict::CannotVouch(why) => {
            eprintln!("staging-budget: FAIL — {why}");
            2
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let ci_env = std::env::var_os("CI").is_some();
    std::process::exit(cli(&args, &cwd, ci_env, &mut std::io::stdout().lock()));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a scratch workspace: `<leaf>/packages/p/src/lib.rs` (+ `tools/staging-budget/src/ceiling.txt`
    /// with the given ceiling) and returns (leaf, guard).
    struct Workspace {
        leaf: std::path::PathBuf,
        _guard: TempRoot,
    }

    impl Workspace {
        fn new(ceiling: &str, lib_body: &str) -> Self {
            // Unique per instance: tests run in parallel and must never share a leaf.
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let leaf = std::env::temp_dir().join(format!("sb-unit-ws-{}-{n}", std::process::id()));
            let _ = std::fs::remove_dir_all(&leaf);
            let guard = TempRoot(leaf.clone());
            std::fs::create_dir_all(leaf.join("packages/p/src")).unwrap();
            std::fs::create_dir_all(leaf.join("tools/staging-budget/src")).unwrap();
            std::fs::write(leaf.join("tools/staging-budget/src/ceiling.txt"), ceiling).unwrap();
            std::fs::write(leaf.join("packages/p/src/lib.rs"), lib_body).unwrap();
            Self {
                leaf,
                _guard: guard,
            }
        }
        fn path(&self) -> &Path {
            &self.leaf
        }
    }

    const ONE_SITE: &str = r#"fn f(x: String) -> Result<(), Error> { Err(Error::Message(x)) }"#;
    const THREE_SITES: &str = r#"
fn f(x: String) -> Result<(), Error> { Err(Error::Message(x)) }
fn g() -> Result<(), Error> { Err(Error::from("x")) }
fn h() -> Result<(), FrontError> { Err(FrontError::plain("x")) }
"#;

    #[test]
    fn clean_within_budget_exits_zero_and_reports_every_bucket() {
        let ws = Workspace::new("10usize", THREE_SITES);
        let v = run(ws.path(), false);
        assert_eq!(v.exit_code(), 0, "{v:?}");
        let Verdict::Clean { counts, ceiling } = v else {
            panic!("{v:?}")
        };
        assert_eq!(ceiling, 10);
        assert_eq!(
            (
                counts.error_from,
                counts.error_message,
                counts.plain_str,
                counts.plain_format,
                counts.new_str,
                counts.new_format
            ),
            (1, 1, 1, 0, 0, 0)
        );
        assert_eq!(counts.total(), 3);
    }

    #[test]
    fn over_budget_exits_one() {
        let ws = Workspace::new("2usize", THREE_SITES);
        let v = run(ws.path(), false);
        assert_eq!(v.exit_code(), 1, "{v:?}");
        assert!(
            matches!(v, Verdict::OverBudget(ref w) if w.contains("3 > 2")),
            "{v:?}"
        );
    }

    #[test]
    fn ceiling_exactly_at_total_is_clean() {
        let ws = Workspace::new("3usize", THREE_SITES);
        assert_eq!(run(ws.path(), false).exit_code(), 0);
    }

    #[test]
    fn unreadable_or_malformed_ceiling_is_a_hard_error_not_a_fallback() {
        // Missing file.
        let ws = Workspace::new("10usize", ONE_SITE);
        std::fs::remove_file(ws.path().join("tools/staging-budget/src/ceiling.txt")).unwrap();
        let v = run(ws.path(), false);
        assert_eq!(v.exit_code(), 2, "{v:?}");
        assert!(
            matches!(v, Verdict::CannotVouch(ref w) if w.contains("cannot read")),
            "{v:?}"
        );
        // Malformed content — must NOT be read as 0 or any default.
        let ws = Workspace::new("not-a-number", ONE_SITE);
        let v = run(ws.path(), false);
        assert_eq!(v.exit_code(), 2, "{v:?}");
        assert!(
            matches!(v, Verdict::CannotVouch(ref w) if w.contains("not a usize literal")),
            "{v:?}"
        );
        let ws = Workspace::new("", ONE_SITE);
        assert_eq!(run(ws.path(), false).exit_code(), 2);
    }

    #[test]
    fn no_roots_or_no_rust_files_refuses_a_vacuous_zero() {
        let leaf = std::env::temp_dir().join(format!("sb-unit-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&leaf);
        let _g = TempRoot(leaf.clone());
        std::fs::create_dir_all(&leaf).unwrap();
        let v = run(&leaf, false);
        assert_eq!(v.exit_code(), 2, "{v:?}");
        // The FIRST guard must be the one that fires (no roots at all), not the no-files guard.
        assert!(
            matches!(v, Verdict::CannotVouch(ref w) if w.contains("no packages/, examples/ or tools/")),
            "{v:?}"
        );
        std::fs::create_dir_all(leaf.join("packages/empty")).unwrap();
        let v = run(&leaf, false);
        assert_eq!(v.exit_code(), 2, "{v:?}");
        assert!(
            matches!(v, Verdict::CannotVouch(ref w) if w.contains("no Rust files")),
            "{v:?}"
        );
    }

    #[test]
    fn an_unparseable_production_file_is_fatal() {
        let ws = Workspace::new("10usize", ONE_SITE);
        std::fs::write(
            ws.path().join("packages/p/src/broken.rs"),
            "fn {{{{ not rust",
        )
        .unwrap();
        let v = run(ws.path(), false);
        assert_eq!(v.exit_code(), 2, "{v:?}");
        assert!(
            matches!(v, Verdict::CannotVouch(ref w) if w.contains("cannot parse") && w.contains("broken.rs")),
            "{v:?}"
        );
    }

    #[test]
    fn per_file_aggregation_sums_every_bucket_across_files_and_trees() {
        let ws = Workspace::new(
            "100usize",
            r#"fn a(x: String) -> Result<(), Error> { Err(Error::Message(x)) }"#,
        );
        std::fs::create_dir_all(ws.path().join("examples/e/src")).unwrap();
        std::fs::create_dir_all(ws.path().join("tools/t/src")).unwrap();
        std::fs::write(
            ws.path().join("examples/e/src/lib.rs"),
            r#"fn b() -> Result<(), Error> { Err(Error::from("x")) }"#,
        )
        .unwrap();
        std::fs::write(
            ws.path().join("tools/t/src/lib.rs"),
            r#"fn c() -> Result<(), FrontError> { Err(FrontError::plain("x")) }
fn d() -> Result<(), FrontError> { Err(FrontError::new(0, 0, "x")) }"#,
        )
        .unwrap();
        std::fs::write(
            ws.path().join("packages/p/src/extra.rs"),
            r#"fn e() -> Result<(), FrontError> { Err(FrontError::plain(format!("x"))) }
fn f2() -> Result<(), FrontError> { Err(FrontError::new(0, 0, format!("x"))) }"#,
        )
        .unwrap();
        let v = run(ws.path(), false);
        let Verdict::Clean { counts, .. } = v else {
            panic!("{v:?}")
        };
        // Each bucket receives exactly one from a DIFFERENT file/tree — a dropped or misrouted
        // `+=` anywhere in the aggregation shows up as a wrong tuple.
        assert_eq!(
            (
                counts.error_from,
                counts.error_message,
                counts.plain_str,
                counts.plain_format,
                counts.new_str,
                counts.new_format
            ),
            (1, 1, 1, 1, 1, 1),
            "{counts:?}"
        );
    }

    #[test]
    fn filename_exclusions_apply_only_to_the_documented_names() {
        let ws = Workspace::new("100usize", ONE_SITE);
        for name in ["foo_test.rs", "test_foo.rs", "i18n.rs"] {
            std::fs::write(ws.path().join("packages/p/src").join(name), ONE_SITE).unwrap();
        }
        // These look similar but are NOT excluded.
        std::fs::write(ws.path().join("packages/p/src/foo_tests.rs"), ONE_SITE).unwrap();
        std::fs::write(ws.path().join("packages/p/src/target_tests.rs"), ONE_SITE).unwrap();
        let Verdict::Clean { counts, .. } = run(ws.path(), false) else {
            panic!()
        };
        assert_eq!(
            counts.error_message, 3,
            "lib.rs + foo_tests.rs + target_tests.rs: {counts:?}"
        );
    }

    #[test]
    fn self_qualified_constructions_are_counted_and_unrelated_types_are_not() {
        let ws = Workspace::new(
            "100usize",
            r#"impl Error { fn m(x: String) -> Result<(), Error> { Err(Self::Message(x)) } }
impl FrontError { fn m() -> Result<(), FrontError> { Err(Self::plain("x")) } }
fn u1(x: String) -> Result<(), Foo> { Err(Foo::Message(x)) }
fn u2() -> Result<(), Baz> { Err(Baz::plain("x")) }"#,
        );
        let Verdict::Clean { counts, .. } = run(ws.path(), false) else {
            panic!()
        };
        assert_eq!(
            (counts.error_message, counts.plain_str),
            (1, 1),
            "{counts:?}"
        );
    }

    #[test]
    fn against_git_without_a_git_repo_is_a_hard_error() {
        let ws = Workspace::new("10usize", ONE_SITE);
        // The temp leaf is not a git repo → baseline unavailable → exit 2 (never a silent downgrade).
        let v = run(ws.path(), true);
        assert_eq!(v.exit_code(), 2, "{v:?}");
        assert!(
            matches!(v, Verdict::CannotVouch(ref w) if w.contains("baseline is unavailable")),
            "{v:?}"
        );
    }

    /// Commits the scratch workspace as a git repo with hooks disabled so the seed commit
    /// never trips a workspace-level pre-commit hook.
    fn git(args: &[&str], cwd: &Path) {
        let status = std::process::Command::new("git")
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
            ])
            .args(args)
            .current_dir(cwd)
            .status()
            .expect("git must be available");
        assert!(status.success(), "git {args:?} failed");
    }

    #[test]
    fn against_git_fails_when_the_ceiling_is_raised_and_passes_when_lowered() {
        let ws = Workspace::new("10usize", ONE_SITE);
        git(&["init", "-q"], ws.path());
        git(&["add", "-A"], ws.path());
        git(&["commit", "-q", "--no-verify", "-m", "seed"], ws.path());
        // Equal → clean.
        assert_eq!(run(ws.path(), true).exit_code(), 0);
        // Raised → exit 1 with the "may only go down" message.
        std::fs::write(
            ws.path().join("tools/staging-budget/src/ceiling.txt"),
            "11usize",
        )
        .unwrap();
        let v = run(ws.path(), true);
        assert_eq!(v.exit_code(), 1, "{v:?}");
        assert!(
            matches!(v, Verdict::OverBudget(ref w) if w.contains("raised 10 -> 11")),
            "{v:?}"
        );
        // Lowered below the real count → exit 1 for the count reason (monotone, but over).
        std::fs::write(
            ws.path().join("tools/staging-budget/src/ceiling.txt"),
            "0usize",
        )
        .unwrap();
        let v = run(ws.path(), true);
        assert_eq!(v.exit_code(), 1, "{v:?}");
        assert!(
            matches!(v, Verdict::OverBudget(ref w) if w.contains("1 > 0")),
            "{v:?}"
        );
        // Lowered but still above the count → clean.
        std::fs::write(
            ws.path().join("tools/staging-budget/src/ceiling.txt"),
            "5usize",
        )
        .unwrap();
        assert_eq!(run(ws.path(), true).exit_code(), 0);
    }

    #[test]
    fn a_workspace_with_only_a_ceiling_and_no_production_trees_cannot_vouch() {
        // Has `tools/staging-budget/` (so the ceiling is readable) but NO packages/ and no Rust
        // file anywhere: the vacuous-zero guard must fire, not report clean with total 0.
        let leaf = std::env::temp_dir().join(format!("sb-unit-hollow-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&leaf);
        let _g = TempRoot(leaf.clone());
        std::fs::create_dir_all(leaf.join("tools/staging-budget/src")).unwrap();
        std::fs::write(leaf.join("tools/staging-budget/src/ceiling.txt"), "10usize").unwrap();
        let v = run(&leaf, false);
        assert_eq!(v.exit_code(), 2, "{v:?}");
        assert!(
            matches!(v, Verdict::CannotVouch(ref w) if w.contains("refusing a vacuous zero")),
            "{v:?}"
        );
    }

    /// Parse `src` and return the six bucket counts as a tuple in printing order.
    fn counts(src: &str) -> (usize, usize, usize, usize, usize, usize) {
        let file = syn::parse_file(src).expect("test source must parse");
        let (c, _) = count_file(&file, "pkg/src/lib.rs", src);
        (
            c.error_from,
            c.error_message,
            c.plain_str,
            c.plain_format,
            c.new_str,
            c.new_format,
        )
    }

    fn prod(src: &str) -> usize {
        let (a, b, c, d, e, f) = counts(src);
        a + b + c + d + e + f
    }

    const MESSAGE_CALL: &str = r#"fn f(x: String) -> Result<(), Error> { Err(Error::Message(x)) }"#;

    #[test]
    fn each_shape_lands_in_its_own_bucket() {
        // The six buckets must be asserted INDIVIDUALLY: a `total()` that silently drops a
        // bucket would otherwise pass every sum-based test below.
        assert_eq!(
            counts(MESSAGE_CALL),
            (0, 1, 0, 0, 0, 0),
            "Error::Message(x)"
        );
        assert_eq!(
            counts(r#"fn f() -> Result<(), Error> { Err(Error::from("x")) }"#),
            (1, 0, 0, 0, 0, 0),
            "Error::from"
        );
        assert_eq!(
            counts(r#"fn f() -> Result<(), FrontError> { Err(FrontError::plain("x")) }"#),
            (0, 0, 1, 0, 0, 0),
            "plain literal"
        );
        assert_eq!(
            counts(r#"fn f() -> Result<(), FrontError> { Err(FrontError::plain(format!("x"))) }"#),
            (0, 0, 0, 1, 0, 0),
            "plain format"
        );
        assert_eq!(
            counts(r#"fn f() -> Result<(), FrontError> { Err(FrontError::new(0, 0, "x")) }"#),
            (0, 0, 0, 0, 1, 0),
            "new literal"
        );
        assert_eq!(
            counts(
                r#"fn f() -> Result<(), FrontError> { Err(FrontError::new(0, 0, format!("x"))) }"#
            ),
            (0, 0, 0, 0, 0, 1),
            "new format"
        );
        // And `total()` must be the sum of exactly these six. Every bucket must be NON-ZERO in
        // this probe, otherwise a `total()` that silently drops a zero bucket is undetectable
        // (a two-shape probe here once let `+ self.new_format` be deleted with all tests green).
        let all = counts(&format!(
            "{MESSAGE_CALL}\n{}\n{}\n{}\n{}\n{}",
            r#"fn g() -> Result<(), Error> { Err(Error::from("x")) }"#,
            r#"fn h() -> Result<(), FrontError> { Err(FrontError::plain("x")) }"#,
            r#"fn i() -> Result<(), FrontError> { Err(FrontError::plain(format!("x"))) }"#,
            r#"fn j() -> Result<(), FrontError> { Err(FrontError::new(0, 0, "x")) }"#,
            r#"fn k() -> Result<(), FrontError> { Err(FrontError::new(0, 0, format!("x"))) }"#,
        ));
        let c = Counts {
            error_from: all.0,
            error_message: all.1,
            plain_str: all.2,
            plain_format: all.3,
            new_str: all.4,
            new_format: all.5,
        };
        assert_eq!(
            (
                c.error_from,
                c.error_message,
                c.plain_str,
                c.plain_format,
                c.new_str,
                c.new_format
            ),
            (1, 1, 1, 1, 1, 1)
        );
        assert_eq!(c.total(), 6);
        // The `Error::` / `FrontError::` qualifier in `path_ends_with` is load-bearing: a
        // `Foo::Message(..)` / `Bar::from(..)` / `Baz::plain(..)` on an unrelated type must
        // not be counted.
        assert_eq!(
            prod(r#"fn q(x: String) -> Result<(), Foo> { Err(Foo::Message(x)) }"#),
            0,
            "unrelated Message"
        );
        assert_eq!(
            prod(r#"fn q() -> Result<(), Foo> { Err(Bar::from("x")) }"#),
            0,
            "unrelated from"
        );
        assert_eq!(
            prod(r#"fn q() -> Result<(), Baz> { Err(Baz::plain("x")) }"#),
            0,
            "unrelated plain"
        );
        assert_eq!(
            prod(r#"fn q() -> Result<(), Baz> { Err(Baz::new(0, 0, "x")) }"#),
            0,
            "unrelated new"
        );
        // `Self::X` inside an impl of the right type IS the staging funnel.
        assert_eq!(
            prod("impl Error { fn m(x: String) -> Result<(), Error> { Err(Self::Message(x)) } }"),
            1,
            "Self::Message"
        );
        assert_eq!(
            prod(r#"impl Error { fn m() -> Result<(), Error> { Err(Self::from("x")) } }"#),
            1,
            "Self::from"
        );
        assert_eq!(
            prod(
                r#"impl FrontError { fn m() -> Result<(), FrontError> { Err(Self::plain("x")) } }"#
            ),
            1,
            "Self::plain"
        );
        assert_eq!(
            prod(
                r#"impl FrontError { fn m() -> Result<(), FrontError> { Err(Self::new(0, 0, "x")) } }"#
            ),
            1,
            "Self::new"
        );
    }

    #[test]
    fn every_counted_shape_is_counted_exactly_once() {
        assert_eq!(prod(MESSAGE_CALL), 1, "Error::Message(x)");
        assert_eq!(
            prod(r#"fn f() -> Result<(), Error> { Err(Error :: Message("x".into())) }"#),
            1,
            "spaced path"
        );
        assert_eq!(
            prod("fn f(x: String) -> Result<(), Error> { Err(Error::Message(\n    x\n)) }"),
            1,
            "multi-line"
        );
        assert_eq!(
            prod(r#"fn f() -> Result<(), Error> { Err(Error::from("x")) }"#),
            1,
            "Error::from"
        );
        assert_eq!(
            prod(r#"fn f() -> Result<(), FrontError> { Err(FrontError::new(0, 0, "x")) }"#),
            1,
            "new literal"
        );
        assert_eq!(
            prod(
                r#"fn f() -> Result<(), FrontError> { Err(FrontError::new(0, 0, format!("x"))) }"#
            ),
            1,
            "new format"
        );
        assert_eq!(
            prod(r#"fn f() -> Result<(), FrontError> { Err(FrontError::plain("x")) }"#),
            1,
            "plain literal"
        );
        assert_eq!(
            prod(r#"fn f() -> Result<(), FrontError> { Err(FrontError::plain(format!("x"))) }"#),
            1,
            "plain format"
        );
        assert_eq!(
            prod("fn f() -> Result<(), FrontError> { Err(FrontError::plain(\n    \"x\"\n)) }"),
            1,
            "multi-line plain"
        );
        // A trailing comment containing `=>` must not exempt a construction.
        assert_eq!(
            prod(
                r#"fn f(x: String) { let _ = Error::Message(x); // maps => code
}"#
            ),
            1
        );
    }

    #[test]
    fn patterns_are_never_constructions() {
        assert_eq!(
            prod(r#"fn f(e: &Error) -> bool { matches!(e, Error::Message(m) if m.is_empty()) }"#),
            0
        );
        assert_eq!(prod("fn f(e: &Error) -> bool { matches!(\n    e,\n    Error::Message(ref m)\n        if m.len() > 0\n) }"), 0);
        assert_eq!(
            prod(
                r#"fn f(e: &Error) -> bool { match e { Error::Message(_) => true, _ => false } }"#
            ),
            0
        );
        assert_eq!(
            prod(
                r#"fn f(e: &Error) -> bool { if let Error::Message(_) = e { true } else { false } }"#
            ),
            0
        );
        assert_eq!(
            prod(
                r#"fn f(e: &Error) -> bool { let Error::Message(_) = e else { return false }; true }"#
            ),
            0
        );
    }

    #[test]
    fn test_only_items_are_skipped_and_production_is_counted() {
        let skip = |attrs: &str, item: &str| prod(&format!("{attrs}\n{item}"));
        // Skipped: compiled only under test.
        assert_eq!(
            skip(
                "#[cfg(test)]",
                "mod m { fn t(x: String) -> Result<(), Error> { Err(Error::Message(x)) } }"
            ),
            0,
            "cfg(test) mod"
        );
        assert_eq!(
            skip(
                "#[cfg(test)]",
                "fn t(x: String) -> Result<(), Error> { Err(Error::Message(x)) }"
            ),
            0,
            "cfg(test) fn"
        );
        assert_eq!(
            skip(
                "#[test]",
                "fn t() { let _ = Error::Message(String::new()); }"
            ),
            0,
            "#[test] fn"
        );
        assert_eq!(
            skip(
                "#[cfg(all(test, feature = \"std\"))]",
                "mod m { fn t(x: String) -> Result<(), Error> { Err(Error::Message(x)) } }"
            ),
            0,
            "all(test, ..)"
        );
        assert_eq!(skip("#[cfg(test)]", "impl S { fn t(&self) -> Result<(), Error> { Err(Error::Message(String::new())) } }"), 0, "cfg(test) impl");
        assert_eq!(
            skip(
                "#[cfg(test)]",
                "const C: fn() -> Result<(), Error> = || Err(Error::Message(String::new()));"
            ),
            0,
            "cfg(test) const"
        );
        assert_eq!(
            skip(
                "#[cfg(test)]",
                "static S2: fn() -> Result<(), Error> = || Err(Error::Message(String::new()));"
            ),
            0,
            "cfg(test) static"
        );
        assert_eq!(
            prod("impl S { #[test] fn t() { let _ = Error::Message(String::new()); } }"),
            0,
            "#[test] fn in impl"
        );
        assert_eq!(
            prod("impl S { #[cfg(test)] fn t() { let _ = Error::Message(String::new()); } }"),
            0,
            "cfg(test) fn in impl"
        );
        assert_eq!(
            prod("trait T { #[test] fn t() { let _ = Error::Message(String::new()); } }"),
            0,
            "#[test] fn in trait"
        );
        assert_eq!(
            prod("trait T { #[cfg(test)] fn t() { let _ = Error::Message(String::new()); } }"),
            0,
            "cfg(test) fn in trait"
        );
        // A trait method WITHOUT a test gate is production and must be counted.
        assert_eq!(
            prod("trait T { fn t() { let _ = Error::Message(String::new()); } }"),
            1,
            "plain trait method is production"
        );
        // `#[test]` on a non-function is NOT a gate at the VISITOR level either (rustc rejects
        // such code, but the gate must not treat it as test-only): pin the visitor, not just
        // the `is_cfg_test` helper.
        assert_eq!(
            prod(
                "#[test] mod m { fn t(x: String) -> Result<(), Error> { Err(Error::Message(x)) } }"
            ),
            1,
            "#[test] on a mod is not a gate"
        );
        assert_eq!(
            prod("#[test] const C: fn() -> Result<(), Error> = || Err(Error::Message(String::new()));"),
            1,
            "#[test] on a const is not a gate"
        );
        // Counted: production.
        assert_eq!(
            skip(
                "#[cfg(not(test))]",
                "mod m { fn t(x: String) -> Result<(), Error> { Err(Error::Message(x)) } }"
            ),
            1,
            "not(test) is production"
        );
        assert_eq!(
            skip(
                "#[cfg(feature = \"latest\")]",
                "mod m { fn t(x: String) -> Result<(), Error> { Err(Error::Message(x)) } }"
            ),
            1,
            "feature gate is production"
        );
        assert_eq!(
            skip(
                "#[cfg_attr(test, allow(dead_code))]",
                "mod m { fn t(x: String) -> Result<(), Error> { Err(Error::Message(x)) } }"
            ),
            1,
            "cfg_attr alone is not a gate"
        );
        assert_eq!(
            skip(
                "",
                "mod m { fn t(x: String) -> Result<(), Error> { Err(Error::Message(x)) } }"
            ),
            1,
            "plain mod"
        );
        assert_eq!(prod("impl S { fn t(&self) -> Result<(), Error> { Err(Error::Message(String::new())) } }"), 1, "plain impl");
        // `any(std, test)` also compiles in production → counted.
        assert_eq!(
            skip(
                "#[cfg(any(std, test))]",
                "mod m { fn t(x: String) -> Result<(), Error> { Err(Error::Message(x)) } }"
            ),
            1,
            "any(std, test) is production-reachable"
        );
        // A cfg(test) `use` line must not swallow what follows it.
        assert_eq!(prod("#[cfg(test)] use std::prelude::v1::*;\nfn f(x: String) -> Result<(), Error> { Err(Error::Message(x)) }"), 1);
    }

    #[test]
    fn cfg_predicate_entailment_table() {
        let t = |src: &str| {
            syn::parse_str::<syn::File>(src).map(|f| match &f.items[0] {
                Item::Mod(m) => is_cfg_test(&m.attrs),
                _ => panic!("test source must be a mod"),
            })
        };
        let yes = [
            "#[cfg(test)] mod m {}",
            "#[cfg(all(test))] mod m {}",
            "#[cfg(all(test, unix))] mod m {}",
            "#[cfg(all(test, feature = \"std\"))] mod m {}",
            "#[cfg(any(test, test))] mod m {}",
            "#[cfg(all(unix, all(test, feature = \"x\")))] mod m {}",
            "#[cfg(test, unix)] mod m {}",
        ];
        let no = [
            "#[cfg(any(std, test))] mod m {}",
            "#[cfg(any(unix, test))] mod m {}",
            "#[cfg(not(test))] mod m {}",
            "#[cfg(all(not(test), std))] mod m {}",
            "#[cfg(feature = \"latest\")] mod m {}",
            "#[cfg(feature = \"test-util\")] mod m {}",
            "#[cfg(target_os = \"test\")] mod m {}",
            "#[cfg(unix)] mod m {}",
            "#[cfg_attr(test, allow(dead_code))] mod m {}",
            "mod m {}",
            // rustc rejects `#[test]` on a mod; the gate must not treat it as a cfg gate.
            "#[test] mod m {}",
        ];
        for src in yes {
            assert!(t(src).expect("parse"), "must be a test gate: {src}");
        }
        for src in no {
            assert!(!t(src).expect("parse"), "must NOT be a test gate: {src}");
        }
    }

    #[test]
    fn parse_ceiling_accepts_the_documented_spellings_and_rejects_garbage() {
        assert_eq!(parse_ceiling("175usize"), Some(175));
        assert_eq!(parse_ceiling("175"), Some(175));
        assert_eq!(parse_ceiling("175usize;"), Some(175));
        assert_eq!(parse_ceiling("  175usize \n"), Some(175));
        assert_eq!(parse_ceiling("175usize // ceiling"), Some(175));
        assert_eq!(parse_ceiling(""), None);
        assert_eq!(parse_ceiling("usize"), None);
        assert_eq!(parse_ceiling("+175"), None);
        assert_eq!(parse_ceiling("-5usize"), None);
    }

    /// Removes the directory when dropped — even if an assertion panics.
    struct TempRoot(std::path::PathBuf);
    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn exclusion_is_a_path_component_relative_to_the_root() {
        // The scan root sits under a parent NAMED `tests` / `target` / `fixtures` on purpose.
        // What this pins: the exclusion is keyed on path COMPONENTS relative to the scan root,
        // so (a) a package's own `target/`/`tests/`/`fixtures/` dirs are skipped, (b) a file
        // merely NAMED `target_tests.rs` is NOT skipped, and (c) the parent above the root is
        // irrelevant. A naive undelimited substring implementation (`contains("tests")`) drops
        // the decoy and fails here; a delimiter-bounded substring on the root-relative path is
        // behaviourally equivalent and is not what this test claims to distinguish. The parent
        // lives INSIDE a unique leaf (never a new top-level dir in the shared tmp), and the
        // guard removes the whole leaf on exit — including on assertion failure.
        for parent in ["tests", "target", "fixtures"] {
            let leaf = std::env::temp_dir().join(format!("sb-unit-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&leaf);
            let root = leaf.join(parent);
            let _guard = TempRoot(leaf.clone());
            for d in ["pkg/target/x/src", "pkg/tests", "pkg/fixtures", "pkg/src"] {
                std::fs::create_dir_all(root.join(d)).unwrap();
            }
            std::fs::write(root.join("pkg/target/x/src/lib.rs"), MESSAGE_CALL).unwrap();
            std::fs::write(root.join("pkg/tests/t.rs"), MESSAGE_CALL).unwrap();
            std::fs::write(root.join("pkg/fixtures/f.rs"), MESSAGE_CALL).unwrap();
            std::fs::write(root.join("pkg/src/lib.rs"), MESSAGE_CALL).unwrap();
            std::fs::write(root.join("pkg/src/target_tests.rs"), MESSAGE_CALL).unwrap(); // filename ≠ component
            let files = rust_files(&root);
            let names: Vec<String> = files
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
                .collect();
            assert_eq!(
                files.len(),
                2,
                "under `{parent}`: exactly the two production files: {names:?}"
            );
            assert!(names.contains(&"lib.rs".to_string()), "{names:?}");
            assert!(names.contains(&"target_tests.rs".to_string()), "{names:?}");
        }
    }

    /// Six sites, one per bucket, at known lines, followed by a test-only site that must not count.
    const SIX_SITES: &str = r#"
fn a(x: String) -> Result<(), Error> { Err(Error::Message(x)) }
fn b() -> Result<(), Error> { Err(Error::from("x")) }
fn c() -> Result<(), FrontError> { Err(FrontError::plain("x")) }
fn d(s: &str) -> Result<(), FrontError> { Err(FrontError::plain(format!("{s}"))) }
fn e() -> Result<(), FrontError> { Err(FrontError::new(1, 2, "x")) }
fn f(s: &str) -> Result<(), FrontError> { Err(FrontError::new(1, 2, format!("{s}"))) }
#[cfg(test)]
mod t { fn g() -> Result<(), Error> { Err(Error::from("test only")) } }
"#;

    #[test]
    fn list_reports_every_counted_site_with_relative_path_line_and_bucket() {
        let ws = Workspace::new("10usize", SIX_SITES);
        let s = scan(ws.path()).unwrap();
        let rendered: Vec<String> = s.sites.iter().map(Site::render).collect();
        assert_eq!(
            rendered,
            vec![
                "packages/p/src/lib.rs:2  error_message  fn a(x: String) -> Result<(), Error> { Err(Error::Message(x)) }",
                "packages/p/src/lib.rs:3  error_from     fn b() -> Result<(), Error> { Err(Error::from(\"x\")) }",
                "packages/p/src/lib.rs:4  plain_str      fn c() -> Result<(), FrontError> { Err(FrontError::plain(\"x\")) }",
                "packages/p/src/lib.rs:5  plain_format   fn d(s: &str) -> Result<(), FrontError> { Err(FrontError::plain(format!(\"{s}\"))) }",
                "packages/p/src/lib.rs:6  new_str        fn e() -> Result<(), FrontError> { Err(FrontError::new(1, 2, \"x\")) }",
                "packages/p/src/lib.rs:7  new_format     fn f(s: &str) -> Result<(), FrontError> { Err(FrontError::new(1, 2, format!(\"{s}\"))) }",
            ]
        );
    }

    #[test]
    fn list_length_always_equals_the_counted_total() {
        // A test-gated site and an i18n.rs file are both walked past; neither may appear.
        let ws = Workspace::new("10usize", SIX_SITES);
        std::fs::write(
            ws.path().join("packages/p/src/i18n.rs"),
            "fn z() -> Result<(), Error> { Err(Error::from(\"catalogue\")) }",
        )
        .unwrap();
        let s = scan(ws.path()).unwrap();
        assert_eq!(s.counts.total(), 6);
        assert_eq!(s.sites.len(), s.counts.total());
        assert!(s.sites.iter().all(|x| !x.path.ends_with("i18n.rs")));
        assert!(s.sites.iter().all(|x| !x.source.contains("test only")));
        // The verdict is computed from the same walk, so the totals agree.
        let Verdict::Clean { counts, .. } = run(ws.path(), false) else {
            panic!("within budget must be clean")
        };
        assert_eq!(counts, s.counts);
    }

    /// One site in each of the three scan roots plus a second file in `packages/`, so the
    /// expected order exercises the cross-root `files.sort()` (`examples/` sorts before
    /// `packages/` although it is walked after it) and the in-file source order.
    #[test]
    fn list_is_sorted_by_path_then_line_across_files_and_roots() {
        let ws = Workspace::new("10usize", ONE_SITE);
        std::fs::create_dir_all(ws.path().join("packages/q/src")).unwrap();
        std::fs::write(
            ws.path().join("packages/q/src/lib.rs"),
            "fn q() -> Result<(), Error> { Err(Error::from(\"x\")) }\nfn r() -> Result<(), Error> { Err(Error::from(\"y\")) }",
        )
        .unwrap();
        std::fs::create_dir_all(ws.path().join("examples/x/src")).unwrap();
        std::fs::write(ws.path().join("examples/x/src/lib.rs"), ONE_SITE).unwrap();
        std::fs::create_dir_all(ws.path().join("tools/t/src")).unwrap();
        std::fs::write(ws.path().join("tools/t/src/lib.rs"), ONE_SITE).unwrap();
        let s = scan(ws.path()).unwrap();
        assert_eq!(s.counts.total(), 5);
        let keys: Vec<(String, usize)> = s.sites.iter().map(|x| (x.path.clone(), x.line)).collect();
        assert_eq!(
            keys,
            vec![
                ("examples/x/src/lib.rs".to_string(), 1),
                ("packages/p/src/lib.rs".to_string(), 1),
                ("packages/q/src/lib.rs".to_string(), 1),
                ("packages/q/src/lib.rs".to_string(), 2),
                ("tools/t/src/lib.rs".to_string(), 1),
            ]
        );
    }

    #[test]
    fn scan_reports_the_same_cannot_vouch_reason_as_run() {
        let ws = Workspace::new("10usize", ONE_SITE);
        std::fs::remove_dir_all(ws.path().join("packages")).unwrap();
        let why = scan(ws.path()).unwrap_err();
        assert!(
            matches!(run(ws.path(), false), Verdict::CannotVouch(ref w) if *w == why),
            "{why}"
        );
    }

    #[test]
    fn list_uses_the_expression_line_not_the_enclosing_item() {
        // The construction sits on the third line of a four-line item: the site must say 4.
        let ws = Workspace::new(
            "10usize",
            "fn w()\n    -> Result<(), Error>\n{\n    Error::from(\"multi-line item\")\n}\n",
        );
        let s = scan(ws.path()).unwrap();
        assert_eq!(s.sites.len(), 1);
        assert_eq!(s.sites[0].line, 4);
        assert_eq!(s.sites[0].source, "Error::from(\"multi-line item\")");
    }

    /// The `--list` CLI path (stdout of the binary) is pinned through `cli`, so a mutation that
    /// empties or drops the list — invisible to every `scan()`-level test — goes red here.
    /// A mistyped flag is a usage error, never a silent no-op: `--against-gi` must not leave
    /// the monotonicity gate off, and a stray positional (e.g. the program name, were `main()`
    /// ever to stop skipping argv[0]) must not be swallowed.
    #[test]
    fn cli_rejects_unknown_arguments_with_exit_two_and_no_output() {
        let ws = Workspace::new("10usize", SIX_SITES);
        for bad in ["--lst", "--against-gi", "--root", "/some/path", "list"] {
            let mut out: Vec<u8> = Vec::new();
            let code = cli(&[bad.to_string()], ws.path(), false, &mut out);
            assert_eq!(code, 2, "{bad:?}");
            assert!(out.is_empty(), "{bad:?}: {}", String::from_utf8_lossy(&out));
        }
        // The value of `--root` is consumed, not treated as a flag.
        let mut out: Vec<u8> = Vec::new();
        assert_eq!(
            cli(
                &[
                    "--root".to_string(),
                    ws.path().to_string_lossy().into_owned()
                ],
                std::path::Path::new("/"),
                false,
                &mut out
            ),
            0
        );
    }

    #[test]
    fn cli_list_prints_every_site_then_the_summary_with_exit_zero() {
        let ws = Workspace::new("10usize", SIX_SITES);
        let mut out: Vec<u8> = Vec::new();
        let code = cli(&["--list".to_string()], ws.path(), false, &mut out);
        assert_eq!(code, 0);
        let text = String::from_utf8(out).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        // 6 site lines + 8 summary lines, sites first and in `(path, line)` order.
        assert_eq!(lines.len(), 6 + 8, "{text}");
        let sites: Vec<&str> = lines.iter().take(6).copied().collect();
        assert!(
            sites
                .iter()
                .all(|l| l.starts_with("packages/p/src/lib.rs:")),
            "{sites:?}"
        );
        assert_eq!(sites[0], "packages/p/src/lib.rs:2  error_message  fn a(x: String) -> Result<(), Error> { Err(Error::Message(x)) }");
        assert_eq!(sites[5], "packages/p/src/lib.rs:7  new_format     fn f(s: &str) -> Result<(), FrontError> { Err(FrontError::new(1, 2, format!(\"{s}\"))) }");
        assert_eq!(lines[6], "error_from     1");
        assert_eq!(lines[12], "total          6  (ceiling 10)");
        assert_eq!(lines[13], "staging-budget: clean");
        assert!(text.ends_with("staging-budget: clean\n"));
    }

    #[test]
    fn cli_without_list_prints_only_the_summary_and_list_is_printed_even_when_over_budget() {
        let ws = Workspace::new("10usize", SIX_SITES);
        let mut out: Vec<u8> = Vec::new();
        assert_eq!(cli(&[], ws.path(), false, &mut out), 0);
        let text = String::from_utf8(out).unwrap();
        assert_eq!(
            text.lines().count(),
            8,
            "no site lines without --list:\n{text}"
        );
        // Over budget: the list is still printed (that is when it is wanted), exit 1.
        let ws = Workspace::new("2usize", SIX_SITES);
        let mut out: Vec<u8> = Vec::new();
        assert_eq!(cli(&["--list".to_string()], ws.path(), false, &mut out), 1);
        let text = String::from_utf8(out).unwrap();
        assert_eq!(
            text.lines().count(),
            6,
            "sites only, no summary on failure:\n{text}"
        );
        // `--root` selects the repository; a missing value is a usage error.
        let ws = Workspace::new("10usize", SIX_SITES);
        let mut out: Vec<u8> = Vec::new();
        assert_eq!(cli(&["--root".to_string()], ws.path(), false, &mut out), 2);
        let mut out: Vec<u8> = Vec::new();
        assert_eq!(
            cli(
                &[
                    "--root".to_string(),
                    ws.path().to_string_lossy().into_owned(),
                    "--list".to_string()
                ],
                std::path::Path::new("/"),
                false,
                &mut out
            ),
            0
        );
        assert_eq!(String::from_utf8(out).unwrap().lines().count(), 6 + 8);
    }
}
