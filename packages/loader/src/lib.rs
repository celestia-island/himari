//! `himari-loader`: resolves `use "relative/path"` into **one** formula set.
//!
//! The front ends (`himari-latex` / `himari-matlab`) are pure string APIs and never touch the
//! file system; cross-file reuse lives at this layer: read file → parse → recursively parse the
//! referenced files → merge.
//!
//! **The dialect is never guessed**: matex by default; for LaTeX one must write
//! `%! dialect: latex` in the file's directive block.
//! There is no AUTO fallback — `si * x` may be legal in both dialects but means different
//! things, and the parser has no right to choose on the author's behalf.
//!
//! Merge semantics (deliberately simple and predictable):
//!   1. The decls/items of a referenced file come **first** (federation relies on order: a later
//!      formula can reference an earlier one);
//!   2. A decl with the same name is **deduplicated** when its dimension/unit agrees, and is an
//!      error otherwise (no silently picking one);
//!   3. An item with the same name is always an error ("later write wins" is a disaster in a
//!      formula library, and cannot be explained);
//!   4. A cyclic reference is an error that names the files on the cycle; there is no "partial
//!      expansion".

mod sha256;

pub use sha256::hex_digest;

use std::path::{Path, PathBuf};

/// Scan the `%! key: value` directive block at the start of the file (before the first
/// non-comment line).
fn directive(src: &str, key: &str) -> Option<String> {
    for line in src.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if !t.starts_with('%') {
            break; // the body begins, so the directive block is over
        }
        let rest = t.strip_prefix('%')?.trim_start_matches('!').trim();
        if let Some(kv) = rest.strip_prefix(key) {
            if let Some(v) = kv.trim_start_matches([':', '=']).split_whitespace().next() {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// The library version declared in the file (`%! version: 1.2.0`). Purely for humans — the
/// semantic guarantee comes from the content hash.
fn declared_version(src: &str) -> Option<String> {
    directive(src, "version")
}

/// The optional dialect directive in a file: `%! dialect: latex` / `%! dialect: matex`.
///
/// Why it is needed: AUTO's "matex first + restricted fallback" already blocked the worst case,
/// but pure-ASCII LaTeX (`x = sigma * f`) is still **legal** on the matex surface — so it gets
/// read as matex (whether `sigma` is one name or s·i·g·m·a depends on the dialect). That is a grey
/// area where "both dialects make sense", and guessing is no solution: write one directive line
/// and the semantics are pinned down.
///
/// Position: only directives in comments **before the first non-comment line** are recognized
/// (so a comment inside the formula body cannot trigger them by accident).
/// An unrecognized value is an error (no silent fallback to AUTO).
fn dialect_directive(src: &str) -> Result<Option<&'static str>, String> {
    for line in src.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if !t.starts_with('%') {
            break; // the body has begun, so the directive block is over
        }
        // `%! dialect: latex` (compact forms such as `%!dialect=latex` are allowed)
        let Some(rest) = t.strip_prefix('%') else {
            break;
        };
        let rest = rest.trim_start_matches('!').trim();
        let Some(kv) = rest.strip_prefix("dialect") else {
            continue;
        };
        let Some(value) = kv.trim_start_matches([':', '=']).split_whitespace().next() else {
            return Err("`%! dialect:` must be followed by `latex` or `matex`".to_string());
        };
        return match value.to_ascii_lowercase().as_str() {
            "latex" | "tex" => Ok(Some("latex")),
            "matex" | "matlab" => Ok(Some("matex")),
            other => Err(format!(
                "unknown dialect `{other}`: only `latex` / `matex` are supported (write `%! dialect: latex`)"
            )),
        };
    }
    Ok(None)
}

/// Typed twin of [`parse_source`]: keeps the [`himari::FrontError`] so callers can render it
/// in their own locale (or read its stable code). [`parse_source`] is the rendered wrapper.
pub fn parse_source_typed(
    src: &str,
    origin: &str,
) -> Result<himari::FormulaSet, himari::FrontError> {
    // The directive strings are plain diagnostics, so they enter the catalogue as the typed
    // `SyntaxError` payload (never laundered through `plain` into a `Message`).
    match dialect_directive(src)
        .map_err(|e| himari::FrontError::plain(himari::Error::SyntaxError(e)))?
    {
        Some("latex") => himari_latex::parse(src).map_err(|mut e| {
            e.msg = e.render_with_source(origin, src);
            e
        }),
        Some("matex") | None => himari_matlab::parse(src).map_err(|mut e| {
            let is_syntax = e.code.code() == "syntax-error";
            e.msg = e.render_with_source(origin, src);
            // Syntax error + LaTeX commands in the source: most likely LaTeX was intended.
            // Offer the way out, but **do not guess on their behalf**. (The check runs on the
            // stable code, so it works in every locale.)
            if is_syntax && src.contains('\\') {
                e.msg.push_str(
                    "\n提示：源里有 LaTeX 命令而当前按 matex 解析。若这是 LaTeX，\
                     请在文件首行写 `%! dialect: latex` 显式声明。",
                );
            }
            e
        }),
        Some(_) => unreachable!("dialect_directive 只产出 latex / matex"),
    }
}

pub fn parse_source(src: &str, origin: &str) -> Result<himari::FormulaSet, String> {
    match parse_source_typed(src, origin) {
        Ok(set) => Ok(set),
        Err(e) => Err(e.msg),
    }
}

/// Parse entry point: read one `.formula` file and inline every library it `use`s.
pub fn load(path: impl AsRef<Path>) -> Result<himari::FormulaSet, String> {
    let path = path.as_ref();
    let mut stack: Vec<PathBuf> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut prints: Fingerprints = Vec::new();
    let root_dir = canonical_dir(path);
    let set = load_inner(path, &mut stack, &mut seen, &mut prints, &root_dir)?;
    // Verify if there is a lock: a changed library file must speak up **at load time**, not wait
    // until something downstream computes a wrong value
    let lock_path = lock_path_for(path);
    if let Some(lock) = read_lock(&lock_path)? {
        verify_lock(&lock, &prints, &lock_path)?;
    }
    Ok(set)
}

/// The **canonical** path of the directory containing the entry file (`strip_prefix` requires
/// both sides to be canonical, otherwise a relative-path entry degrades into writing absolute
/// paths into the lock).
fn canonical_dir(entry: &Path) -> PathBuf {
    // For a bare file name (`himari-fmt --lock entry.formula`) `parent()` is the empty path "",
    // and `canonicalize("")` fails and falls back to the relative ".", which makes the later
    // `strip_prefix` mismatch and writes **absolute paths** into the lock (incomparable across
    // directories/machines: the same lock errors out as soon as it is spelled differently).
    let dir = match entry.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };
    dir.canonicalize().unwrap_or(dir)
}

