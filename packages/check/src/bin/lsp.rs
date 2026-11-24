//! `himari-lsp`: a minimal language server for `.formula` files (stdio + JSON-RPC).
//!
//! Why it is hand-written instead of pulling in a dependency: the whole protocol surface is just
//! four methods (initialize / didOpen / didChange / completion) plus shutdown, and JSON only uses
//! objects/arrays/strings/numbers. Pulling in an LSP framework (a dependency tree of hundreds of
//! kilobytes) for that does not fit this repo's "extremely narrow dependency surface" stance.
//!
//! The protocol layer is deliberately made of **pure functions** (`handle`: state + method +
//! params → response), so unit tests can cover it directly; `main` only handles Content-Length
//! framing and moving bytes between stdin/stdout.
//!
//! Capabilities offered:
//!   - **diagnostics**: computed on open/change (parse + dimension + shape), with line/column and
//!     caret text in the errors;
//!   - **completion**: the file's decl names + formula names + the unary function table;
//!   - **hover**: done by the VS Code extension through the sidecar (not reimplemented here).
//!
//! How to run: `cargo run -p himari-check --bin himari-lsp` (the editor launches it over stdio).

use std::collections::HashMap;
use std::io::{BufRead, Read, Write};

// ---------------------------------------------------------------- minimal JSON

#[derive(Clone, Debug, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
    fn as_num(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }
    fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }
    /// Serialization (only objects/arrays/strings/numbers/bool/null are used).
    fn dump(&self) -> String {
        match self {
            Json::Null => "null".to_string(),
            Json::Bool(b) => b.to_string(),
            Json::Num(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{n:.0}")
                } else {
                    format!("{n}")
                }
            }
            Json::Str(s) => {
                let mut out = String::from("\"");
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                        c => out.push(c),
                    }
                }
                out.push('"');
                out
            }
            Json::Arr(items) => {
                let body: Vec<String> = items.iter().map(Json::dump).collect();
                format!("[{}]", body.join(","))
            }
            Json::Obj(kv) => {
                let body: Vec<String> = kv
                    .iter()
                    .map(|(k, v)| format!("{}:{}", Json::Str(k.clone()).dump(), v.dump()))
                    .collect();
                format!("{{{}}}", body.join(","))
            }
        }
    }
}

fn parse_json(src: &str) -> Option<Json> {
    let bytes: Vec<char> = src.chars().collect();
    let mut i = 0usize;
    let v = parse_value(&bytes, &mut i)?;
    skip_ws(&bytes, &mut i);
    if i == bytes.len() {
        Some(v)
    } else {
        None
    }
}

fn skip_ws(b: &[char], i: &mut usize) {
    while *i < b.len() && matches!(b[*i], ' ' | '\t' | '\r' | '\n') {
        *i += 1;
    }
}

fn parse_value(b: &[char], i: &mut usize) -> Option<Json> {
    parse_value_d(b, i, 0)
}

/// Nesting depth limit: malicious/corrupted input must not be able to blow up the server's stack
/// (measured once at 300k levels → SIGABRT).
const JSON_MAX_DEPTH: usize = 128;

fn parse_value_d(b: &[char], i: &mut usize, depth: usize) -> Option<Json> {
    if depth > JSON_MAX_DEPTH {
        return None;
    }
    skip_ws(b, i);
    match *b.get(*i)? {
        '{' => {
            *i += 1;
            let mut kv = Vec::new();
            skip_ws(b, i);
            if *b.get(*i)? == '}' {
                *i += 1;
                return Some(Json::Obj(kv));
            }
            loop {
                skip_ws(b, i);
                let k = match parse_json_string(b, i)? {
                    Json::Str(s) => s,
                    _ => return None,
                };
                skip_ws(b, i);
                if *b.get(*i)? != ':' {
                    return None;
                }
                *i += 1;
                let v = parse_value_d(b, i, depth + 1)?;
                kv.push((k, v));
                skip_ws(b, i);
                match *b.get(*i)? {
                    ',' => *i += 1,
                    '}' => {
                        *i += 1;
                        return Some(Json::Obj(kv));
                    }
                    _ => return None,
                }
            }
        }
        '[' => {
            *i += 1;
            let mut arr = Vec::new();
            skip_ws(b, i);
            if *b.get(*i)? == ']' {
                *i += 1;
                return Some(Json::Arr(arr));
            }
            loop {
                arr.push(parse_value_d(b, i, depth + 1)?);
                skip_ws(b, i);
                match *b.get(*i)? {
                    ',' => *i += 1,
                    ']' => {
                        *i += 1;
                        return Some(Json::Arr(arr));
                    }
                    _ => return None,
                }
            }
        }
        '"' => parse_json_string(b, i),
        't' => {
            for c in "true".chars() {
                if *b.get(*i)? != c {
                    return None;
                }
                *i += 1;
            }
            Some(Json::Bool(true))
        }
        'f' => {
            for c in "false".chars() {
                if *b.get(*i)? != c {
                    return None;
                }
                *i += 1;
            }
            Some(Json::Bool(false))
        }
        'n' => {
            for c in "null".chars() {
                if *b.get(*i)? != c {
                    return None;
                }
                *i += 1;
            }
            Some(Json::Null)
        }
        _ => {
            let start = *i;
            while *i < b.len() && matches!(b[*i], '-' | '+' | '.' | 'e' | 'E' | '0'..='9') {
                *i += 1;
            }
            if start == *i {
                return None;
            }
            let s: String = b[start..*i].iter().collect();
            s.parse::<f64>().ok().map(Json::Num)
        }
    }
}