/// A relative path (computed lexically): the relative representation from `from_dir` to `to`,
/// **`../` allowed**.
///
/// The lock must hold relative paths (comparable across machines). `strip_prefix` can only handle
/// "to is under from", so an out-of-tree reference such as `use "../common/lib.formula"` would
/// mismatch — and falling back to an **absolute path** there would tie the lock to the current
/// machine. Here the number of `..` is computed from the common prefix.
fn rel_path(from_dir: &Path, to: &Path) -> String {
    let (Some(from), Ok(to)) = (from_dir.canonicalize().ok(), to.canonicalize()) else {
        return to.display().to_string().replace('\\', "/");
    };
    let fc: Vec<_> = from.components().collect();
    let tc: Vec<_> = to.components().collect();
    let mut i = 0;
    while i < fc.len() && i < tc.len() && fc[i] == tc[i] {
        i += 1;
    }
    let mut parts: Vec<String> = Vec::new();
    for _ in i..fc.len() {
        parts.push("..".to_string());
    }
    for c in &tc[i..] {
        parts.push(c.as_os_str().to_string_lossy().to_string());
    }
    let joined = parts.join("/");
    if joined.is_empty() {
        ".".to_string()
    } else {
        joined
    }
}

/// Lock-file path convention: `<entry>.lock` (next to the entry file).
pub fn lock_path_for(entry: &Path) -> PathBuf {
    let mut s = entry.as_os_str().to_os_string();
    s.push(".lock");
    PathBuf::from(s)
}

/// Generate the lock-file content (deterministic: sorted by path, plain text, git-readable).
///
/// Format (**TAB-separated**, the path is the last column so it may contain spaces):
/// `sha256\t<hash>\t<version or ->\t<relative path>`
pub fn lock_text(entry: impl AsRef<Path>) -> Result<String, String> {
    let entry = entry.as_ref();
    let mut stack: Vec<PathBuf> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut prints: Fingerprints = Vec::new();
    let root_dir = canonical_dir(entry);
    load_inner(entry, &mut stack, &mut seen, &mut prints, &root_dir)?;
    prints.sort();
    let mut out = String::new();
    out.push_str("# himari 公式库锁 —— 由 `himari-fmt --lock` 生成，勿手改。\n");
    out.push_str("# 格式：sha256<TAB><内容哈希><TAB><声明版本或 -><TAB><相对路径>\n");
    for (rel, hash, ver) in prints {
        // The lock format splits columns with TAB and keeps the path last: a TAB or a **trailing
        // space** in a path makes it impossible to parse unambiguously (the lock we wrote could not
        // even be read back). This is **explicitly refused** here instead of generating an artifact
        // that `--lock-check` fails on immediately.
        if rel.contains('\t') || rel.ends_with(' ') || rel.contains('\n') {
            return Err(format!(
                "文件名 `{rel}` 含 TAB / 换行 / 尾随空格：锁格式无法无歧义表示它 —— \
                 请重命名该文件（空格可以在中间）"
            ));
        }
        out.push_str(&format!(
            "sha256\t{hash}\t{}\t{rel}\n",
            ver.unwrap_or_else(|| "-".to_string())
        ));
    }
    Ok(out)
}

/// One lock record.
struct LockEntry {
    hash: String,
    version: Option<String>,
    path: String,
}

/// Parse a lock file. **Every non-empty non-comment line must be valid** — a hand-broken lock
/// must fail hard, and the line must not be "skipped as if unlocked": deleting one line would then
/// let a modified protected library pass anyway.
fn parse_lock(lock: &str, lock_path: &Path) -> Result<Vec<LockEntry>, String> {
    let mut out = Vec::new();
    for (ln, line) in lock.lines().enumerate() {
        let t = line.trim_end_matches(['\r', '\n']);
        let t = t.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let bad = |why: &str| {
            format!(
                "{}: 第 {} 行不是合法的锁记录（{why}）：\n  {t}\n\
                 锁文件由 `himari-fmt --lock <入口>` 生成，请重新生成而不是手改。",
                lock_path.display(),
                ln + 1
            )
        };
        // TAB takes precedence; locks from an earlier space-separated era are still accepted (back
        // then paths could not contain spaces)
        let parts: Vec<&str> = if t.contains('\t') {
            t.split('\t').map(|x| x.trim_end()).collect()
        } else {
            t.split_whitespace().collect()
        };
        if parts.len() < 3 {
            return Err(bad("字段不足：需要 sha256 / 哈希 / [版本] / 路径"));
        }
        if parts[0] != "sha256" {
            return Err(bad("首列必须是 `sha256`"));
        }
        let hash = parts[1].to_string();
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(bad("哈希不是 64 位十六进制"));
        }
        // TAB format: sha256 / hash / version / path; space format: sha256 / hash / path
        // [version]
        let (version, path) = if t.contains('\t') {
            if parts.len() < 4 {
                return Err(bad("字段不足：需要 sha256 / 哈希 / 版本 / 路径"));
            }
            let v = parts[2];
            (
                if v == "-" { None } else { Some(v.to_string()) },
                parts[3].to_string(),
            )
        } else if parts.len() >= 4 {
            (Some(parts[3].to_string()), parts[2].to_string())
        } else {
            (None, parts[2].to_string())
        };
        if path.is_empty() {
            return Err(bad("路径为空"));
        }
        out.push(LockEntry {
            hash,
            version,
            path,
        });
    }
    Ok(out)
}

/// Read the lock file. **Missing = not locked** (Ok(None)); present but unreadable (not UTF-8,
/// insufficient permissions, it is a directory) is always a **hard error** — otherwise treating
/// those as "not locked" would silently switch the protection off, which is the same class of
/// requirement as "a bad line must fail hard".
fn read_lock(lock_path: &Path) -> Result<Option<String>, String> {
    match std::fs::metadata(lock_path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(himari::Error::IoError(format!(
            "cannot read lock file {}: {e}",
            lock_path.display()
        ))
        .to_string()),
        // Only **regular files** are accepted: reading a directory fails, while a FIFO / device
        // file makes `std::fs::read` **block forever** in open(2) (which would hang both build.rs
        // and CI).
        Ok(md) if !md.is_file() => Err(format!(
            "锁路径 {} 不是普通文件（目录/FIFO/设备）：删掉它，\
             或用 `himari-fmt --lock <入口>` 重新生成",
            lock_path.display()
        )),
        Ok(_) => match std::fs::read(lock_path) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(t) => Ok(Some(t)),
                Err(_) => Err(format!(
                    "锁文件 {} 不是 UTF-8（可能被编辑器改成了 UTF-16）：\
                     删掉它并用 `himari-fmt --lock <入口>` 重新生成",
                    lock_path.display()
                )),
            },
            Err(e) => Err(himari::Error::IoError(format!(
                "cannot read lock file {}: {e}",
                lock_path.display()
            ))
            .to_string()),
        },
    }
}

/// Verify without writing: compare when a lock exists, `Ok(())` when there is none (opt-in).
/// Once a lock exists it is a **whitelist**: one extra file, one missing file, changed content or
/// a corrupted line all count as a mismatch.
/// Intended for hygiene gates — a stale lock must surface in CI/gates instead of waiting for
/// someone to load the file.
pub fn check_lock(entry: impl AsRef<Path>) -> Result<(), String> {
    let entry = entry.as_ref();
    let lock_path = lock_path_for(entry);
    let Some(lock) = read_lock(&lock_path)? else {
        return Ok(());
    };
    let mut stack: Vec<PathBuf> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut prints: Fingerprints = Vec::new();
    let root_dir = canonical_dir(entry);
    load_inner(entry, &mut stack, &mut seen, &mut prints, &root_dir)?;
    verify_lock(&lock, &prints, &lock_path)
}