fn parse_json_string(b: &[char], i: &mut usize) -> Option<Json> {
    if *b.get(*i)? != '"' {
        return None;
    }
    *i += 1;
    let mut out = String::new();
    loop {
        let c = *b.get(*i)?;
        *i += 1;
        match c {
            '"' => return Some(Json::Str(out)),
            '\\' => {
                let e = *b.get(*i)?;
                *i += 1;
                match e {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'u' => {
                        let hex: String = b.get(*i..*i + 4)?.iter().collect();
                        *i += 4;
                        let n = u32::from_str_radix(&hex, 16).ok()?;
                        match char::from_u32(n) {
                            Some(c) => out.push(c),
                            // High surrogate (0xD800..0xDBFF): wait for a following low surrogate to
                            // compose one code point.
                            // Previously the whole message was dropped with `?` — a request carrying
                            // an id would then never be answered.
                            None if (0xD800..0xDC00).contains(&n) => {
                                // High surrogate: check whether `\u` + a low surrogate follows, and
                                // compose one code point
                                let pair = if *b.get(*i)? == '\\' && *b.get(*i + 1)? == 'u' {
                                    let hex2: String = b.get(*i + 2..*i + 6)?.iter().collect();
                                    u32::from_str_radix(&hex2, 16)
                                        .ok()
                                        .filter(|n2| (0xDC00..0xE000).contains(n2))
                                } else {
                                    None
                                };
                                match pair {
                                    Some(n2) => {
                                        *i += 6;
                                        let cp = 0x10000 + ((n - 0xD800) << 10) + (n2 - 0xDC00);
                                        out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                                    }
                                    // Lone high surrogate: replacement character (do not drop the
                                    // whole message)
                                    None => out.push('\u{FFFD}'),
                                }
                            }
                            // Lone low surrogate: replacement character (do not drop the message
                            // either)
                            _ => out.push('\u{FFFD}'),
                        }
                    }
                    _ => return None,
                }
            }
            c => out.push(c),
        }
    }
}

// ---------------------------------------------------------------- diagnostics

/// One diagnostic: `(start line, start column, end line, end column, message)`, 0-based rows and
/// columns (the LSP convention).
type Diag = (usize, usize, usize, usize, String);

/// Run full diagnostics over a (not yet saved) source text: parse + dimension + shape.
/// One quick-fix: replace `[start, end)` with `new_text` (rows/columns are 0-based).
///
/// Columns are counted in **chars**, not UTF-16 code units: the existing diagnostic ranges in this
/// repo are computed that way too (`end_col` of `diagnose_text`), keeping things internally
/// consistent. In pure-ASCII source the two are the same, and non-BMP characters (emoji and the
/// like) do not appear in formulas.
struct Fix {
    title: String,
    start: (usize, usize),
    end: (usize, usize),
    new_text: String,
    preferred: bool,
}

/// Apply one `contentChanges` entry: with a `range` it is an incremental replacement, without one
/// it is a full replacement.
///
/// A position beyond the current buffer is **clamped** instead of discarded: the editor's range may
/// be based on an older buffer, and dropping one change would leave the local state permanently
/// diverged from the editor (worse than clamping). Clamping only affects the position accuracy of
/// this one diagnostic; the next didChange realigns.
fn apply_change(text: &str, change: &Json) -> String {
    let Some(new_text) = change.get("text").and_then(Json::as_str) else {
        return text.to_string();
    };
    let Some(range) = change.get("range") else {
        return new_text.to_string();
    };
    let pos = |which: &str| -> (usize, usize) {
        let p = range.get(which);
        let line = p
            .and_then(|x| x.get("line"))
            .and_then(Json::as_num)
            .unwrap_or(0.0f64)
            .max(0.0f64) as usize;
        let ch = p
            .and_then(|x| x.get("character"))
            .and_then(Json::as_num)
            .unwrap_or(0.0f64)
            .max(0.0f64) as usize;
        (line, ch)
    };
    /// (line, UTF-16 column) → byte offset.
    ///
    /// LSP's `character` is a count of **UTF-16 code units**, not of chars: counting chars would
    /// misplace edits on lines containing non-BMP characters (emoji etc.) — measured to drop an
    /// edit at the end of the file and silently swallow the second half.
    /// Out-of-range positions clamp to **the end of this line** (not the end of the document):
    /// clamping to the document end would let a single out-of-range edit move content to the tail
    /// of the file, which is harder to notice than discarding it.
    fn offset_of(text: &str, line: usize, ch: usize) -> usize {
        let (mut l, mut c) = (0usize, 0usize);
        let mut line_end: Option<usize> = None;
        for (off, chr) in text.char_indices() {
            if l == line && c == ch {
                return off;
            }
            if l == line && line_end.is_none() && chr == '\n' {
                line_end = Some(off);
            }
            if chr == '\n' {
                l += 1;
                c = 0;
            } else if l == line {
                c += chr.len_utf16();
            }
        }
        // Ran out: if the target line exists, use its line end; otherwise use the document end
        if l >= line {
            line_end.unwrap_or(text.len())
        } else {
            text.len()
        }
    }
    let (sl, sc) = pos("start");
    let (el, ec) = pos("end");
    let start = offset_of(text, sl, sc);
    let end = offset_of(text, el, ec);
    // A reversed range (clients occasionally flip them) is treated as empty, without panicking
    let (start, end) = if end < start {
        (start, start)
    } else {
        (start, end)
    };
    let mut out = String::with_capacity(text.len() - (end - start) + new_text.len());
    out.push_str(&text[..start]);
    out.push_str(new_text);
    out.push_str(&text[end..]);
    out
}

/// Find every position (line, column) where `name` occurs as a **whole identifier** in the
/// source.
fn find_ident(src: &str, name: &str) -> Vec<(usize, usize)> {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut out = Vec::new();
    for (ln, line) in src.lines().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let want: Vec<char> = name.chars().collect();
        if want.is_empty() || chars.len() < want.len() {
            continue;
        }
        for i in 0..=(chars.len() - want.len()) {
            if chars[i..i + want.len()] != want[..] {
                continue;
            }
            let before_ok = i == 0 || !is_word(chars[i - 1]);
            let after_ok = i + want.len() == chars.len() || !is_word(chars[i + want.len()]);
            if before_ok && after_ok {
                out.push((ln, i));
            }
        }
    }
    out
}

/// Extract the declared symbol names from the source (`decl { name : ...; }` block).
fn declared_names(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(set) = himari_loader::parse_source(src, "<buffer>") {
        for d in &set.decls {
            out.push(d.name.clone());
        }
    }
    out
}

/// Derive quick-fixes from the **message** of the first compile error.
///
/// Why derive from the message: for these errors `FrontError` does not carry an exact column
/// (`check_calls` gets the IR rather than a source position), so the position is obtained by
/// "find this name in the source" — replacement fixes are offered **only when it occurs exactly
/// once**; when it occurs several times only insertion fixes are offered (no guessing).
fn quickfixes(src: &str) -> Vec<Fix> {
    let diags = diagnose_text(src);
    let Some(_first_diag) = diags.first().cloned() else {
        return Vec::new();
    };
    let mut fixes: Vec<Fix> = Vec::new();
    // Names come from the **typed** error, never from the localized sentence: quick-fixes must
    // work in all nine languages. Codes without a name payload (syntax errors, staging messages)
    // simply produce no name-based fixes.
    // Undeclared symbols surface at the `dims_of_set` stage, AFTER a successful parse — both
    // stages must feed the typed extraction.
    let typed = match himari_loader::parse_source_typed(src, "<buffer>") {
        Err(e) => Some(e),
        Ok(set) => himari::dims_of_set(&set)
            .err()
            .map(himari::FrontError::plain),
    };
    let name_of = |err: &himari::FrontError| -> Option<String> {
        match &*err.code {
            himari::Error::UndeclaredSymbol(name) => Some(name.clone()),
            himari::Error::UnknownFunction { name, .. } => Some(name.clone()),
            _ => None,
        }
    };
    let problem_name = typed.as_ref().and_then(&name_of);
    let problem_code = typed.as_ref().map(|e| e.code.code());

    // A) Undeclared symbol: insert a declaration, or correct the spelling to the nearest
    // declared symbol.
    if let Some(name) = problem_name
        .as_ref()
        .filter(|_| problem_code == Some("undeclared-symbol"))
        .cloned()
    {
        if !name.is_empty() {
            let hits = find_ident(src, &name);
            let decls = declared_names(src);
            // A1. Spelling correction (only when the name occurs exactly once)
            if hits.len() == 1 {
                let best = decls
                    .iter()
                    .map(|d| (d, name_distance(&name, d)))
                    .filter(|(_, d)| *d <= 2)
                    .min_by_key(|(_, d)| *d)
                    .map(|(d, _)| d.clone());
                if let Some(d) = best {
                    fixes.push(Fix {
                        title: format!("把 `{name}` 改成已声明的 `{d}`"),
                        start: hits[0],
                        end: (hits[0].0, hits[0].1 + name.chars().count()),
                        new_text: d,
                        preferred: true,
                    });
                }
            }
            // A2. Append one declaration line to the decl block (needs no position, always
            // available)
            if let Some((ln, col, indent)) = decl_insert_point(src) {
                // A single-line decl block (col>0) gets it inserted before `}` to stay one line; a
                // multi-line block gets it at the start of a declaration line
                let (new_text, title) = if col > 0 {
                    (
                        format!("{name} : quantity [1]; "),
                        format!("声明 `{name}`（单行 decl 块内）"),
                    )
                } else {
                    (
                        format!("{indent}{name} : quantity [1];\n"),
                        format!("声明 `{name}`：{indent}{name} : quantity [1];"),
                    )
                };
                fixes.push(Fix {
                    title,
                    start: (ln, col),
                    end: (ln, col),
                    new_text,
                    preferred: hits.len() != 1,
                });
            }
        }
    }

    // B) Unknown function: correct the spelling to the nearest name in the registry.
    if let Some(name) = problem_name
        .as_ref()
        .filter(|_| problem_code == Some("unknown-function"))
        .cloned()
    {
        if let Some(best) = himari::closest_func(&name) {
            let hits = find_ident(src, &name);
            if hits.len() == 1 {
                fixes.push(Fix {
                    title: format!("把 `{name}` 改成 `{best}`"),
                    start: hits[0],
                    end: (hits[0].0, hits[0].1 + name.chars().count()),
                    new_text: best.to_string(),
                    preferred: true,
                });
            }
        }
    }
    fixes
}

/// The insertion point of the `decl {` block: returns (the line to insert before, indentation).
fn decl_insert_point(src: &str) -> Option<(usize, usize, String)> {
    for (ln, line) in src.lines().enumerate() {
        // It must satisfy **both** "starts with decl" and "this line contains `{`": a formula may
        // be named `decl_ratio` or `decline`, and matching on the prefix alone would insert the
        // declaration line after the formula, turning a legal file into a syntax error (the
        // quick-fix would then break the user's file).
        let t = line.trim_start();
        // The criterion must be "after `decl`, **only whitespace** before `{`": checking only "the
        // line contains `{`" is fooled by LaTeX — `decline = \frac{A}{E}` contains `{` on the line,
        // so the declaration would be inserted inside `\frac{A}{E}` and a legal file would become a
        // syntax error (wrong in both dialects).
        let Some(rest) = t.strip_prefix("decl") else {
            continue;
        };
        if !rest.trim_start().starts_with('{') {
            continue;
        }
        // Single-line block: if there is already a `}` on the line → insert before it, keeping one
        // line (starting a new line would break `decl { a : d; }` apart and turn the file into a
        // syntax error)
        if let Some(pos) = line.rfind('}') {
            let col = line[..pos].chars().count();
            return Some((ln, col, " ".to_string()));
        }
        // Multi-line block: insert on the line after `decl {`, reusing the next line's indentation
        let indent = src
            .lines()
            .nth(ln + 1)
            .map(|l| {
                l.chars()
                    .take_while(|c| c.is_whitespace())
                    .collect::<String>()
            })
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "    ".to_string());
        return Some((ln + 1, 0, indent));
    }
    None
}

/// Edit distance (matching the same-named notion in core: only used for "similar spelling"
/// hints).
fn name_distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