/// Verify the lock against the current file fingerprints. Any changed file content is an error
/// (naming which file).
fn verify_lock(lock: &str, prints: &Fingerprints, lock_path: &Path) -> Result<(), String> {
    let locked = parse_lock(lock, lock_path)?;
    // A 0-byte lock = unlocked (explicitly recording "not locked yet"); but as long as the file
    // has content and not a single record can be parsed, `parse_lock` has already reported it as a
    // broken lock.
    if locked.is_empty() {
        return Ok(());
    }
    let mut now: std::collections::BTreeMap<&str, (&str, Option<&str>)> =
        std::collections::BTreeMap::new();
    for (rel, hash, ver) in prints {
        now.insert(rel.as_str(), (hash.as_str(), ver.as_deref()));
    }
    let mut bad: Vec<String> = Vec::new();
    for e in &locked {
        match now.get(e.path.as_str()) {
            Some((h, v)) => {
                if *h != e.hash.as_str() {
                    let vtxt = match (e.version.as_deref(), *v) {
                        (Some(a), Some(b)) if a != b => format!("（声明版本 {a} → {b}）"),
                        _ => String::new(),
                    };
                    bad.push(format!("内容变了{vtxt}：{}", e.path));
                }
            }
            None => bad.push(format!("锁里有、实际没被引用：{}", e.path)),
        }
    }
    // The lock is a **whitelist**: a file actually referenced but absent from the lock is also a
    // mismatch.
    // It used to be only a hint — so "delete one line from the lock" let a modified protected
    // library pass anyway.
    let mut fresh: Vec<String> = Vec::new();
    for rel in now.keys() {
        if !locked.iter().any(|e| e.path == *rel) {
            fresh.push((*rel).to_string());
        }
    }
    if !fresh.is_empty() {
        bad.push(format!(
            "锁里没有这些文件（新增引用，或锁被改过）：{}",
            fresh.join("、")
        ));
    }
    if !bad.is_empty() {
        // Bounded errors: a stale lock may have tens of thousands of entries, and listing them all
        // both floods the screen and nobody reads it
        const SHOW: usize = 8;
        let mut listed = bad
            .iter()
            .take(SHOW)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  ");
        if bad.len() > SHOW {
            listed.push_str(&format!("\n  …… 另有 {} 条不一致", bad.len() - SHOW));
        }
        let msg = format!(
            "公式库与锁文件不一致（{}）：\n  {listed}\n\
             复核改动后运行 `himari-fmt --lock <入口>` 重新生成锁。",
            lock_path.display()
        );
        return Err(msg);
    }
    Ok(())
}

/// File fingerprints collected while loading (relative to the entry file's directory, so they
/// can be compared across machines).
type Fingerprints = Vec<(String, String, Option<String>)>;

fn load_inner(
    path: &Path,
    stack: &mut Vec<PathBuf>,
    seen: &mut Vec<PathBuf>,
    prints: &mut Fingerprints,
    root_dir: &Path,
) -> Result<himari::FormulaSet, String> {
    let canonical = path.canonicalize().map_err(|e| {
        himari::Error::IoError(format!("cannot read {}: {e}", path.display())).to_string()
    })?;
    if stack.contains(&canonical) {
        let chain: Vec<String> = stack
            .iter()
            .skip_while(|p| **p != canonical)
            .map(|p| p.display().to_string())
            .chain(std::iter::once(canonical.display().to_string()))
            .collect();
        return Err(format!(
            "公式库有环引用：{}\n  （环引用不做部分展开：改成单向依赖）",
            chain.join(" → ")
        ));
    }
    // The same library referenced twice through two paths is normal: inline it only once
    if seen.contains(&canonical) {
        return Ok(himari::FormulaSet::default());
    }

    let src = std::fs::read_to_string(&canonical).map_err(|e| {
        // Catalogued `io-error`; the String API renders it (English template), the typed API
        // keeps the variant.
        himari::Error::IoError(format!("cannot read {}: {e}", canonical.display())).to_string()
    })?;
    let origin = canonical.display().to_string();
    // Front-end priority: **matex first**. The canonical format is matex, whereas LaTeX reads a
    // multi-letter name such as `sigma * f` as s·i·g·m·a·f (syntactically legal, semantically
    // entirely wrong), so "LaTeX first, then MATLAB" would silently produce wrong values on matex
    // files.
    let own = parse_source(&src, &origin)?;
    // Record the fingerprint: content hash (comparable across machines) + the library version
    // declared in the file (readability)
    let rel = rel_path(root_dir, &canonical);
    prints.push((rel, hex_digest(src.as_bytes()), declared_version(&src)));

    stack.push(canonical.clone());
    let mut merged = himari::FormulaSet::default();
    let base = canonical.parent().unwrap_or(Path::new(".")).to_path_buf();
    for rel in &own.uses {
        let target = base.join(rel);
        let sub = load_inner(&target, stack, seen, prints, root_dir)?;
        merge(&mut merged, sub)?;
    }
    stack.pop();
    seen.push(canonical);

    merge(&mut merged, own)?;
    Ok(merged)
}

/// Merge `src` into `dst` (name conflicts are described in the module docs).
fn merge(dst: &mut himari::FormulaSet, src: himari::FormulaSet) -> Result<(), String> {
    for d in src.decls {
        if let Some(existing) = dst.decls.iter().find(|x| x.name == d.name) {
            if existing != &d {
                return Err(format!(
                    "同名声明冲突：`{}` 在两处写法不同（{} vs {}）—— 公式库里必须一致",
                    d.name, existing.quantity, d.quantity
                ));
            }
            continue; // exactly identical: deduplicate
        }
        dst.decls.push(d);
    }
    for item in src.items {
        if let Some(name) = &item.name {
            if dst.items.iter().any(|x| x.name.as_ref() == Some(name)) {
                return Err(format!(
                    "同名公式冲突：`{name}` 在两处都有 —— 公式库不允许同名覆盖"
                ));
            }
        }
        dst.items.push(item);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("himari-loader-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        p
    }

    #[test]
    fn includes_are_prepended_in_order() {
        let dir = tmpdir("inc");
        // Library: one dimensionless gain (a library must stay reusable by callers with other
        // dimensions)
        write(
            &dir,
            "lib/base.formula",
            "decl {\n    gain : gain [1];\n}\nlib_g = gain * 2\n",
        );
        let main = write(
            &dir,
            "main.formula",
            "use \"lib/base.formula\"\ndecl {\n    F : force [N];\n    k : stiffness [N/m];\n}\n\
             my_x = lib_g * F / k\n",
        );
        let set = load(&main).unwrap();
        // The referenced file comes first: decls order = gain, F, k; items order = lib_g, my_x
        assert_eq!(
            set.decls
                .iter()
                .map(|d| d.name.as_str())
                .collect::<Vec<_>>(),
            vec!["gain", "F", "k"]
        );
        assert_eq!(
            set.items
                .iter()
                .map(|i| i.name.clone().unwrap())
                .collect::<Vec<_>>(),
            vec!["lib_g", "my_x"]
        );
        // Federation inlining: the later formula's reference to the earlier one is really replaced
        let items = himari::federate_inline(&set).unwrap();
        assert_eq!(items.len(), 2);
        // Dimension: my_x = (gain*2)·F/k = m
        let dims = himari::dims_of_set(&set).unwrap();
        assert_eq!(dims["my_x"], himari::parse_unit("m").unwrap());
    }

    #[test]
    fn duplicate_identical_decls_are_deduped_but_conflicts_are_rejected() {
        let dir = tmpdir("dup");
        write(
            &dir,
            "a.formula",
            "decl {\n    k : stiff [N/m];\n}\na_x = k\n",
        );
        write(
            &dir,
            "b.formula",
            "decl {\n    k : stiff [N/m];\n}\nb_x = k\n",
        );
        let main = write(
            &dir,
            "main.formula",
            "use \"a.formula\"\nuse \"b.formula\"\ndecl {\n    k : stiff [N/m];\n}\nm_x = k\n",
        );
        let set = load(&main).unwrap();
        assert_eq!(set.decls.len(), 1, "一致的声明应去重");

        // Different spelling → error
        write(
            &dir,
            "c.formula",
            "decl {\n    k : other [m];\n}\nc_x = k\n",
        );
        let bad = write(
            &dir,
            "bad.formula",
            "use \"a.formula\"\nuse \"c.formula\"\nz = k\n",
        );
        let err = load(&bad).unwrap_err();
        assert!(err.contains("同名声明冲突"), "{err}");
    }

    #[test]
    fn duplicate_item_names_are_rejected() {
        let dir = tmpdir("item");
        write(&dir, "a.formula", "decl {\n    k : d;\n}\nsame = k\n");
        let main = write(
            &dir,
            "main.formula",
            "use \"a.formula\"\ndecl {\n    k : d;\n}\nsame = k * 2\n",
        );
        let err = load(&main).unwrap_err();
        assert!(err.contains("同名公式冲突"), "{err}");
    }

    #[test]
    fn cycles_are_reported_with_the_chain() {
        let dir = tmpdir("cycle");
        write(
            &dir,
            "a.formula",
            "use \"b.formula\"\ndecl {\n    a : d;\n}\nfa = a\n",
        );
        write(
            &dir,
            "b.formula",
            "use \"a.formula\"\ndecl {\n    b : d;\n}\nfb = b\n",
        );
        let err = load(dir.join("a.formula")).unwrap_err();
        assert!(err.contains("环引用"), "{err}");
        assert!(err.contains("a.formula"), "{err}");
    }

    #[test]
    fn matex_is_the_default_and_the_dialect_is_never_guessed() {
        // The file-lane counterpart of "the macro is the dialect": matex by default, **no
        // guessing**.
        // This pins down a real silent wrong value — for `si * x` with `si` declared:
        //   · matex reads it as si·x        → si=100, x=2  → 200
        //   · LaTeX reads it as s·i·x       → s=3, i=5, x=2 → 30
        // The old AUTO fallback (when matex syntax failed and the source contained `\`, try
        // LaTeX) always gave 200 and never reported an error, because what it "guessed" was only
        // which parser succeeded first, not which one the author meant to write.
        let body = "decl {\n    s : d;\n    i : d;\n    x : d;\n    si : d;\n}\nq = si * x\n";
        // No directive = matex: si is one name. Evaluation uses matex semantics.
        let set = parse_source(body, "<t>").expect("无指令时按 matex 解析");
        let expr = set.items[0].expr.clone();
        let env: std::collections::HashMap<String, f64> =
            [("s", 3.0), ("i", 5.0), ("x", 2.0), ("si", 100.0)]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect();
        assert_eq!(
            himari::eval(&expr, &env).unwrap(),
            200.0,
            "默认 matex：si·x"
        );
        // Explicit LaTeX: si = s·i. Same file, same inputs, a different value — decided by the
        // **declaration**, not by guessing.
        let tex = format!("%! dialect: latex\n{body}");
        let set = parse_source(&tex, "<t>").expect("显式 latex 应走 LaTeX 前端");
        let expr = set.items[0].expr.clone();
        assert_eq!(
            himari::eval(&expr, &env).unwrap(),
            30.0,
            "显式 LaTeX：s·i·x"
        );
        // A pure matex spelling is synonymous in both dialects (there is no ambiguity), and it
        // also works under an explicit latex directive
        let plain = "decl {\n    a : d;\n    b : d;\n}\nq = a / b\n";
        assert!(parse_source(plain, "<t>").is_ok());
        assert!(parse_source(&format!("%! dialect: latex\n{plain}"), "<t>").is_ok());
    }

    #[test]
    fn latex_source_without_a_directive_fails_loudly_with_an_escape_hatch() {
        // A LaTeX-only spelling without a dialect declaration → parsed as the default matex →
        // **error**, never a silently wrong value.
        // The error must offer the way out (write `%! dialect: latex`), but **do not guess on
        // their behalf**.
        let src = "decl {\n    a : d;\n    b : d;\n}\nq = \\frac{a}{b}\n";
        let err = parse_source(src, "<t>").unwrap_err();
        // The hint rides on the catalogued syntax-error code; the rendered sentence is English
        // through this (String) API, and the escape hatch must still be offered.
        assert!(err.contains("syntax error"), "matex 读不了 \\frac：{err}");
        assert!(
            err.contains("%! dialect: latex"),
            "必须给出 LaTeX 的出口：{err}"
        );
        // The reverse: after an explicit declaration the same source works
        assert!(parse_source(&format!("%! dialect: latex\n{src}"), "<t>").is_ok());
        // A semantic rejection (unknown function) still reports the matex error — this case never
        // relied on the fallback.
        // Note the argument must be a **non-bare** expression: `unknown_name(bare_symbol)` is read
        // by matex as an indexed symbol (the tensor atom a(i)), which is legal, not an error.
        let unknown = "decl {\n    x : d;\n}\nq = expp(x * 2)\n";
        let err = parse_source(unknown, "<t>").unwrap_err();
        // The error is the catalogued `unknown-function`; assert the code-bearing rendering, not
        // a sentence (the wording is per-locale and the catalogues render in English here).
        assert!(err.contains("unknown function"), "{err}");
        assert!(!err.contains("dialect"), "语义拒绝不该提示换方言：{err}");
    }

    #[test]
    fn explicit_dialect_directive_removes_the_ambiguity() {
        // This line is **legal in both dialects but semantically different**:
        //   matex: `sigma` is one identifier → sigma·f
        //   LaTeX: is `sigma` the name of \sigma? No — in pure ASCII LaTeX also reads it as a
        //          multi-letter name; it is things like `\frac` that need LaTeX. The real grey area
        //          is "pure ASCII carrying LaTeX markers".
        // `si` is plainer: under matex it is a single name, under LaTeX it is s·i multiplied.
        let body = "decl {\n    si : d;\n    x : d;\n}\nq = si * x\n";
        // No directive: matex semantics (the AUTO default)
        let auto = parse_source(body, "<t>").unwrap();
        assert_eq!(auto.items.len(), 1);
        // Explicit matex: the same
        let mex = format!("%! dialect: matex\n{body}");
        assert_eq!(parse_source(&mex, "<t>").unwrap().items.len(), 1);
        // Explicit latex: `si` is read as s·i (the parts that only make sense with \\ are left to
        // LaTeX)
        let tex = format!("%! dialect: latex\n{body}");
        let set = parse_source(&tex, "<t>").expect("显式 latex 指令应走 LaTeX 前端");

        // The two dialects give **different trees** for the same line — which is exactly why the
        // directive exists
        let a = himari::key(&auto.items[0].expr);
        let b = himari::key(&set.items[0].expr);
        assert_ne!(a, b, "matex 与 latex 应给出不同的树（否则这个测试没意义）");

        // An unknown dialect name must be an error, with no silent fallback to AUTO
        let bad = format!("%! dialect: fortran\n{body}");
        let err = parse_source(&bad, "<t>").unwrap_err();
        assert!(err.contains("unknown dialect"), "{err}");
        // Directives are recognized only in the comment region: a `%!` in the body must not change
        // the dialect
        let inline = format!("{body}%! dialect: latex\n");
        assert!(parse_source(&inline, "<t>").is_ok());
    }

    #[test]
    fn content_lock_detects_library_changes() {
        let dir = tmpdir("lock");
        write(&dir, "lib.formula", "decl {\n    k : d;\n}\nk2 = k * 2\n");
        let main = write(
            &dir,
            "main.formula",
            "use \"lib.formula\"\ndecl {\n    m : d;\n}\nq = m * 3\n",
        );
        // Nothing is verified without a lock (opt-in; existing repos are unaffected)
        assert!(load(&main).is_ok());
        // Generate the lock: deterministic, covers both files, paths relative to the entry,
        // readable
        let text = lock_text(&main).unwrap();
        assert!(text.contains("sha256"), "{text}");
        assert!(text.contains("main.formula"), "{text}");
        assert!(text.contains("lib.formula"), "{text}");
        assert!(!text.contains("/sandbox/"), "路径必须相对：{text}");
        assert_eq!(text, lock_text(&main).unwrap(), "锁内容必须确定性");
        std::fs::write(lock_path_for(&main), &text).unwrap();
        // Content matches → passes
        assert!(load(&main).is_ok(), "锁与内容一致时应通过");
        // Change the library → hard failure at load time, naming which file
        write(&dir, "lib.formula", "decl {\n    k : d;\n}\nk2 = k * 3\n");
        let err = load(&main).unwrap_err();
        assert!(err.contains("内容变了"), "{err}");
        assert!(err.contains("lib.formula"), "{err}");
        assert!(err.contains("--lock"), "应给出修复指令：{err}");
        // Relocking restores it
        std::fs::write(lock_path_for(&main), lock_text(&main).unwrap()).unwrap();
        assert!(load(&main).is_ok(), "重新上锁后应通过");
        // The version declaration goes into the lock (the column for humans)
        write(
            &dir,
            "lib.formula",
            "%! version: 1.4.2\ndecl {\n    k : d;\n}\nk2 = k * 4\n",
        );
        let text = lock_text(&main).unwrap();
        assert!(text.contains("1.4.2"), "声明版本应入锁：{text}");
        // The version changed but the content did not → not an error (the semantic guarantee comes
        // from the hash)
        std::fs::write(lock_path_for(&main), &text).unwrap();
        write(
            &dir,
            "lib.formula",
            "%! version: 2.0.0\ndecl {\n    k : d;\n}\nk2 = k * 4\n",
        );
        let err = load(&main).unwrap_err();
        assert!(
            err.contains("声明版本 1.4.2 → 2.0.0"),
            "版本变化要说清：{err}"
        );
    }

    #[test]
    fn lock_paths_are_relative_and_robust() {
        let dir = tmpdir("lockrel");
        // Both paths with spaces and paths with Chinese characters must be writable and readable
        write(
            &dir,
            "my lib/a b.formula",
            "decl {\n    k : d;\n}\nk2 = k * 2\n",
        );
        let main = write(
            &dir,
            "main.formula",
            "use \"my lib/a b.formula\"\ndecl {\n    m : d;\n}\nq = m\n",
        );
        let text = lock_text(&main).unwrap();
        assert!(
            text.contains("my lib/a b.formula"),
            "空格路径要完整入锁：{text}"
        );
        assert!(
            !text.contains("/tmp") && !text.contains("/mnt"),
            "不得有绝对路径：{text}"
        );
        std::fs::write(lock_path_for(&main), &text).unwrap();
        assert!(load(&main).is_ok(), "空格路径的锁必须能校验通过");
        // A bare-filename call (`--lock entry.formula`) and a call with a directory must produce the
        // same lock
        let cwd_lock = {
            let prev = std::env::current_dir().unwrap();
            std::env::set_current_dir(&dir).unwrap();
            let t = lock_text(Path::new("main.formula")).unwrap();
            std::env::set_current_dir(prev).unwrap();
            t
        };
        assert_eq!(text, cwd_lock, "裸文件名与带目录的写法必须一致");
        // An out-of-tree `../` reference: the lock still holds a relative path (machine-portable)
        let sub = dir.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        write(&dir, "common.formula", "decl {\n    c : d;\n}\nc2 = c\n");
        let deep = write(
            &sub,
            "deep.formula",
            "use \"../common.formula\"\ndecl {\n    d : d;\n}\nq = d\n",
        );
        let t2 = lock_text(&deep).unwrap();
        assert!(
            t2.contains("../common.formula"),
            "越界引用要写 ../ 形式：{t2}"
        );
        assert!(!t2.contains("/tmp"), "越界引用也不得落绝对路径：{t2}");
    }

    #[test]
    fn corrupted_lock_lines_fail_closed() {
        let dir = tmpdir("lockbad");
        write(&dir, "lib.formula", "decl {\n    k : d;\n}\nk2 = k * 2\n");
        let main = write(
            &dir,
            "main.formula",
            "use \"lib.formula\"\ndecl {\n    m : d;\n}\nq = m\n",
        );
        let good = lock_text(&main).unwrap();
        std::fs::write(lock_path_for(&main), &good).unwrap();
        assert!(load(&main).is_ok());
        // Change the library content (to prove that "broken lock + changed library" must not pass)
        write(&dir, "lib.formula", "decl {\n    k : d;\n}\nk2 = k * 3\n");
        for (what, broken) in [
            ("删掉一条记录", {
                let mut lines: Vec<&str> = good.lines().collect();
                lines.retain(|l| !l.contains("lib.formula"));
                lines.join("\n")
            }),
            // The truncated hash is constructed separately below (a self-replacing `replace` is
            // flagged by clippy as a no-op)
            ("关键字写错", good.replace("sha256\t", "sha256x\t")),
            ("整行乱码", format!("{good}\nthis is not a lock line\n")),
        ] {
            std::fs::write(lock_path_for(&main), &broken).unwrap();
            let err = load(&main).expect_err(&format!("坏锁（{what}）必须硬失败"));
            assert!(
                err.contains("锁") || err.contains("记录"),
                "{what} 的报错要说清是锁的问题：{err}"
            );
        }
        // Explicitly truncate the hash
        let truncated: String = good
            .lines()
            .map(|l| {
                if l.starts_with("sha256\t") {
                    let mut parts: Vec<&str> = l.split('\t').collect();
                    parts[1] = "abcd";
                    parts.join("\t")
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(lock_path_for(&main), &truncated).unwrap();
        let err = load(&main).unwrap_err();
        assert!(err.contains("64 位十六进制"), "{err}");
        // A 0-byte lock = explicitly "not locked yet", not an error
        std::fs::write(lock_path_for(&main), "").unwrap();
        assert!(load(&main).is_ok(), "空锁应视为未上锁");
    }

    #[test]
    fn unreadable_lock_paths_fail_closed() {
        // Review counterexample: treating a failed lock read as "not locked" = silently switching
        // the protection off.
        let dir = tmpdir("lockread");
        write(&dir, "lib.formula", "decl {\n    k : d;\n}\nk2 = k * 2\n");
        let main = write(
            &dir,
            "main.formula",
            "use \"lib.formula\"\ndecl {\n    m : d;\n}\nq = m\n",
        );
        let lock = lock_path_for(&main);
        // ① Not UTF-8: hard failure
        std::fs::write(&lock, [0xffu8, 0xfe, 0x00, 0x41]).unwrap();
        let err = check_lock(&main).unwrap_err();
        assert!(err.contains("UTF-8"), "{err}");
        // ② The lock path is a directory: hard failure (and it must not block)
        std::fs::remove_file(&lock).ok();
        std::fs::create_dir_all(&lock).unwrap();
        let err = check_lock(&main).unwrap_err();
        assert!(
            err.contains("不是普通文件") || err.contains("目录"),
            "{err}"
        );
        std::fs::remove_dir_all(&lock).unwrap();
        // ③ Missing: Ok (explicitly "not locked yet")
        assert!(check_lock(&main).is_ok());
        // ④ A normal lock: OK; after changing the content: failure
        std::fs::write(&lock, lock_text(&main).unwrap()).unwrap();
        assert!(check_lock(&main).is_ok());
        write(&dir, "lib.formula", "decl {\n    k : d;\n}\nk2 = k * 3\n");
        assert!(check_lock(&main).is_err());
    }

    #[test]
    fn lock_refuses_filenames_it_cannot_represent() {
        // The lock format splits columns with TAB and keeps the path last: a TAB / newline /
        // trailing space cannot be represented unambiguously. It must be **refused** instead of
        // generating an artifact that `--lock-check` fails on immediately.
        let dir = tmpdir("lockname");
        let main = write(
            &dir,
            "main.formula",
            "use \"lib\twith\ttab.formula\"\ndecl {\n    m : d;\n}\nq = m\n",
        );
        write(
            &dir,
            "lib\twith\ttab.formula",
            "decl {\n    k : d;\n}\nk2 = k\n",
        );
        let err = lock_text(&main).unwrap_err();
        assert!(err.contains("TAB"), "{err}");
        assert!(err.contains("重命名"), "{err}");
    }

    #[test]
    fn missing_include_names_the_file() {
        let dir = tmpdir("missing");
        let main = write(
            &dir,
            "main.formula",
            "use \"nope.formula\"\ndecl {\n    x : d;\n}\nx\n",
        );
        let err = load(&main).unwrap_err();
        assert!(err.contains("nope.formula"), "{err}");
    }
}