fn diagnose_text(src: &str) -> Vec<Diag> {
    // matex first (consistent with build/loader)
    let parsed = himari_loader::parse_source_typed(src, "<buffer>");
    let set = match parsed {
        Ok(s) => s,
        Err(mut e) => {
            // The typed entry keeps the catalogue variant; render it in the caller's locale.
            e.msg = e.render_in(himari::i18n::Locale::from_env());
            let (line, col) = if e.line == 0 {
                (0, 0)
            } else {
                (e.line - 1, e.col.saturating_sub(1))
            };
            let end_col = src
                .lines()
                .nth(line)
                .map(|l| l.chars().count())
                .unwrap_or(col + 1)
                .max(col + 1);
            return vec![(line, col, line, end_col, e.msg)];
        }
    };
    let mut out: Vec<Diag> = Vec::new();
    let dims = match himari::dims_of_set(&set) {
        Ok(d) => Some(d),
        Err(e) => {
            // Diagnose in the caller's locale (HIMARI_LANG / LC_ALL / LANG).
            out.push((0, 0, 0, 1, e.render(himari::i18n::Locale::from_env())));
            None
        }
    };
    if let Some(dims) = &dims {
        let runtime_shapes = himari::runtime::shapes_of_set(&set).ok();
        for (k, item) in set.items.iter().enumerate() {
            let folded = himari::simplify(&item.expr);
            let label = item.name.clone().unwrap_or_else(|| {
                // The label is user-visible prose: render it in the caller's locale.
                himari::Error::FormulaLabel(k + 1).render(himari::i18n::Locale::from_env())
            });
            if let Some(shapes) = &runtime_shapes {
                if let Err(e) = himari::runtime::dim_check_runtime(&folded, dims, shapes) {
                    out.push((
                        item.line.saturating_sub(1),
                        0,
                        item.line.saturating_sub(1),
                        1,
                        format!("{label} {}", e.render(himari::i18n::Locale::from_env())),
                    ));
                }
                if !himari::runtime::uses_runtime_values(&folded, shapes) {
                    if let Err(e) = himari::shape_check(&folded) {
                        // Same locale rule as every other branch.
                        let text =
                            format!("{label} {}", e.render(himari::i18n::Locale::from_env()));
                        out.push((
                            item.line.saturating_sub(1),
                            0,
                            item.line.saturating_sub(1),
                            1,
                            text,
                        ));
                    }
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------- service state and the pure-function handler

#[derive(Default)]
struct State {
    /// uri → document text (maintained by didOpen/didChange; diagnostics use it instead of the
    /// disk)
    docs: HashMap<String, String>,
}

const FUNCS: &[&str] = &[
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

/// The pure-function protocol layer: given the state, the JSON-RPC method name and the params,
/// produce the JSON to write back (None when there is no response).
fn handle(state: &mut State, method: &str, params: &Json, id: Option<&Json>) -> Option<Json> {
    match method {
        "initialize" => Some(Json::Obj(vec![
            ("jsonrpc".into(), Json::Str("2.0".into())),
            (
                "result".into(),
                Json::Obj(vec![
                    (
                        "capabilities".into(),
                        Json::Obj(vec![
                            // 2 = incremental sync. Full changes (contentChanges without a range)
                            // are still **accepted**, so old clients and hand-written scripts do not
                            // break.
                            ("textDocumentSync".into(), Json::Num(2.0)),
                            // quick-fix: undeclared symbols (spelling correction / add a
                            // declaration), misspelled function names
                            ("codeActionProvider".into(), Json::Bool(true)),
                            (
                                "completionProvider".into(),
                                Json::Obj(vec![(
                                    "triggerCharacters".into(),
                                    Json::Arr(vec![
                                        Json::Str(" ".into()),
                                        Json::Str("=".into()),
                                        Json::Str("(".into()),
                                    ]),
                                )]),
                            ),
                        ]),
                    ),
                    (
                        "serverInfo".into(),
                        Json::Obj(vec![
                            ("name".into(), Json::Str("himari-lsp".into())),
                            (
                                "version".into(),
                                Json::Str(env!("CARGO_PKG_VERSION").into()),
                            ),
                        ]),
                    ),
                ]),
            ),
            ("id".into(), id.cloned().unwrap_or(Json::Null)),
        ])),
        "shutdown" => Some(Json::Obj(vec![
            ("jsonrpc".into(), Json::Str("2.0".into())),
            ("result".into(), Json::Null),
            ("id".into(), id.cloned().unwrap_or(Json::Null)),
        ])),
        "exit" => None,
        "textDocument/didOpen" | "textDocument/didChange" => {
            let uri = params
                .get("textDocument")
                .and_then(|t| t.get("uri"))
                .and_then(Json::as_str)
                .unwrap_or_default()
                .to_string();
            // didOpen carries the full text; didChange carries contentChanges (which may be an
            // incremental range or the full text — both are accepted, clients need not agree)
            let opened = params
                .get("textDocument")
                .and_then(|t| t.get("text"))
                .and_then(Json::as_str)
                .map(|s| s.to_string());
            let text = match opened {
                Some(t) => t,
                None => {
                    let mut buf = state.docs.get(&uri).cloned().unwrap_or_default();
                    if let Some(changes) = params.get("contentChanges").and_then(Json::as_arr) {
                        for ch in changes {
                            buf = apply_change(&buf, ch);
                        }
                    }
                    buf
                }
            };
            state.docs.insert(uri.clone(), text.clone());
            let diags: Vec<Json> = diagnose_text(&text)
                .into_iter()
                .map(|(l0, c0, l1, c1, msg)| {
                    Json::Obj(vec![
                        (
                            "range".into(),
                            Json::Obj(vec![
                                (
                                    "start".into(),
                                    Json::Obj(vec![
                                        ("line".into(), Json::Num(l0 as f64)),
                                        ("character".into(), Json::Num(c0 as f64)),
                                    ]),
                                ),
                                (
                                    "end".into(),
                                    Json::Obj(vec![
                                        ("line".into(), Json::Num(l1 as f64)),
                                        ("character".into(), Json::Num(c1 as f64)),
                                    ]),
                                ),
                            ]),
                        ),
                        ("severity".into(), Json::Num(1.0)), // Error
                        ("source".into(), Json::Str("himari".into())),
                        ("message".into(), Json::Str(msg)),
                    ])
                })
                .collect();
            Some(Json::Obj(vec![
                ("jsonrpc".into(), Json::Str("2.0".into())),
                (
                    "method".into(),
                    Json::Str("textDocument/publishDiagnostics".into()),
                ),
                (
                    "params".into(),
                    Json::Obj(vec![
                        ("uri".into(), Json::Str(uri)),
                        ("diagnostics".into(), Json::Arr(diags)),
                    ]),
                ),
            ]))
        }
        "textDocument/completion" => {
            let uri = params
                .get("textDocument")
                .and_then(|t| t.get("uri"))
                .and_then(Json::as_str)
                .unwrap_or_default()
                .to_string();
            let text = state.docs.get(&uri).cloned().unwrap_or_default();
            let mut names: Vec<(String, f64)> = FUNCS
                .iter()
                .map(|f| ((*f).to_string(), 3.0)) // Function
                .collect();
            // Symbols and formula names already declared in the file (Variable / Value)
            if let Ok(set) = himari_loader::parse_source(&text, "<buffer>") {
                for d in &set.decls {
                    names.push((d.name.clone(), 6.0));
                }
                for item in &set.items {
                    if let Some(n) = &item.name {
                        names.push((n.clone(), 12.0));
                    }
                }
            }
            let items: Vec<Json> = names
                .into_iter()
                .map(|(label, kind)| {
                    Json::Obj(vec![
                        ("label".into(), Json::Str(label)),
                        ("kind".into(), Json::Num(kind)),
                    ])
                })
                .collect();
            Some(Json::Obj(vec![
                ("jsonrpc".into(), Json::Str("2.0".into())),
                (
                    "result".into(),
                    Json::Obj(vec![("items".into(), Json::Arr(items))]),
                ),
                ("id".into(), id.cloned().unwrap_or(Json::Null)),
            ]))
        }
        "textDocument/codeAction" => {
            let uri = params
                .get("textDocument")
                .and_then(|t| t.get("uri"))
                .and_then(Json::as_str)
                .unwrap_or_default()
                .to_string();
            let text = state.docs.get(&uri).cloned().unwrap_or_default();
            let actions: Vec<Json> = quickfixes(&text)
                .into_iter()
                .map(|f| {
                    let range = Json::Obj(vec![
                        (
                            "start".into(),
                            Json::Obj(vec![
                                ("line".into(), Json::Num(f.start.0 as f64)),
                                ("character".into(), Json::Num(f.start.1 as f64)),
                            ]),
                        ),
                        (
                            "end".into(),
                            Json::Obj(vec![
                                ("line".into(), Json::Num(f.end.0 as f64)),
                                ("character".into(), Json::Num(f.end.1 as f64)),
                            ]),
                        ),
                    ]);
                    Json::Obj(vec![
                        ("title".into(), Json::Str(f.title)),
                        ("kind".into(), Json::Str("quickfix".into())),
                        ("isPreferred".into(), Json::Bool(f.preferred)),
                        (
                            "edit".into(),
                            Json::Obj(vec![(
                                "changes".into(),
                                Json::Obj(vec![(
                                    uri.clone(),
                                    Json::Arr(vec![Json::Obj(vec![
                                        ("range".into(), range),
                                        ("newText".into(), Json::Str(f.new_text)),
                                    ])]),
                                )]),
                            )]),
                        ),
                    ])
                })
                .collect();
            Some(Json::Obj(vec![
                ("jsonrpc".into(), Json::Str("2.0".into())),
                ("result".into(), Json::Arr(actions)),
                ("id".into(), id.cloned().unwrap_or(Json::Null)),
            ]))
        }
        // Unimplemented methods: reply with an empty result when there is an id (so the client does
        // not hang)
        _ => id.map(|id| {
            Json::Obj(vec![
                ("jsonrpc".into(), Json::Str("2.0".into())),
                ("result".into(), Json::Null),
                ("id".into(), id.clone()),
            ])
        }),
    }
}

// ---------------------------------------------------------------- stdio framing

/// Roughly scan the value of `"id"` out of (possibly corrupted) JSON text (only numbers and string
/// literals are recognized).
/// Only used for the error reply when parsing fails — it does not need to be complete.
fn scan_id(body: &str) -> Json {
    if let Some(pos) = body.find("\"id\"") {
        let rest = &body[pos + 4..];
        if let Some(cpos) = rest.find(':') {
            let tail = rest[cpos + 1..].trim_start();
            let end = tail.find(|c| [',', '}'].contains(&c)).unwrap_or(tail.len());
            let token = tail[..end].trim();
            if let Ok(n) = token.parse::<f64>() {
                return Json::Num(n);
            }
            if token.len() >= 2 && token.starts_with('"') && token.ends_with('"') {
                return Json::Str(token[1..token.len() - 1].to_string());
            }
        }
    }
    Json::Null
}

fn main() {
    let mut state = State::default();
    let stdin = std::io::stdin();
    let mut reader = std::io::BufReader::new(stdin.lock());
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    loop {
        // Header: Content-Length: N\r\n (other headers may follow and are ignored)
        let mut len: Option<usize> = None;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                return; // EOF: the editor closed
            }
            let t = line.trim_end();
            if t.is_empty() {
                break;
            }
            if let Some(v) = t.strip_prefix("Content-Length:") {
                len = v.trim().parse().ok();
            }
        }
        // Without Content-Length the body length is unknown, so frame synchronization is
        // unrecoverable → exit
        let Some(len) = len else {
            return;
        };
        let mut buf = vec![0u8; len];
        if reader.read_exact(&mut buf).is_err() {
            return;
        }
        let body = String::from_utf8_lossy(&buf);
        let Some(msg) = parse_json(&body) else {
            // Parse failure (including over-deep nesting): reply -32700 per the spec, carrying the id
            // that can be extracted, so the client does not wait for a response that never comes.
            let id = scan_id(&body);
            let payload = Json::Obj(vec![
                ("jsonrpc".into(), Json::Str("2.0".into())),
                (
                    "error".into(),
                    Json::Obj(vec![
                        ("code".into(), Json::Num(-32700.0)),
                        ("message".into(), Json::Str("JSON 解析失败（含 id）".into())),
                    ]),
                ),
                ("id".into(), id),
            ])
            .dump();
            let _ = write!(out, "Content-Length: {}\r\n\r\n{}", payload.len(), payload);
            let _ = out.flush();
            continue;
        };
        let method = msg.get("method").and_then(Json::as_str).unwrap_or_default();
        let params = msg.get("params").cloned().unwrap_or(Json::Null);
        let id = msg.get("id");
        if method == "exit" {
            return;
        }
        if let Some(resp) = handle(&mut state, method, &params, id) {
            let payload = resp.dump();
            let _ = write!(out, "Content-Length: {}\r\n\r\n{}", payload.len(), payload);
            let _ = out.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Locale-sensitive assertions pin the catalogue language instead of inheriting the host
    /// shell (a localized `HIMARI_LANG` used to flip them red).
    fn pin_locale() {
        std::env::set_var("HIMARI_LANG", "en");
    }

    fn call(state: &mut State, method: &str, params: &str) -> Option<String> {
        let p = parse_json(params).unwrap();
        handle(state, method, &p, Some(&Json::Num(1.0))).map(|j| j.dump())
    }

    #[test]
    fn json_codec_round_trips() {
        pin_locale();
        let src = r#"{"a":[1,2.5,"x\n\"y\"",true,null],"b":{"c":-0.5}}"#;
        let v = parse_json(src).unwrap();
        let again = parse_json(&v.dump()).unwrap();
        assert_eq!(v, again);
        assert_eq!(v.get("a").and_then(Json::as_arr).unwrap().len(), 5);
    }

    #[test]
    fn initialize_advertises_sync_and_completion() {
        pin_locale();
        let mut st = State::default();
        let out = call(&mut st, "initialize", "{}").unwrap();
        assert!(out.contains("textDocumentSync"));
        assert!(out.contains("completionProvider"));
        assert!(out.contains("himari-lsp"));
    }

    #[test]
    fn did_open_publishes_diagnostics() {
        pin_locale();
        let mut st = State::default();
        // An undeclared symbol → 1 diagnostic
        let bad =
            r#"{"textDocument":{"uri":"file:///a.formula","text":"decl { x : d; }\nx + y\n"}}"#;
        let out = call(&mut st, "textDocument/didOpen", bad).unwrap();
        assert!(out.contains("publishDiagnostics"), "{out}");
        assert!(out.contains("undeclared symbol"), "{out}");
        // Legal content → 0 diagnostics
        let good =
            r#"{"textDocument":{"uri":"file:///b.formula","text":"decl { x : d; }\nx * 2\n"}}"#;
        let out = call(&mut st, "textDocument/didOpen", good).unwrap();
        assert!(out.contains("\"diagnostics\":[]"), "{out}");
    }

    #[test]
    fn incremental_change_splices_ranges_correctly() {
        pin_locale();
        // Replacement within a line: `x` becomes `z` (producing an undeclared symbol)
        let base = "decl { x : d; }\nq = x * 2\n";
        let ch = parse_json(
            r#"{"text":"z","range":{"start":{"line":1,"character":4},"end":{"line":1,"character":5}}}"#,
        )
        .unwrap();
        assert_eq!(apply_change(base, &ch), "decl { x : d; }\nq = z * 2\n");

        // Cross-line deletion: delete the whole second line
        let ch = parse_json(
            r#"{"text":"","range":{"start":{"line":1,"character":0},"end":{"line":2,"character":0}}}"#,
        )
        .unwrap();
        assert_eq!(apply_change(base, &ch), "decl { x : d; }\n");

        // Insertion (zero-width range): insert at the start of the line
        let ch = parse_json(
            r#"{"text":"% hi\n","range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}}}"#,
        )
        .unwrap();
        assert_eq!(
            apply_change(base, &ch),
            "% hi\ndecl { x : d; }\nq = x * 2\n"
        );

        // No range = full replacement (old clients / hand-written scripts)
        let ch = parse_json(r#"{"text":"decl { y : d; }\nq = y\n"}"#).unwrap();
        assert_eq!(apply_change(base, &ch), "decl { y : d; }\nq = y\n");

        // Out-of-range position → **clamp** (never drop the change): a line number past the last
        // line lands on the last line
        let ch = parse_json(
            r#"{"text":"!","range":{"start":{"line":99,"character":99},"end":{"line":99,"character":99}}}"#,
        )
        .unwrap();
        let got = apply_change(base, &ch);
        assert!(
            got.starts_with("decl { x : d; }\nq = x * 2"),
            "不该破坏缓冲：{got}"
        );
        assert!(got.ends_with('!'), "越界插入应落在末尾：{got}");
    }

    #[test]
    fn did_change_applies_incremental_edits_end_to_end() {
        pin_locale();
        let mut st = State::default();
        let open = r#"{"textDocument":{"uri":"file:///inc.formula","text":"decl { x : d; }\nq = x * 2\n"}}"#;
        let out = call(&mut st, "textDocument/didOpen", open).unwrap();
        assert!(out.contains("\"diagnostics\":[]"), "{out}");
        // Incremental: change x to the undeclared z → the diagnostic must appear at once
        let change = r#"{"textDocument":{"uri":"file:///inc.formula"},"contentChanges":[{"text":"z","range":{"start":{"line":1,"character":4},"end":{"line":1,"character":5}}}]}"#;
        let out = call(&mut st, "textDocument/didChange", change).unwrap();
        assert!(
            out.contains("undeclared symbol"),
            "增量编辑后应产生诊断：{out}"
        );
        assert_eq!(
            st.docs.get("file:///inc.formula").map(|s| s.as_str()),
            Some("decl { x : d; }\nq = z * 2\n")
        );
    }

    #[test]
    fn incremental_edits_count_utf16_units_and_clamp_to_line_end() {
        pin_locale();
        // ① LSP's `character` counts UTF-16 code units: an emoji (outside the BMP) takes 2.
        //    Counting chars would misplace the edit — measured to drop it at the end of the file and
        //    silently swallow the second half.
        let base = "decl { x : d; }\nq = x * 2  % 😀 note\nr = x\n";
        // Delete the emoji in the comment: it starts at UTF-16 column 13 (`q = x * 2  % ` is 13
        // code units)
        let ch = parse_json(
            r#"{"text":"","range":{"start":{"line":1,"character":13},"end":{"line":1,"character":15}}}"#,
        )
        .unwrap();
        let got = apply_change(base, &ch);
        assert_eq!(
            got, "decl { x : d; }\nq = x * 2  %  note\nr = x\n",
            "emoji 应被删掉"
        );
        assert!(got.contains("r = x"), "后半篇不能被吞：{got}");

        // ② An in-line overflow must clamp to **the end of this line**, not the end of the document
        let ch = parse_json(
            r#"{"text":"!","range":{"start":{"line":0,"character":99},"end":{"line":0,"character":99}}}"#,
        )
        .unwrap();
        let got = apply_change("abc\ndef\n", &ch);
        assert_eq!(got, "abc!\ndef\n", "越界插入应落在第 0 行末");
    }

    #[test]
    fn decl_quickfix_keeps_a_single_line_decl_block_parseable() {
        pin_locale();
        // A single-line decl block: the added declaration goes before `}` (starting a new line would
        // turn the file into a syntax error)
        let src = "decl { sigma : d; }\nq = sigam * 2\n";
        let fixes = quickfixes(src);
        let decl = fixes
            .iter()
            .find(|f| f.new_text.contains("quantity"))
            .expect("应给出补声明");
        // Single-point insertion (the source is pure ASCII, so char column == byte column)
        let (ln, col) = decl.start;
        let mut lines: Vec<String> = src.lines().map(|l| l.to_string()).collect();
        let mut line_chars: Vec<char> = lines[ln].chars().collect();
        let at = col.min(line_chars.len());
        let ins: Vec<char> = decl.new_text.chars().collect();
        line_chars.splice(at..at, ins);
        lines[ln] = line_chars.into_iter().collect();
        let out = lines.join("\n") + "\n";
        assert!(
            out.starts_with("decl { sigma : d; sigam : quantity [1]; }"),
            "{out}"
        );
        // Key point: after the edit it must **still parse**
        assert!(
            himari_loader::parse_source(&out, "<t>").is_ok(),
            "补声明后的文件必须可解析：{out}"
        );
    }

    #[test]
    fn decl_quickfix_never_matches_a_formula_named_decl_something() {
        pin_locale();
        // `decl_ratio = 2` is a formula name, not a decl block. It used to match on the prefix → the
        // declaration line was inserted after it and a legal file became a syntax error (the
        // quick-fix broke the user's file).
        for src in [
            "decl_ratio = 2\nq = sigam * decl_ratio\n",
            "decline = 2\nq = sigam * decline\n",
        ] {
            let fixes = quickfixes(src);
            assert!(
                fixes.iter().all(|f| !f.new_text.contains("quantity")),
                "没有 decl 块时不该提供补声明：{:?}",
                fixes.iter().map(|f| &f.title).collect::<Vec<_>>()
            );
            // Only "change the spelling" or nothing should be offered — but never an edit that breaks
            // the file
            for f in &fixes {
                let mut lines: Vec<String> = src.lines().map(|l| l.to_string()).collect();
                let (ln, col) = f.start;
                if ln >= lines.len() {
                    continue;
                }
                let mut cs: Vec<char> = lines[ln].chars().collect();
                let at = col.min(cs.len());
                let ins: Vec<char> = f.new_text.chars().collect();
                cs.splice(at..at, ins);
                lines[ln] = cs.into_iter().collect();
                let out = lines.join("\n") + "\n";
                assert!(
                    himari_loader::parse_source(&out, "<t>").is_ok(),
                    "应用修复后必须仍可解析：{out}"
                );
            }
        }
    }

    #[test]
    fn decl_quickfix_handles_latex_braces_and_comment_markers() {
        pin_locale();
        // Review counterexample: `{` is everywhere on the LaTeX surface, so checking only "the line
        // contains {" would insert the declaration inside `\frac{A}{E}` — a legal file becomes a
        // syntax error.
        for src in [
            "decline = \\frac{A}{E}\nq = sigam * decline\n",
            "declaration = \\frac{1}{2}\nq = sigam * declaration\n",
            "declx = \\sqrt{x}\nq = sigam * declx\n",
        ] {
            let fixes = quickfixes(src);
            assert!(
                fixes.iter().all(|f| !f.new_text.contains("quantity")),
                "`decl` 前缀的公式名不该被当成 decl 块：{:?}",
                fixes.iter().map(|f| &f.title).collect::<Vec<_>>()
            );
        }
        // A real decl block (after `decl` only whitespace, then `{`) must still work
        for src in [
            "decl { sigma : d; }\nq = sigam * 2\n",
            "decl{ sigma : d; }\nq = sigam * 2\n",
            "decl {\n    sigma : d;\n}\nq = sigam * 2\n",
        ] {
            let fixes = quickfixes(src);
            let decl = fixes
                .iter()
                .find(|f| f.new_text.contains("quantity"))
                .unwrap_or_else(|| panic!("真 decl 块应给补声明：{src}"));
            // It must still parse after applying the fix
            let (ln, col) = decl.start;
            let mut lines: Vec<String> = src.lines().map(|l| l.to_string()).collect();
            let mut cs: Vec<char> = lines[ln].chars().collect();
            let at = col.min(cs.len());
            let ins: Vec<char> = decl.new_text.chars().collect();
            cs.splice(at..at, ins);
            lines[ln] = cs.into_iter().collect();
            let out = lines.join("\n") + "\n";
            assert!(
                himari_loader::parse_source(&out, "<t>").is_ok(),
                "补声明后应可解析：{out}"
            );
        }
        // A `{` inside a matex comment must not trigger it either (`decline = 2 % { not a decl`)
        let commented = "decline = 2 % { not a decl\nq = sigam * decline\n";
        let fixes = quickfixes(commented);
        assert!(fixes.iter().all(|f| !f.new_text.contains("quantity")));
    }

    #[test]
    fn did_change_recomputes_from_the_buffer() {
        pin_locale();
        let mut st = State::default();
        let open =
            r#"{"textDocument":{"uri":"file:///c.formula","text":"decl { x : d; }\nx * 2\n"}}"#;
        let out = call(&mut st, "textDocument/didOpen", open).unwrap();
        assert!(out.contains("\"diagnostics\":[]"), "{out}");
        // Change to content with an error (full sync)
        let change = r#"{"textDocument":{"uri":"file:///c.formula"},"contentChanges":[{"text":"decl { x : d; }\nx + z\n"}]}"#;
        let out = call(&mut st, "textDocument/didChange", change).unwrap();
        assert!(out.contains("undeclared symbol"), "{out}");
    }

    #[test]
    fn code_action_fixes_undeclared_symbol_two_ways() {
        pin_locale();
        // `sigam` (misspelled) appears once in the source and `sigma` is declared → both should be
        // offered:
        // ① change the spelling ② add a declaration line
        let src = "decl {\n    sigma : d;\n}\nq = sigam * 2\n";
        let fixes = quickfixes(src);
        assert_eq!(
            fixes.len(),
            2,
            "{:?}",
            fixes.iter().map(|f| &f.title).collect::<Vec<_>>()
        );
        let spell = &fixes[0];
        assert!(spell.title.contains("sigma"), "{}", spell.title);
        assert!(spell.preferred, "拼写纠正应是首选");
        // The range must cover `sigam` exactly (line 3, starting at column 4)
        assert_eq!(spell.start, (3, 4), "{:?}", spell.start);
        assert_eq!(spell.end, (3, 9), "{:?}", spell.end);
        assert_eq!(spell.new_text, "sigma");
        // Add a declaration: insert after the decl block's `{`, reusing the four-space indent
        let decl = &fixes[1];
        assert_eq!(decl.start, (1, 0));
        assert_eq!(decl.new_text, "    sigam : quantity [1];\n");
        assert!(!decl.preferred);
    }

    #[test]
    fn code_action_fixes_unknown_function_typo() {
        pin_locale();
        // `expp` → the nearest registry name `exp`.
        // The argument must be a **non-bare symbol**: a shape like `f(x)` is recognized by the matlab
        // front end as the indexed symbol a(i) (a tensor atom, not a function call) — which then
        // reports an "undeclared symbol" instead of an unknown function.
        let src = "decl {\n    x : d;\n}\ny = expp(x * 2)\n";
        let fixes = quickfixes(src);
        let f = fixes
            .iter()
            .find(|f| f.new_text == "exp")
            .unwrap_or_else(|| {
                panic!(
                    "应给出 exp 的修复：{:?}",
                    fixes.iter().map(|f| &f.title).collect::<Vec<_>>()
                )
            });
        assert_eq!(f.start, (3, 4));
        assert_eq!(f.end, (3, 8));
        assert!(f.preferred);
    }

    #[test]
    fn code_action_stays_silent_when_it_cannot_be_sure() {
        pin_locale();
        // The name appears twice → no replacement (do not guess which one); a declaration can still
        // be added
        let src = "decl {\n    sigma : d;\n}\nq = sigam + sigam\n";
        let fixes = quickfixes(src);
        assert!(
            fixes.iter().all(|f| !f.title.contains("改成")),
            "多处出现时不该猜替换点"
        );
        // No fixes on a completely clean source
        let ok = "decl {\n    x : d;\n}\ny = x * 2\n";
        assert!(quickfixes(ok).is_empty());
        // No replacement either when there is no similar name in the declaration table
        let far = "decl {\n    abcdef : d;\n}\nq = zzzzz\n";
        assert!(quickfixes(far).iter().all(|f| !f.title.contains("改成")));
    }

    #[test]
    fn code_action_response_is_well_formed() {
        pin_locale();
        let mut st = State::default();
        let open = r#"{"textDocument":{"uri":"file:///ca.formula","text":"decl {
    sigma : d;
}
q = sigam * 2
"}}"#;
        call(&mut st, "textDocument/didOpen", open);
        let out = call(
            &mut st,
            "textDocument/codeAction",
            r#"{"textDocument":{"uri":"file:///ca.formula"},"range":{},"context":{}}"#,
        )
        .unwrap();
        assert!(out.contains("quickfix"), "{out}");
        assert!(out.contains("isPreferred"), "{out}");
        assert!(out.contains("file:///ca.formula"), "{out}");
        assert!(out.contains("newText"), "{out}");
        // A clean document → an empty array
        let clean = r#"{"textDocument":{"uri":"file:///clean.formula","text":"decl {
    x : d;
}
y = x * 2
"}}"#;
        call(&mut st, "textDocument/didOpen", clean);
        let out = call(
            &mut st,
            "textDocument/codeAction",
            r#"{"textDocument":{"uri":"file:///clean.formula"}}"#,
        )
        .unwrap();
        assert!(out.contains(r#""result":[]"#), "{out}");
    }

    #[test]
    fn completion_offers_functions_and_declared_symbols() {
        pin_locale();
        let mut st = State::default();
        let open = r#"{"textDocument":{"uri":"file:///d.formula","text":"decl { sigma : d; }\nmy_val = sigma * 2\n"}}"#;
        call(&mut st, "textDocument/didOpen", open);
        let out = call(
            &mut st,
            "textDocument/completion",
            r#"{"textDocument":{"uri":"file:///d.formula"}}"#,
        )
        .unwrap();
        assert!(out.contains("\"sqrt\""), "{out}");
        assert!(out.contains("\"sigma\""), "{out}");
        assert!(out.contains("\"my_val\""), "{out}");
    }

    #[test]
    fn surrogate_pairs_compose_instead_of_dropping() {
        pin_locale();
        // A message with 4-byte UTF-8 (escaped surrogate pairs) used to be dropped entirely
        let v = parse_json(r#"{"k":"\ud83d\ude00"}"#).unwrap();
        let s = v.get("k").and_then(Json::as_str).unwrap();
        assert_eq!(s, "\u{1F600}");
        // A lone surrogate → U+FFFD, and it still parses (the message is not dropped)
        let v = parse_json(r#"{"k":"\ud83d"}"#).unwrap();
        assert!(v.get("k").is_some());
    }

    #[test]
    fn deep_nesting_is_capped_not_crashing() {
        pin_locale();
        let deep = format!("{}1{}", "[".repeat(5000), "]".repeat(5000));
        assert!(parse_json(&deep).is_none(), "超深嵌套应解析失败");
        // Under the limit it works as usual
        let ok = format!("{}1{}", "[".repeat(50), "]".repeat(50));
        assert!(parse_json(&ok).is_some());
    }

    #[test]
    fn scan_id_finds_numeric_and_string_ids() {
        pin_locale();
        assert_eq!(
            scan_id(r#"{"jsonrpc":"2.0","id":42,"method":"x"}"#),
            Json::Num(42.0)
        );
        assert_eq!(
            scan_id(r#"{"id":"abc-1","method":"x"}"#),
            Json::Str("abc-1".into())
        );
        assert_eq!(scan_id("no id here"), Json::Null);
    }

    #[test]
    fn frame_headers_and_unknown_methods_are_tolerated() {
        pin_locale();
        // Unknown method: reply null when there is an id (the client will not hang)
        let mut st = State::default();
        let out = call(&mut st, "workspace/something", "{}").unwrap();
        assert!(out.contains("null"), "{out}");
    }
}
