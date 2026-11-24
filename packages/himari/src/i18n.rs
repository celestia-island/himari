//! Localization for user-facing messages.
//!
//! The project speaks nine languages, mirroring the rest of the org
//! (`lagrange.toml`: `en, zh-Hans, zh-Hant, ja, ko, fr, es, ru, ar`, default `en`).
//!
//! Two rules make this maintainable:
//!
//! 1. **Errors are identified by a stable code, not by a sentence.** Callers match on
//!    [`Error::code`]; the text is produced at the edge by [`render`]. Tests therefore assert
//!    [`Error::code`] (or destructure the variant) rather than a translated sentence. Two
//!    tracked exceptions: (a) a test may assert a substring of a *staging* `Message` payload —
//!    that text passes through verbatim by contract, so the assertion pins the payload; (b) a
//!    catalogue test may assert a localized fragment to prove a variant IS translated, and
//!    such tests should pair it with a negative assertion that no English phrase leaks.
//!    Neither exception asserts a translation as a stand-in for the code.
//! 2. **A missing sentence falls back to English; payloads do not.** A code whose arm is absent
//!    from a locale's table renders the English template verbatim (never a blank line). Payload
//!    values composed at construction sites (suggestions, arithmetic hints) are locale-neutral
//!    or English by design — they travel inside the variant, so the catalogue can only translate
//!    the sentence around them.
//!
//! This module is deliberately allocation-light: rendering writes into a `String`, and
//! the catalog contains no hash map, so it works under `no_std + alloc`.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

/// The nine supported languages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Locale {
    /// English (default).
    #[default]
    En,
    /// Simplified Chinese.
    ZhHans,
    /// Traditional Chinese.
    ZhHant,
    /// Japanese.
    Ja,
    /// Korean.
    Ko,
    /// French.
    Fr,
    /// Spanish.
    Es,
    /// Russian.
    Ru,
    /// Arabic.
    Ar,
}

impl Locale {
    /// Every locale, in the org's canonical order.
    pub const ALL: [Locale; 9] = [
        Locale::En,
        Locale::ZhHans,
        Locale::ZhHant,
        Locale::Ja,
        Locale::Ko,
        Locale::Fr,
        Locale::Es,
        Locale::Ru,
        Locale::Ar,
    ];

    /// BCP 47 tag used by the org's locale directories and `lagrange.toml`.
    pub fn tag(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::ZhHans => "zh-Hans",
            Locale::ZhHant => "zh-Hant",
            Locale::Ja => "ja",
            Locale::Ko => "ko",
            Locale::Fr => "fr",
            Locale::Es => "es",
            Locale::Ru => "ru",
            Locale::Ar => "ar",
        }
    }

    /// Parses a tag, accepting the common shorter spellings (`zh`, `zh-CN`, `zh-TW`).
    ///
    /// An unknown tag is `None`: callers decide whether that is an error or a fallback.
    pub fn parse(tag: &str) -> Option<Locale> {
        let normalized = tag.trim().replace('_', "-").to_ascii_lowercase();
        let base = normalized.split(['-', '.']).next().unwrap_or("");
        Some(match normalized.as_str() {
            "zh-hans" | "zh-cn" | "zh-sg" => Locale::ZhHans,
            "zh-hant" | "zh-tw" | "zh-hk" | "zh-mo" => Locale::ZhHant,
            "en" | "en-us" | "en-gb" => Locale::En,
            "ja" | "ja-jp" => Locale::Ja,
            "ko" | "ko-kr" => Locale::Ko,
            "fr" | "fr-fr" => Locale::Fr,
            "es" | "es-es" => Locale::Es,
            "ru" | "ru-ru" => Locale::Ru,
            "ar" | "ar-sa" => Locale::Ar,
            _ => match base {
                "zh" => Locale::ZhHans,
                "en" => Locale::En,
                "ja" => Locale::Ja,
                "ko" => Locale::Ko,
                "fr" => Locale::Fr,
                "es" => Locale::Es,
                "ru" => Locale::Ru,
                "ar" => Locale::Ar,
                _ => return None,
            },
        })
    }

    /// Resolves the locale from `HIMARI_LANG`, then `LC_ALL`, then `LANG`.
    ///
    /// Only available with `std`: under `no_std` there is no environment to read.
    #[cfg(feature = "std")]
    pub fn from_env() -> Locale {
        for key in ["HIMARI_LANG", "LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(value) = std::env::var(key) {
                if let Some(locale) = Locale::parse(&value) {
                    return locale;
                }
            }
        }
        Locale::En
    }
}

/// The crate's error type.
///
/// The `as_str` form is the wire/API spelling: it appears in CLI output and in tests, and
/// must never change once released. Adding a variant is additive; renaming one is not.
/// Why a declaration name is reserved (translatable, unlike free text).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReservedReason {
    /// The name is also a built-in function; a call site would be ambiguous.
    BuiltinFunction,
    /// The name is the reserved imaginary-unit constant `j`.
    ImaginaryUnit,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    // ---- parsing / declarations
    SyntaxError(String),
    UndeclaredSymbol(String),
    DuplicateDeclaration {
        name: String,
        /// `None` when the declaration carries no source position (hand-built sets).
        line: Option<usize>,
    },
    ReservedName {
        name: String,
        reason: ReservedReason,
    },
    UnknownFunction {
        name: String,
        suggestion: String,
    },
    FunctionArity {
        name: String,
        expected: usize,
        got: usize,
    },
    EmptyFormula,
    NestingTooDeep {
        got: String,
        limit: String,
    },
    InvisibleCharacter(String),
    UnknownUnit(String),
    OffsetUnit(String),
    LogUnit(String),
    AmbiguousUnit {
        name: String,
        why: String,
        remedy: String,
    },
    MalformedUnit(String, String),
    // ---- dimensions / shapes
    DimensionMismatch {
        left: String,
        right: String,
    },
    ExponentNotDimensionless(String),
    ExponentNotHalfInteger(String),
    ShapeMismatch(String),
    MatrixScalarMix(String),
    MatrixDivisionUnsupported(String),
    // ---- symbolic solving
    NonlinearEquation {
        name: String,
        why: String,
    },
    Underdetermined(String),
    Inconsistent,
    MatrixCoefficient(String),
    // ---- runtime lane
    SingularMatrix {
        column: usize,
    },
    IllConditioned {
        rcond: f64,
        limit: f64,
    },
    UnstableResidual {
        residual: f64,
        limit: f64,
    },
    NoRealRoots {
        discriminant: f64,
    },
    NonFiniteValue(String),
    InvalidInput(String),
    ShapeError(String),
    // ---- emission / tooling
    NotEmittable(String),
    UnsupportedOperation(String),
    IoError(String),
    /// A declaration's unit is invalid; carries the declared symbol name and the unit error.
    DeclarationUnit {
        name: String,
        unit: Box<Error>,
    },
    /// A formula's result dimension differs from the dimension declared for its name.
    ResultDimensionMismatch {
        name: String,
        got: String,
        declared: String,
        line: usize,
    },
    /// A user-visible label for an unnamed formula ("formula #3"), localized like a message.
    FormulaLabel(usize),
    /// A message that has not been catalogued yet. Rendering returns it verbatim, so a
    /// staged migration stays honest: nothing is silently translated or dropped.
    Message(String),
}

impl Error {
    /// The stable wire code for this error (kebab-case).
    pub fn code(&self) -> &'static str {
        match self {
            Error::Message(_) => "message",
            Error::DeclarationUnit { .. } => "declaration-unit",
            Error::ResultDimensionMismatch { .. } => "result-dimension-mismatch",
            Error::FormulaLabel(_) => "formula-label",
            other => other.static_code(),
        }
    }

    /// Renders this error in `locale`. `Message` passes through unchanged.
    pub fn render(&self, locale: Locale) -> String {
        match self {
            Error::Message(text) => text.clone(),
            other => render(other, &[], locale),
        }
    }

    fn static_code(&self) -> &'static str {
        match self {
            Error::Message(_) => "message",
            Error::DeclarationUnit { .. } => "declaration-unit",
            Error::ResultDimensionMismatch { .. } => "result-dimension-mismatch",
            Error::FormulaLabel(_) => "formula-label",
            Error::SyntaxError(_) => "syntax-error",
            Error::UndeclaredSymbol(_) => "undeclared-symbol",
            Error::DuplicateDeclaration { .. } => "duplicate-declaration",
            Error::ReservedName { .. } => "reserved-name",
            Error::UnknownFunction { .. } => "unknown-function",
            Error::FunctionArity { .. } => "function-arity",
            Error::EmptyFormula => "empty-formula",
            Error::NestingTooDeep { .. } => "nesting-too-deep",
            Error::InvisibleCharacter(_) => "invisible-character",
            Error::UnknownUnit(_) => "unknown-unit",
            Error::OffsetUnit(_) => "offset-unit",
            Error::LogUnit(_) => "log-unit",
            Error::AmbiguousUnit { .. } => "ambiguous-unit",
            Error::MalformedUnit(_, _) => "malformed-unit",
            Error::DimensionMismatch { .. } => "dimension-mismatch",
            Error::ExponentNotDimensionless(_) => "exponent-not-dimensionless",
            Error::ExponentNotHalfInteger(_) => "exponent-not-half-integer",
            Error::ShapeMismatch(_) => "shape-mismatch",
            Error::MatrixScalarMix(_) => "matrix-scalar-mix",
            Error::MatrixDivisionUnsupported(_) => "matrix-division-unsupported",
            Error::NonlinearEquation { .. } => "nonlinear-equation",
            Error::Underdetermined(_) => "underdetermined",
            Error::Inconsistent => "inconsistent",
            Error::MatrixCoefficient(_) => "matrix-coefficient",
            Error::SingularMatrix { .. } => "singular-matrix",
            Error::IllConditioned { .. } => "ill-conditioned",
            Error::UnstableResidual { .. } => "unstable-residual",
            Error::NoRealRoots { .. } => "no-real-roots",
            Error::NonFiniteValue(_) => "non-finite-value",
            Error::InvalidInput(_) => "invalid-input",
            Error::ShapeError(_) => "shape-error",
            Error::NotEmittable(_) => "not-emittable",
            Error::UnsupportedOperation(_) => "unsupported-operation",
            Error::IoError(_) => "io-error",
        }
    }

    /// English template for this code. Used for every locale that lacks a specific entry
    /// and as the source of truth when reviewing translations.
    fn template_en(&self) -> &'static str {
        match self {
            Error::SyntaxError(_) => "syntax error: {0}",
            Error::UndeclaredSymbol(_) => "undeclared symbol `{0}`: every symbol needs a declared dimension",
            Error::DuplicateDeclaration { .. } => "symbol `{0}` is declared more than once{1}; a formula set must not overwrite",
            Error::ReservedName { .. } => "`{0}` is reserved ({1}) and cannot be declared",
            Error::UnknownFunction { .. } => "unknown function `{0}`",
            Error::FunctionArity { .. } => "function `{0}` takes {1} argument(s), got {2}",
            Error::EmptyFormula => "the set contains no formula (at least one is required)",
            Error::NestingTooDeep { .. } => "nesting too deep ({0} > limit {1}): split the formula",
            Error::InvisibleCharacter(_) => "invisible character U+{0}: it can hide what is being reviewed",
            Error::UnknownUnit(_) => "unknown unit `{0}`",
            Error::OffsetUnit(_) => "offset unit `{0}` is unsupported: this algebra is multiplicative, use K and shift outside the formula",
            Error::LogUnit(_) => "logarithmic unit `{0}` is unsupported: it is a ratio, not a dimension",
            Error::AmbiguousUnit { .. } => "unit `{0}` is ambiguous ({1}): {2}",
            Error::MalformedUnit(_, _) => "malformed unit `{0}`: {1}",
            Error::DimensionMismatch { .. } => "dimension mismatch: {0} vs {1}",
            Error::ExponentNotDimensionless(_) => "the exponent must be dimensionless, got {0}",
            Error::ExponentNotHalfInteger(_) => "exponent `{0}` is not a multiple of 1/2, so the dimension is undefined",
            Error::ShapeMismatch(_) => "shape mismatch: {0}",
            Error::MatrixScalarMix(_) => "matrix and scalar cannot be combined here: {0}",
            Error::MatrixDivisionUnsupported(_) => "matrix division is unsupported: use inv(A) * b or solve(A, b)",
            Error::NonlinearEquation { .. } => "the equation is not linear in `{0}`: {1}",
            Error::Underdetermined(_) => "the system is underdetermined for `{0}`: the solution is not unique",
            Error::Inconsistent => "the system is inconsistent: elimination produced 0 = non-zero",
            Error::MatrixCoefficient(_) => "the coefficient of `{0}` is matrix-valued: solve a scalar system or write inv(A) * b",
            Error::SingularMatrix { .. } => "matrix is singular at column {0}",
            Error::IllConditioned { .. } => "matrix is too ill-conditioned (rcond={0}, required >= {1})",
            Error::UnstableResidual { .. } => "the residual is too large (relative {0}, limit {1})",
            Error::NoRealRoots { .. } => "no real roots (discriminant {0}): write `j` explicitly for the complex lane",
            Error::NonFiniteValue(_) => "non-finite value: {0}",
            Error::InvalidInput(_) => "invalid input: {0}",
            Error::ShapeError(_) => "shape error: {0}",
            Error::NotEmittable(_) => "the expression cannot be emitted: {0}",
            Error::UnsupportedOperation(_) => "unsupported operation: {0}",
            Error::IoError(_) => "I/O error: {0}",
            // `render` returns the message verbatim before consulting templates.
            Error::Message(_) => "{0}",
            Error::DeclarationUnit { .. } => "declaration `{0}`: {1}",
            Error::ResultDimensionMismatch { .. } => {
                "formula `{0}` produces {1} but its name is declared as {2} (line {3})"
            }
            Error::FormulaLabel(_) => "formula #{0}",
        }
    }

    fn template_zh_hans(&self) -> &'static str {
        match self {
            Error::UndeclaredSymbol(_) => "未声明的符号 `{0}`：每个符号都必须显式声明量纲",
            Error::DuplicateDeclaration { .. } => {
                "符号 `{0}` 被重复声明（第 {1} 行）：同一公式集不允许覆盖"
            }
            Error::ReservedName { .. } => "`{0}` 是保留名（{1}），不能声明",
            Error::UnknownFunction { .. } => "未知函数 `{0}`",
            Error::FunctionArity { .. } => "函数 `{0}` 需要 {1} 个参数，实际给了 {2}",
            Error::EmptyFormula => "空公式（至少一条）",
            Error::NestingTooDeep { .. } => "嵌套过深（{0} > 上限 {1}）：拆成多条公式再算",
            Error::InvisibleCharacter(_) => {
                "源里有不可见字符 U+{0}：它会让人评审到和实际不一样的东西"
            }
            Error::UnknownUnit(_) => "未知单位 `{0}`",
            Error::OffsetUnit(_) => {
                "温度偏移单位 `{0}` 不支持：本设施只有乘法量纲，请用 K 并在公式外做偏移"
            }
            Error::LogUnit(_) => "对数单位 `{0}` 不支持：它是比值不是量纲",
            Error::DimensionMismatch { .. } => "量纲不一致：{0} vs {1}",
            Error::ExponentNotDimensionless(_) => "指数必须无量纲，得到 {0}",
            Error::ShapeMismatch(_) => "形状不一致：{0}",
            Error::MatrixDivisionUnsupported(_) => "矩阵除法不支持：请写 inv(A) * b 或 solve(A, b)",
            Error::NonlinearEquation { .. } => "方程对 `{0}` 不是线性的：{1}",
            Error::Underdetermined(_) => "方程组对 `{0}` 不满秩（欠定）：解不唯一",
            Error::Inconsistent => "方程组矛盾（消元后出现 0 = 非零）：无解",
            Error::SingularMatrix { .. } => "矩阵奇异（第 {0} 列）",
            Error::IllConditioned { .. } => "矩阵条件数过大（rcond={0}，要求 >= {1}）",
            Error::UnstableResidual { .. } => "解残差过大（相对 {0}，上限 {1}）",
            Error::NoRealRoots { .. } => "无实根（判别式 {0}）：需要复根请显式写 `j` 进入复数车道",
            Error::NonFiniteValue(_) => "非有限值：{0}",
            Error::InvalidInput(_) => "输入无效：{0}",
            Error::ShapeError(_) => "形状错误：{0}",
            Error::SyntaxError(_) => "语法错误：{0}",
            Error::AmbiguousUnit { .. } => "单位 `{0}` 有歧义（{1}）：{2}",
            Error::MalformedUnit(_, _) => "单位串 `{0}` 格式错误：{1}",
            Error::MatrixScalarMix(_) => "矩阵与标量不能这样组合：{0}",
            Error::ExponentNotHalfInteger(_) => "指数 `{0}` 不是 1/2 的整数倍，量纲无定义",
            Error::NotEmittable(_) => "该表达式无法发射为代码：{0}",
            Error::UnsupportedOperation(_) => "不支持的操作：{0}",
            Error::IoError(_) => "I/O 错误：{0}",
            Error::MatrixCoefficient(_) => {
                "方程里 `{0}` 的系数是矩阵值：请解标量方程组或写 inv(A) * b"
            }
            Error::FormulaLabel(_) => "第 {0} 条",
            Error::DeclarationUnit { .. } => "声明 `{0}` 的单位错：{1}",
            Error::ResultDimensionMismatch { .. } => {
                "公式 `{0}` 的结果量纲为 {1}，与其声明的 {2} 不一致（第 {3} 行）"
            }
            _ => self.template_en(),
        }
    }

    fn template_zh_hant(&self) -> &'static str {
        match self {
            Error::UndeclaredSymbol(_) => "未宣告的符號 `{0}`：每個符號都必須明確宣告量綱",
            Error::DuplicateDeclaration { .. } => {
                "符號 `{0}` 被重複宣告（第 {1} 行）：同一公式集不允許覆蓋"
            }
            Error::ReservedName { .. } => "`{0}` 是保留名（{1}），不能宣告",
            Error::UnknownFunction { .. } => "未知函式 `{0}`",
            Error::FunctionArity { .. } => "函式 `{0}` 需要 {1} 個參數，實際給了 {2}",
            Error::EmptyFormula => "空公式（至少一條）",
            Error::NestingTooDeep { .. } => "巢狀過深（{0} > 上限 {1}）：拆成多條公式再算",
            Error::InvisibleCharacter(_) => {
                "原始碼中有不可見字元 U+{0}：它會讓人審閱到與實際不同的內容"
            }
            Error::UnknownUnit(_) => "未知單位 `{0}`",
            Error::OffsetUnit(_) => {
                "溫度偏移單位 `{0}` 不支援：本設施只有乘法量綱，請用 K 並在公式外做偏移"
            }
            Error::LogUnit(_) => "對數單位 `{0}` 不支援：它是比值而非量綱",
            Error::DimensionMismatch { .. } => "量綱不一致：{0} vs {1}",
            Error::ExponentNotDimensionless(_) => "指數必須無量綱，得到 {0}",
            Error::ShapeMismatch(_) => "形狀不一致：{0}",
            Error::MatrixDivisionUnsupported(_) => "矩陣除法不支援：請寫 inv(A) * b 或 solve(A, b)",
            Error::NonlinearEquation { .. } => "方程式對 `{0}` 不是線性的：{1}",
            Error::Underdetermined(_) => "方程組對 `{0}` 不滿秩（欠定）：解不唯一",
            Error::Inconsistent => "方程組矛盾（消元後出現 0 = 非零）：無解",
            Error::SingularMatrix { .. } => "矩陣奇異（第 {0} 欄）",
            Error::IllConditioned { .. } => "矩陣條件數過大（rcond={0}，要求 >= {1}）",
            Error::UnstableResidual { .. } => "解殘差過大（相對 {0}，上限 {1}）",
            Error::NoRealRoots { .. } => "無實根（判別式 {0}）：需要複根請明確寫 `j` 進入複數車道",
            Error::NonFiniteValue(_) => "非有限值：{0}",
            Error::InvalidInput(_) => "輸入無效：{0}",
            Error::ShapeError(_) => "形狀錯誤：{0}",
            Error::SyntaxError(_) => "語法錯誤：{0}",
            Error::AmbiguousUnit { .. } => "單位 `{0}` 有歧義（{1}）：{2}",
            Error::MalformedUnit(_, _) => "單位字串 `{0}` 格式錯誤：{1}",
            Error::MatrixScalarMix(_) => "矩陣與純量不能這樣組合：{0}",
            Error::ExponentNotHalfInteger(_) => "指數 `{0}` 不是 1/2 的整數倍，量綱無定義",
            Error::NotEmittable(_) => "此運算式無法產生程式碼：{0}",
            Error::UnsupportedOperation(_) => "不支援的操作：{0}",
            Error::IoError(_) => "I/O 錯誤：{0}",
            Error::MatrixCoefficient(_) => {
                "方程式裡 `{0}` 的係數是矩陣值：請解純量方程組或寫 inv(A) * b"
            }
            Error::FormulaLabel(_) => "第 {0} 條",
            Error::DeclarationUnit { .. } => "宣告 `{0}` 的單位錯：{1}",
            Error::ResultDimensionMismatch { .. } => {
                "公式 `{0}` 的結果量綱為 {1}，與其宣告的 {2} 不一致（第 {3} 行）"
            }
            _ => self.template_en(),
        }
    }

    fn template_ja(&self) -> &'static str {
        match self {
            Error::UndeclaredSymbol(_) => "未宣言のシンボル `{0}`：すべてのシンボルに次元の宣言が必要です",
            Error::DuplicateDeclaration { .. } => "シンボル `{0}` が重複して宣言されています{1}：同じ式セットでの上書きはできません",
            Error::ReservedName { .. } => "`{0}` は予約名のため宣言できません（{1}）",
            Error::UnknownFunction { .. } => "未知の関数 `{0}`",
            Error::FunctionArity { .. } => "関数 `{0}` は {1} 個の引数を取りますが、{2} 個渡されました",
            Error::EmptyFormula => "空の式です（少なくとも 1 つ必要）",
            Error::NestingTooDeep { .. } => "入れ子が深すぎます（{0} > 上限 {1}）：式を分割してください",
            Error::InvisibleCharacter(_) => "不可視文字 U+{0} が含まれています：レビュー内容と実際が食い違う恐れがあります",
            Error::UnknownUnit(_) => "未知の単位 `{0}`",
            Error::OffsetUnit(_) => "オフセット単位 `{0}` は未対応です：本機構は乗法的な次元のみを扱うため、K を使い式の外で補正してください",
            Error::LogUnit(_) => "対数単位 `{0}` は未対応です：次元ではなく比です",
            Error::DimensionMismatch { .. } => "次元が一致しません：{0} vs {1}",
            Error::ExponentNotDimensionless(_) => "指数は無次元でなければなりません（{0}）",
            Error::ShapeMismatch(_) => "形状が一致しません：{0}",
            Error::MatrixDivisionUnsupported(_) => "行列の除算は未対応です：inv(A) * b または solve(A, b) を使ってください",
            Error::NonlinearEquation { .. } => "式は `{0}` について線形ではありません：{1}",
            Error::Underdetermined(_) => "連立方程式が `{0}` について不足階数（不定）：解は一意ではありません",
            Error::Inconsistent => "連立方程式が矛盾しています（消去後に 0 = 非ゼロ）：解なし",
            Error::SingularMatrix { .. } => "行列が特異です（列 {0}）",
            Error::IllConditioned { .. } => "行列の条件数が大きすぎます（rcond={0}、必要 >= {1}）",
            Error::UnstableResidual { .. } => "残差が大きすぎます（相対 {0}、上限 {1}）",
            Error::NoRealRoots { .. } => "実根がありません（判別式 {0}）：複素根が必要なら `j` を明示してください",
            Error::NonFiniteValue(_) => "非有限値：{0}",
            Error::InvalidInput(_) => "入力が不正です：{0}",
            Error::ShapeError(_) => "形状エラー：{0}",
            Error::SyntaxError(_) => "構文エラー：{0}",
            Error::AmbiguousUnit { .. } => "単位 `{0}` は曖昧です（{1}）：{2}",
            Error::MalformedUnit(_, _) => "単位文字列 `{0}` の書式が不正です：{1}",
            Error::MatrixScalarMix(_) => "行列とスカラーはこの形で組み合わせられません：{0}",
            Error::ExponentNotHalfInteger(_) => "指数 `{0}` は 1/2 の整数倍ではないため、次元が定義できません",
            Error::NotEmittable(_) => "この式はコードとして出力できません：{0}",
            Error::UnsupportedOperation(_) => "未対応の操作です：{0}",
            Error::IoError(_) => "I/O エラー：{0}",
            Error::MatrixCoefficient(_) => "式中の `{0}` の係数が行列です：スカラー連立方程式を解くか inv(A) * b と書いてください",
            Error::FormulaLabel(_) => "{0} 番目の式",
            Error::DeclarationUnit { .. } => "宣言 `{0}` の単位が不正です：{1}",
            Error::ResultDimensionMismatch { .. } =>
                "式 `{0}` の結果の次元は {1} ですが、名前は {2} として宣言されています（{3} 行目）",
            _ => self.template_en(),
        }
    }

    fn template_ko(&self) -> &'static str {
        match self {
            Error::UndeclaredSymbol(_) => "선언되지 않은 기호 `{0}`: 모든 기호는 차원을 선언해야 합니다",
            Error::DuplicateDeclaration { .. } => "기호 `{0}` 가 중복 선언되었습니다{1}: 같은 수식 세트에서 덮어쓸 수 없습니다",
            Error::ReservedName { .. } => "`{0}` 는 예약어라 선언할 수 없습니다({1})",
            Error::UnknownFunction { .. } => "알 수 없는 함수 `{0}`",
            Error::FunctionArity { .. } => "함수 `{0}` 는 인자 {1}개를 받지만 {2}개가 주어졌습니다",
            Error::EmptyFormula => "빈 수식입니다(최소 하나 필요)",
            Error::NestingTooDeep { .. } => "중첩이 너무 깊습니다({0} > 한계 {1}): 수식을 나누세요",
            Error::InvisibleCharacter(_) => "보이지 않는 문자 U+{0} 가 있습니다: 검토 내용과 실제가 달라질 수 있습니다",
            Error::UnknownUnit(_) => "알 수 없는 단위 `{0}`",
            Error::OffsetUnit(_) => "오프셋 단위 `{0}` 는 지원하지 않습니다: 이 체계는 곱셈 차원만 다루므로 K 를 쓰고 수식 밖에서 보정하세요",
            Error::LogUnit(_) => "로그 단위 `{0}` 는 지원하지 않습니다: 차원이 아니라 비율입니다",
            Error::DimensionMismatch { .. } => "차원이 일치하지 않습니다: {0} vs {1}",
            Error::ExponentNotDimensionless(_) => "지수는 무차원이어야 합니다({0})",
            Error::ShapeMismatch(_) => "형상이 일치하지 않습니다: {0}",
            Error::MatrixDivisionUnsupported(_) => "행렬 나눗셈은 지원하지 않습니다: inv(A) * b 또는 solve(A, b) 를 쓰세요",
            Error::NonlinearEquation { .. } => "수식이 `{0}` 에 대해 선형이 아닙니다: {1}",
            Error::Underdetermined(_) => "연립방정식이 `{0}` 에 대해 계수 부족(미정): 해가 유일하지 않습니다",
            Error::Inconsistent => "연립방정식이 모순입니다(소거 후 0 = 0 아님): 해 없음",
            Error::SingularMatrix { .. } => "행렬이 특이합니다(열 {0})",
            Error::IllConditioned { .. } => "행렬의 조건수가 너무 큽니다(rcond={0}, 요구 >= {1})",
            Error::UnstableResidual { .. } => "잔차가 너무 큽니다(상대 {0}, 한계 {1})",
            Error::NoRealRoots { .. } => "실근이 없습니다(판별식 {0}): 복소근이 필요하면 `j` 를 명시하세요",
            Error::NonFiniteValue(_) => "유한하지 않은 값: {0}",
            Error::InvalidInput(_) => "입력이 올바르지 않습니다: {0}",
            Error::ShapeError(_) => "형상 오류: {0}",
            Error::SyntaxError(_) => "구문 오류: {0}",
            Error::AmbiguousUnit { .. } => "단위 `{0}` 가 모호합니다({1}): {2}",
            Error::MalformedUnit(_, _) => "단위 문자열 `{0}` 형식이 잘못되었습니다: {1}",
            Error::MatrixScalarMix(_) => "행렬과 스칼라는 이렇게 결합할 수 없습니다: {0}",
            Error::ExponentNotHalfInteger(_) => "지수 `{0}` 가 1/2 의 정수배가 아니어서 차원을 정의할 수 없습니다",
            Error::NotEmittable(_) => "이 식은 코드로 내보낼 수 없습니다: {0}",
            Error::UnsupportedOperation(_) => "지원하지 않는 연산입니다: {0}",
            Error::IoError(_) => "I/O 오류: {0}",
            Error::MatrixCoefficient(_) => "`{0}` 의 계수가 행렬입니다: 스칼라 연립방정식을 풀거나 inv(A) * b 를 쓰세요",
            Error::FormulaLabel(_) => "{0}번째 수식",
            Error::DeclarationUnit { .. } => "선언 `{0}` 의 단위가 잘못되었습니다: {1}",
            Error::ResultDimensionMismatch { .. } =>
                "수식 `{0}` 의 결과 차원은 {1} 이지만 이름은 {2} 으로 선언되어 있습니다({3}행)",
            _ => self.template_en(),
        }
    }

    fn template_fr(&self) -> &'static str {
        match self {
            Error::UndeclaredSymbol(_) => "symbole non déclaré `{0}` : chaque symbole doit déclarer sa dimension",
            Error::DuplicateDeclaration { .. } => "le symbole `{0}` est déclaré plusieurs fois{1} : un ensemble ne doit pas écraser",
            Error::ReservedName { .. } => "`{0}` est réservé ({1}) et ne peut pas être déclaré",
            Error::UnknownFunction { .. } => "fonction inconnue `{0}`",
            Error::FunctionArity { .. } => "la fonction `{0}` attend {1} argument(s), {2} reçu(s)",
            Error::EmptyFormula => "aucune formule dans l'ensemble (au moins une requise)",
            Error::NestingTooDeep { .. } => "imbrication trop profonde ({0} > limite {1}) : découpez la formule",
            Error::InvisibleCharacter(_) => "caractère invisible U+{0} : il peut masquer ce qui est relu",
            Error::UnknownUnit(_) => "unité inconnue `{0}`",
            Error::OffsetUnit(_) => "unité à décalage `{0}` non prise en charge : cette algèbre est multiplicative, utilisez K et décalez hors de la formule",
            Error::LogUnit(_) => "unité logarithmique `{0}` non prise en charge : c'est un rapport, pas une dimension",
            Error::DimensionMismatch { .. } => "dimensions incompatibles : {0} vs {1}",
            Error::ExponentNotDimensionless(_) => "l'exposant doit être sans dimension, obtenu {0}",
            Error::ShapeMismatch(_) => "formes incompatibles : {0}",
            Error::MatrixDivisionUnsupported(_) => "division matricielle non prise en charge : utilisez inv(A) * b ou solve(A, b)",
            Error::NonlinearEquation { .. } => "l'équation n'est pas linéaire en `{0}` : {1}",
            Error::Underdetermined(_) => "système sous-déterminé pour `{0}` : la solution n'est pas unique",
            Error::Inconsistent => "système incohérent : l'élimination donne 0 = non nul",
            Error::SingularMatrix { .. } => "matrice singulière (colonne {0})",
            Error::IllConditioned { .. } => "matrice trop mal conditionnée (rcond={0}, requis >= {1})",
            Error::UnstableResidual { .. } => "résidu trop grand (relatif {0}, limite {1})",
            Error::NoRealRoots { .. } => "aucune racine réelle (discriminant {0}) : écrivez `j` pour la voie complexe",
            Error::NonFiniteValue(_) => "valeur non finie : {0}",
            Error::InvalidInput(_) => "entrée invalide : {0}",
            Error::ShapeError(_) => "erreur de forme : {0}",
            Error::SyntaxError(_) => "erreur de syntaxe : {0}",
            Error::AmbiguousUnit { .. } => "l'unité `{0}` est ambiguë ({1}) : {2}",
            Error::MalformedUnit(_, _) => "unité mal formée `{0}` : {1}",
            Error::MatrixScalarMix(_) => "matrice et scalaire ne se combinent pas ici : {0}",
            Error::ExponentNotHalfInteger(_) => "exposant `{0}` non multiple de 1/2 : la dimension n'est pas définie",
            Error::NotEmittable(_) => "cette expression ne peut pas être émise : {0}",
            Error::UnsupportedOperation(_) => "opération non prise en charge : {0}",
            Error::IoError(_) => "erreur d'E/S : {0}",
            Error::MatrixCoefficient(_) => "le coefficient de `{0}` est une matrice : résolvez un système scalaire ou écrivez inv(A) * b",
            Error::FormulaLabel(_) => "formule n° {0}",
            Error::DeclarationUnit { .. } => "la déclaration `{0}` a une unité invalide : {1}",
            Error::ResultDimensionMismatch { .. } =>
                "la formule `{0}` produit {1} mais son nom est déclaré en {2} (ligne {3})",
            _ => self.template_en(),
        }
    }

    fn template_es(&self) -> &'static str {
        match self {
            Error::UndeclaredSymbol(_) => "símbolo no declarado `{0}`: cada símbolo debe declarar su dimensión",
            Error::DuplicateDeclaration { .. } => "el símbolo `{0}` está declarado más de una vez{1}: un conjunto no debe sobrescribir",
            Error::ReservedName { .. } => "`{0}` está reservado ({1}) y no puede declararse",
            Error::UnknownFunction { .. } => "función desconocida `{0}`",
            Error::FunctionArity { .. } => "la función `{0}` toma {1} argumento(s), se dieron {2}",
            Error::EmptyFormula => "no hay ninguna fórmula (se requiere al menos una)",
            Error::NestingTooDeep { .. } => "anidamiento demasiado profundo ({0} > límite {1}): divida la fórmula",
            Error::InvisibleCharacter(_) => "carácter invisible U+{0}: puede ocultar lo que se revisa",
            Error::UnknownUnit(_) => "unidad desconocida `{0}`",
            Error::OffsetUnit(_) => "unidad con desplazamiento `{0}` no soportada: este álgebra es multiplicativa, use K y desplace fuera de la fórmula",
            Error::LogUnit(_) => "unidad logarítmica `{0}` no soportada: es una razón, no una dimensión",
            Error::DimensionMismatch { .. } => "dimensiones incompatibles: {0} vs {1}",
            Error::ExponentNotDimensionless(_) => "el exponente debe ser adimensional, se obtuvo {0}",
            Error::ShapeMismatch(_) => "formas incompatibles: {0}",
            Error::MatrixDivisionUnsupported(_) => "división matricial no soportada: use inv(A) * b o solve(A, b)",
            Error::NonlinearEquation { .. } => "la ecuación no es lineal en `{0}`: {1}",
            Error::Underdetermined(_) => "sistema indeterminado para `{0}`: la solución no es única",
            Error::Inconsistent => "sistema incompatible: la eliminación produce 0 = distinto de cero",
            Error::SingularMatrix { .. } => "matriz singular (columna {0})",
            Error::IllConditioned { .. } => "matriz demasiado mal condicionada (rcond={0}, requerido >= {1})",
            Error::UnstableResidual { .. } => "residuo demasiado grande (relativo {0}, límite {1})",
            Error::NoRealRoots { .. } => "sin raíces reales (discriminante {0}): escriba `j` para la vía compleja",
            Error::NonFiniteValue(_) => "valor no finito: {0}",
            Error::InvalidInput(_) => "entrada inválida: {0}",
            Error::ShapeError(_) => "error de forma: {0}",
            Error::SyntaxError(_) => "error de sintaxis: {0}",
            Error::AmbiguousUnit { .. } => "la unidad `{0}` es ambigua ({1}): {2}",
            Error::MalformedUnit(_, _) => "unidad mal formada `{0}`: {1}",
            Error::MatrixScalarMix(_) => "matriz y escalar no se combinan así: {0}",
            Error::ExponentNotHalfInteger(_) => "exponente `{0}` no es múltiplo de 1/2: la dimensión no está definida",
            Error::NotEmittable(_) => "esta expresión no puede emitirse: {0}",
            Error::UnsupportedOperation(_) => "operación no soportada: {0}",
            Error::IoError(_) => "error de E/S: {0}",
            Error::MatrixCoefficient(_) => "el coeficiente de `{0}` es una matriz: resuelva un sistema escalar o escriba inv(A) * b",
            Error::FormulaLabel(_) => "fórmula n.º {0}",
            Error::DeclarationUnit { .. } => "la declaración `{0}` tiene una unidad no válida: {1}",
            Error::ResultDimensionMismatch { .. } =>
                "la fórmula `{0}` produce {1} pero su nombre está declarado como {2} (línea {3})",
            _ => self.template_en(),
        }
    }

    fn template_ru(&self) -> &'static str {
        match self {
            Error::UndeclaredSymbol(_) => "необъявленный символ `{0}`: у каждого символа должна быть объявлена размерность",
            Error::DuplicateDeclaration { .. } => "символ `{0}` объявлен более одного раза{1}: перезапись в наборе запрещена",
            Error::ReservedName { .. } => "`{0}` зарезервировано ({1}) и не может быть объявлено",
            Error::UnknownFunction { .. } => "неизвестная функция `{0}`",
            Error::FunctionArity { .. } => "функция `{0}` принимает {1} аргумент(ов), передано {2}",
            Error::EmptyFormula => "в наборе нет ни одной формулы (нужна хотя бы одна)",
            Error::NestingTooDeep { .. } => "слишком глубокая вложенность ({0} > предел {1}): разделите формулу",
            Error::InvisibleCharacter(_) => "невидимый символ U+{0}: он может скрыть то, что проверяют",
            Error::UnknownUnit(_) => "неизвестная единица `{0}`",
            Error::OffsetUnit(_) => "единица со смещением `{0}` не поддерживается: эта алгебра мультипликативна, используйте K и смещайте вне формулы",
            Error::LogUnit(_) => "логарифмическая единица `{0}` не поддерживается: это отношение, а не размерность",
            Error::DimensionMismatch { .. } => "несовпадение размерностей: {0} vs {1}",
            Error::ExponentNotDimensionless(_) => "показатель должен быть безразмерным, получено {0}",
            Error::ShapeMismatch(_) => "несовпадение форм: {0}",
            Error::MatrixDivisionUnsupported(_) => "деление матриц не поддерживается: используйте inv(A) * b или solve(A, b)",
            Error::NonlinearEquation { .. } => "уравнение нелинейно относительно `{0}`: {1}",
            Error::Underdetermined(_) => "система недоопределена по `{0}`: решение не единственно",
            Error::Inconsistent => "система противоречива: после исключения получено 0 = ненулевое",
            Error::SingularMatrix { .. } => "матрица вырождена (столбец {0})",
            Error::IllConditioned { .. } => "матрица слишком плохо обусловлена (rcond={0}, требуется >= {1})",
            Error::UnstableResidual { .. } => "невязка слишком велика (относительная {0}, предел {1})",
            Error::NoRealRoots { .. } => "нет действительных корней (дискриминант {0}): укажите `j` для комплексной ветви",
            Error::NonFiniteValue(_) => "неконечное значение: {0}",
            Error::InvalidInput(_) => "недопустимый ввод: {0}",
            Error::ShapeError(_) => "ошибка формы: {0}",
            Error::SyntaxError(_) => "синтаксическая ошибка: {0}",
            Error::AmbiguousUnit { .. } => "единица `{0}` неоднозначна ({1}): {2}",
            Error::MalformedUnit(_, _) => "некорректная единица `{0}`: {1}",
            Error::MatrixScalarMix(_) => "матрица и скаляр так не сочетаются: {0}",
            Error::ExponentNotHalfInteger(_) => "показатель `{0}` не кратен 1/2, размерность не определена",
            Error::NotEmittable(_) => "выражение невозможно сгенерировать: {0}",
            Error::UnsupportedOperation(_) => "неподдерживаемая операция: {0}",
            Error::IoError(_) => "ошибка ввода-вывода: {0}",
            Error::MatrixCoefficient(_) => "коэффициент при `{0}` — матрица: решайте скалярную систему или пишите inv(A) * b",
            Error::FormulaLabel(_) => "формула № {0}",
            Error::DeclarationUnit { .. } => "в объявлении `{0}` неверная единица: {1}",
            Error::ResultDimensionMismatch { .. } =>
                "формула `{0}` даёт {1}, а её имя объявлено как {2} (строка {3})",
            _ => self.template_en(),
        }
    }

    fn template_ar(&self) -> &'static str {
        match self {
            Error::UndeclaredSymbol(_) => "رمز غير معلن `{0}`: يجب إعلان البُعد لكل رمز",
            Error::DuplicateDeclaration { .. } => {
                "الرمز `{0}` معلن أكثر من مرة (السطر {1}): لا يُسمح بالاستبدال في المجموعة"
            }
            Error::ReservedName { .. } => "`{0}` محجوز ({1}) ولا يمكن إعلانه",
            Error::UnknownFunction { .. } => "دالة غير معروفة `{0}`",
            Error::FunctionArity { .. } => "الدالة `{0}` تأخذ {1} وسيطًا، وتم تمرير {2}",
            Error::EmptyFormula => "لا توجد أي صيغة (مطلوبة واحدة على الأقل)",
            Error::NestingTooDeep { .. } => "التداخل عميق جدًا ({0} > الحد {1}): قسّم الصيغة",
            Error::InvisibleCharacter(_) => "محرف غير مرئي U+{0}: قد يخفي ما تتم مراجعته",
            Error::UnknownUnit(_) => "وحدة غير معروفة `{0}`",
            Error::OffsetUnit(_) => {
                "وحدة بإزاحة `{0}` غير مدعومة: هذا الجبر ضربي، استخدم K وأزح خارج الصيغة"
            }
            Error::LogUnit(_) => "وحدة لوغاريتمية `{0}` غير مدعومة: إنها نسبة لا بُعد",
            Error::DimensionMismatch { .. } => "عدم توافق في الأبعاد: {0} مقابل {1}",
            Error::ExponentNotDimensionless(_) => "يجب أن يكون الأس بلا بُعد، والنتيجة {0}",
            Error::ShapeMismatch(_) => "عدم توافق في الأشكال: {0}",
            Error::MatrixDivisionUnsupported(_) => {
                "قسمة المصفوفات غير مدعومة: استخدم inv(A) * b أو solve(A, b)"
            }
            Error::NonlinearEquation { .. } => "المعادلة ليست خطية بالنسبة إلى `{0}`: {1}",
            Error::Underdetermined(_) => "النظام ناقص التحديد بالنسبة إلى `{0}`: الحل غير وحيد",
            Error::Inconsistent => "النظام متناقض: أنتج الحذف 0 = قيمة غير صفرية",
            Error::SingularMatrix { .. } => "المصفوفة شاذة (العمود {0})",
            Error::IllConditioned { .. } => "المصفوفة سيئة الاشتراط جدًا (rcond={0}، المطلوب >= {1})",
            Error::UnstableResidual { .. } => "الباقي كبير جدًا (نسبي {0}، الحد {1})",
            Error::NoRealRoots { .. } => "لا توجد جذور حقيقية (المميز {0}): اكتب `j` للمسار العقدي",
            Error::NonFiniteValue(_) => "قيمة غير منتهية: {0}",
            Error::InvalidInput(_) => "مدخل غير صالح: {0}",
            Error::ShapeError(_) => "خطأ في الشكل: {0}",
            Error::SyntaxError(_) => "خطأ في الصياغة: {0}",
            Error::AmbiguousUnit { .. } => "الوحدة `{0}` ملتبسة ({1}): {2}",
            Error::MalformedUnit(_, _) => "وحدة غير صحيحة `{0}`: {1}",
            Error::MatrixScalarMix(_) => "لا يمكن جمع المصفوفة والعدد بهذه الطريقة: {0}",
            Error::ExponentNotHalfInteger(_) => {
                "الأُس `{0}` ليس من مضاعفات 1/2، لذا لا يمكن تعريف البُعد"
            }
            Error::NotEmittable(_) => "لا يمكن توليد تعليمات برمجية من هذا التعبير: {0}",
            Error::UnsupportedOperation(_) => "عملية غير مدعومة: {0}",
            Error::IoError(_) => "خطأ في الإدخال/الإخراج: {0}",
            Error::MatrixCoefficient(_) => "معامل `{0}` مصفوفة: حلّ نظامًا قياسيًا أو اكتب inv(A) * b",
            Error::FormulaLabel(_) => "الصيغة رقم {0}",
            Error::DeclarationUnit { .. } => "الوحدة في إعلان `{0}` غير صحيحة: {1}",
            Error::ResultDimensionMismatch { .. } => {
                "الصيغة `{0}` تنتج {1} بينما اسمها معلن بـ {2} (السطر {3})"
            }
            _ => self.template_en(),
        }
    }

    /// The template for this code in `locale`.
    pub fn template(&self, locale: Locale) -> &'static str {
        match locale {
            Locale::En => self.template_en(),
            Locale::ZhHans => self.template_zh_hans(),
            Locale::ZhHant => self.template_zh_hant(),
            Locale::Ja => self.template_ja(),
            Locale::Ko => self.template_ko(),
            Locale::Fr => self.template_fr(),
            Locale::Es => self.template_es(),
            Locale::Ru => self.template_ru(),
            Locale::Ar => self.template_ar(),
        }
    }
}

/// Renders `code` in `locale`, substituting `{0}`, `{1}`, ... with `args`.
///
/// Placeholders without a matching argument are left as-is rather than replaced with an
/// empty string: a visible `{2}` in output is a bug report, a silently dropped value is not.
pub fn render(code: &Error, args: &[&str], locale: Locale) -> String {
    if let Error::Message(text) = code {
        return text.clone();
    }
    // The spelling suggestion is advice, not part of the failure sentence: it is appended only
    // when a real near-match exists, in the locale's own phrasing.
    if let Error::UnknownFunction { suggestion, .. } = code {
        if !suggestion.is_empty() {
            let tail = match locale {
                Locale::En => format!("; did you mean `{suggestion}`?"),
                Locale::ZhHans => format!("；你是不是想写 `{suggestion}`？"),
                Locale::ZhHant => format!("；你是不是想寫 `{suggestion}`？"),
                Locale::Ja => format!("：`{suggestion}` ではないですか？"),
                Locale::Ko => format!(": `{suggestion}` 을(를) 의도하셨습니까?"),
                Locale::Fr => format!(" : vouliez-vous `{suggestion}` ?"),
                Locale::Es => format!(": ¿quiso decir `{suggestion}`?"),
                Locale::Ru => format!("; может быть, `{suggestion}`?"),
                Locale::Ar => format!("؛ هل تقصد `{suggestion}`؟"),
            };
            return format!("{}{tail}", basic_render(code, &[], locale));
        }
    }
    basic_render(code, args, locale)
}

/// Sentence-only rendering: template + placeholder substitution, no cross-variant adornments.
fn basic_render(code: &Error, args: &[&str], locale: Locale) -> String {
    if let Error::ReservedName { name, reason } = code {
        let why = match (reason, locale) {
            (ReservedReason::BuiltinFunction, Locale::En) => {
                "also a built-in function; rename the declaration"
            }
            (ReservedReason::BuiltinFunction, Locale::ZhHans) => "也是内置函数名，请换一个声明名",
            (ReservedReason::BuiltinFunction, Locale::ZhHant) => "也是內建函式名，請換一個宣告名",
            (ReservedReason::BuiltinFunction, Locale::Ja) => {
                "組み込み関数と同名です；宣言名を変えてください"
            }
            (ReservedReason::BuiltinFunction, Locale::Ko) => {
                "내장 함수와 같은 이름입니다; 선언 이름을 바꾸세요"
            }
            (ReservedReason::BuiltinFunction, Locale::Fr) => {
                "aussi une fonction intégrée ; renommez la déclaration"
            }
            (ReservedReason::BuiltinFunction, Locale::Es) => {
                "también es una función integrada; cambie el nombre de la declaración"
            }
            (ReservedReason::BuiltinFunction, Locale::Ru) => {
                "совпадает со встроенной функцией; переименуйте объявление"
            }
            (ReservedReason::BuiltinFunction, Locale::Ar) => "هو أيضًا دالة مدمجة؛ غيّر اسم الإعلان",
            (ReservedReason::ImaginaryUnit, Locale::En) => {
                "reserved for the imaginary unit `j`; pick another name"
            }
            (ReservedReason::ImaginaryUnit, Locale::ZhHans) => "保留给虚数单位 `j`，请换一个名字",
            (ReservedReason::ImaginaryUnit, Locale::ZhHant) => "保留給虛數單位 `j`，請換一個名字",
            (ReservedReason::ImaginaryUnit, Locale::Ja) => {
                "虚数単位 `j` のために予約されています；別の名前を選んでください"
            }
            (ReservedReason::ImaginaryUnit, Locale::Ko) => {
                "허수 단위 `j` 를 위해 예약되어 있습니다; 다른 이름을 고르세요"
            }
            (ReservedReason::ImaginaryUnit, Locale::Fr) => {
                "réservé à l'unité imaginaire `j` ; choisissez un autre nom"
            }
            (ReservedReason::ImaginaryUnit, Locale::Es) => {
                "reservado para la unidad imaginaria `j`; elija otro nombre"
            }
            (ReservedReason::ImaginaryUnit, Locale::Ru) => {
                "зарезервировано для мнимой единицы `j`; выберите другое имя"
            }
            (ReservedReason::ImaginaryUnit, Locale::Ar) => {
                "محجوز للوحدة التخيلية `j`؛ اختر اسمًا آخر"
            }
        };
        let sentence = match locale {
            Locale::En => format!("`{name}` is reserved ({why}) and cannot be declared"),
            Locale::ZhHans => format!("`{name}` 是保留名（{why}），不能声明"),
            Locale::ZhHant => format!("`{name}` 是保留名（{why}），不能宣告"),
            Locale::Ja => format!("`{name}` は予約名のため宣言できません（{why}）"),
            Locale::Ko => format!("`{name}` 는 예약어라 선언할 수 없습니다({why})"),
            Locale::Fr => format!("`{name}` est réservé ({why}) et ne peut pas être déclaré"),
            Locale::Es => format!("`{name}` está reservado ({why}) y no puede declararse"),
            Locale::Ru => format!("`{name}` зарезервировано ({why}) и не может быть объявлено"),
            Locale::Ar => format!("`{name}` محجوز ({why}) ولا يمكن إعلانه"),
        };
        return sentence;
    }
    if let Error::DuplicateDeclaration { line, .. } = code {
        let line = *line;
        // Localize the optional position: " (line 3)" / "（第 3 行）" / "" when unknown.
        let suffix = match line {
            None => String::new(),
            Some(l) => match locale {
                Locale::En => format!(" (line {l})"),
                Locale::ZhHans => format!("（第 {l} 行）"),
                Locale::ZhHant => format!("（第 {l} 行）"),
                Locale::Ja => format!("（{l} 行目）"),
                Locale::Ko => format!("({l}행)"),
                Locale::Fr => format!(" (ligne {l})"),
                Locale::Es => format!(" (línea {l})"),
                Locale::Ru => format!(" (строка {l})"),
                Locale::Ar => format!(" (السطر {l})"),
            },
        };
        let name = code.arguments()[0].clone();
        let sentence = match locale {
            Locale::En => format!("symbol `{name}` is declared more than once{suffix}; a formula set must not overwrite"),
            Locale::ZhHans => format!("符号 `{name}` 被重复声明{suffix}：同一公式集不允许覆盖"),
            Locale::ZhHant => format!("符號 `{name}` 被重複宣告{suffix}：同一公式集不允許覆蓋"),
            Locale::Ja => format!("シンボル `{name}` が重複して宣言されています{suffix}：同じ式セットでの上書きはできません"),
            Locale::Ko => format!("기호 `{name}` 가 중복 선언되었습니다{suffix}: 같은 수식 세트에서 덮어쓸 수 없습니다"),
            Locale::Fr => format!("le symbole `{name}` est déclaré plusieurs fois{suffix} : un ensemble ne doit pas écraser"),
            Locale::Es => format!("el símbolo `{name}` está declarado más de una vez{suffix}: un conjunto no debe sobrescribir"),
            Locale::Ru => format!("символ `{name}` объявлен более одного раза{suffix}: перезапись в наборе запрещена"),
            Locale::Ar => format!("الرمز `{name}` معلن أكثر من مرة{suffix}: لا يُسمح بالاستبدال في المجموعة"),
        };
        return sentence;
    }
    if let Error::DeclarationUnit { name, unit } = code {
        // The inner unit error must render in the *requested* locale, not English.
        return match locale {
            Locale::ZhHans => format!("声明 `{name}` 的单位错：{}", unit.render(locale)),
            Locale::ZhHant => format!("宣告 `{name}` 的單位錯：{}", unit.render(locale)),
            Locale::Ja => format!("宣言 `{name}` の単位が不正です：{}", unit.render(locale)),
            Locale::Ko => format!(
                "선언 `{name}` 의 단위가 잘못되었습니다: {}",
                unit.render(locale)
            ),
            Locale::Fr => format!(
                "la déclaration `{name}` a une unité invalide : {}",
                unit.render(locale)
            ),
            Locale::Es => format!(
                "la declaración `{name}` tiene una unidad no válida: {}",
                unit.render(locale)
            ),
            Locale::Ru => format!(
                "в объявлении `{name}` неверная единица: {}",
                unit.render(locale)
            ),
            Locale::Ar => format!("الوحدة في إعلان `{name}` غير صحيحة: {}", unit.render(locale)),
            Locale::En => format!("declaration `{name}`: {}", unit.render(locale)),
        };
    }
    let owned = code.arguments();
    let effective: Vec<&str> = if args.is_empty() {
        owned.iter().map(|s| s.as_str()).collect()
    } else {
        args.to_vec()
    };
    let args = effective.as_slice();
    let template = code.template(locale);
    let mut out = String::with_capacity(template.len() + 16);
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '{' {
            out.push(ch);
            continue;
        }
        let mut digits = String::new();
        let mut closed = false;
        for next in chars.by_ref() {
            if next == '}' {
                closed = true;
                break;
            }
            digits.push(next);
        }
        match digits.parse::<usize>() {
            Ok(index) if closed => match args.get(index) {
                Some(value) => out.push_str(value),
                None => {
                    out.push('{');
                    out.push_str(&digits);
                    out.push('}');
                }
            },
            _ => {
                out.push('{');
                out.push_str(&digits);
                if closed {
                    out.push('}');
                }
            }
        }
    }
    out
}

/// Convenience for building the positional arguments of [`render`].
pub fn args(items: &[&str]) -> String {
    items.join("\u{1f}")
}

/// Formats a float for messages without dragging in `format!` precision surprises.
pub fn num(value: f64) -> String {
    format!("{value}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_locale_renders_every_code_without_leaking_placeholders() {
        // A code without a translation falls back to English; that is allowed. What is
        // NOT allowed is an empty string or a leftover `{0}` when an argument was given.
        for code in [
            Error::UndeclaredSymbol("x".into()),
            Error::DimensionMismatch {
                left: "kg".into(),
                right: "m".into(),
            },
            Error::NoRealRoots { discriminant: -4.0 },
            Error::SingularMatrix { column: 2 },
            Error::IllConditioned {
                rcond: 1e-18,
                limit: 1e-14,
            },
            Error::UnknownUnit("N".into()),
            Error::SyntaxError("bad token".into()),
        ] {
            for locale in Locale::ALL {
                let text = render(&code, &["X", "Y", "Z"], locale);
                assert!(!text.trim().is_empty(), "{code:?}/{locale:?} is empty");
                assert!(
                    !text.contains("{0}"),
                    "{code:?}/{locale:?} left {{0}}: {text}"
                );
                assert!(
                    !text.contains("{1}"),
                    "{code:?}/{locale:?} left {{1}}: {text}"
                );
            }
        }
    }

    #[test]
    fn a_missing_argument_stays_visible_instead_of_vanishing() {
        // Fields drive the substitution: the variant supplies its own arguments.
        let err = Error::FunctionArity {
            name: "f".into(),
            expected: 2,
            got: 3,
        };
        let text = render(&err, &[], Locale::En);
        assert!(
            text.contains('f') && text.contains('2') && text.contains('3'),
            "{text}"
        );

        // When a caller passes fewer explicit arguments than the template needs, the missing
        // one stays visible as `{n}` rather than silently collapsing to empty.
        let text = render(
            &Error::DimensionMismatch {
                left: "kg".into(),
                right: "m".into(),
            },
            &["kg"],
            Locale::En,
        );
        assert!(text.contains("{1}"), "{text}");
    }

    #[test]
    fn locale_tags_round_trip_and_accept_short_spellings() {
        for locale in Locale::ALL {
            assert_eq!(Locale::parse(locale.tag()), Some(locale), "{locale:?}");
        }
        assert_eq!(Locale::parse("zh-CN"), Some(Locale::ZhHans));
        assert_eq!(Locale::parse("zh_TW"), Some(Locale::ZhHant));
        assert_eq!(Locale::parse("zh"), Some(Locale::ZhHans));
        assert_eq!(Locale::parse("de"), None, "German is not one of the nine");
        assert_eq!(Locale::parse(""), None);
        assert_eq!(Locale::default(), Locale::En);
    }

    #[test]
    fn codes_have_unique_stable_spellings() {
        let codes = [
            Error::SyntaxError("x".into()),
            Error::UndeclaredSymbol("x".into()),
            Error::DimensionMismatch {
                left: "kg".into(),
                right: "m".into(),
            },
            Error::NoRealRoots { discriminant: -4.0 },
        ];
        let mut seen = alloc::vec::Vec::new();
        for code in codes {
            let tag = code.code();
            assert!(!seen.contains(&tag), "duplicate code `{tag}`");
            seen.push(tag);
            assert!(!tag.is_empty());
            assert!(
                tag.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "code `{tag}` must be kebab-case"
            );
        }
    }
}
#[cfg(test)]
mod localization_tests {
    use crate::i18n::{Error, Locale};
    use crate::FrontError;

    #[test]
    fn a_coded_error_re_renders_from_its_kept_arguments() {
        let err = FrontError::coded(
            Error::FunctionArity {
                name: "f".into(),
                expected: 2,
                got: 3,
            },
            &[],
            7,
            4,
        );
        assert!(err.msg.contains("f"), "{}", err.msg);
        assert_eq!(err.code.code(), "function-arity");
        for locale in Locale::ALL {
            let text = err.render_in(locale);
            assert!(text.contains('f'), "{locale:?}: {text}");
            assert!(
                !text.contains("{1}"),
                "{locale:?} left a placeholder: {text}"
            );
        }
        let zh = err.in_locale(Locale::ZhHans);
        assert!(zh.msg.contains("函数"), "{}", zh.msg);
        assert_eq!(
            zh.code.code(),
            err.code.code(),
            "localization must not change the code"
        );
    }

    #[test]
    fn an_uncatalogued_error_keeps_its_original_text_in_every_locale() {
        let legacy = FrontError::plain("legacy message");
        // A raw string becomes the staging variant, not the `Raw` code: both render verbatim,
        // but `Message` is the one the migration counts.
        assert_eq!(*legacy.code, Error::Message("legacy message".to_string()));
        for locale in Locale::ALL {
            assert_eq!(legacy.render_in(locale), "legacy message");
        }
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.render(Locale::En))
    }
}

impl core::error::Error for Error {}

impl From<String> for Error {
    fn from(text: String) -> Self {
        Error::Message(text)
    }
}

impl From<&str> for Error {
    fn from(text: &str) -> Self {
        Error::Message(String::from(text))
    }
}

impl From<core::fmt::Arguments<'_>> for Error {
    fn from(args: core::fmt::Arguments<'_>) -> Self {
        Error::Message(format!("{args}"))
    }
}

impl From<crate::FrontError> for Error {
    fn from(err: crate::FrontError) -> Self {
        *err.code
    }
}

impl Error {
    /// Positional arguments taken from the variant's own fields, in template order.
    pub fn arguments(&self) -> Vec<String> {
        match self {
            Error::UndeclaredSymbol(name) => vec![name.clone()],
            Error::FunctionArity {
                name,
                expected,
                got,
            } => vec![name.clone(), expected.to_string(), got.to_string()],
            Error::NestingTooDeep { got, limit } => vec![got.clone(), limit.clone()],
            Error::InvisibleCharacter(what) => vec![what.clone()],
            Error::UnknownUnit(name) => vec![name.clone()],
            Error::OffsetUnit(name) => vec![name.clone()],
            Error::LogUnit(name) => vec![name.clone()],
            Error::AmbiguousUnit { name, why, remedy } => {
                vec![name.clone(), why.clone(), remedy.clone()]
            }
            Error::DimensionMismatch { left, right } => vec![left.clone(), right.clone()],
            Error::ShapeError(what) => vec![what.clone()],
            Error::SyntaxError(what) => vec![what.clone()],
            Error::FormulaLabel(k) => vec![k.to_string()],
            Error::DeclarationUnit { name, unit } => {
                vec![name.clone(), unit.render(Locale::En)]
            }
            Error::ResultDimensionMismatch {
                name,
                got,
                declared,
                line,
            } => vec![
                name.clone(),
                got.clone(),
                declared.clone(),
                line.to_string(),
            ],
            Error::MalformedUnit(what, why) => vec![what.clone(), why.clone()],
            Error::ExponentNotDimensionless(got)
            | Error::ExponentNotHalfInteger(got)
            | Error::ShapeMismatch(got)
            | Error::MatrixScalarMix(got)
            | Error::Underdetermined(got)
            | Error::MatrixCoefficient(got)
            | Error::NotEmittable(got)
            | Error::UnsupportedOperation(got)
            | Error::IoError(got) => vec![got.clone()],
            Error::UnknownFunction { name, suggestion } => {
                vec![name.clone(), suggestion.clone()]
            }
            Error::ReservedName { name, reason } => vec![
                name.clone(),
                match reason {
                    ReservedReason::BuiltinFunction => "also a built-in function",
                    ReservedReason::ImaginaryUnit => "the imaginary-unit constant",
                }
                .to_string(),
            ],
            Error::DuplicateDeclaration { name, line } => {
                // The position suffix is pre-phrased per the requested locale at render time;
                // `basic_render` substitutes it, so here we only pass the raw number or "".
                vec![
                    name.clone(),
                    line.map(|l| l.to_string()).unwrap_or_default(),
                ]
            }
            Error::NonlinearEquation { name, why } => vec![name.clone(), why.clone()],
            Error::InvalidInput(what) => vec![what.clone()],
            Error::NonFiniteValue(what) => vec![what.clone()],
            Error::SingularMatrix { column } => vec![column.to_string()],
            Error::IllConditioned { rcond, limit } => {
                vec![format!("{rcond:.6e}"), format!("{limit:.6e}")]
            }
            Error::UnstableResidual { residual, limit } => {
                vec![format!("{residual:.6e}"), format!("{limit:.6e}")]
            }
            Error::NoRealRoots { discriminant } => vec![format!("{discriminant:.6e}")],
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod suggestion_tests {
    use crate::i18n::{Error, Locale};

    /// The suggestion clause must appear **exactly once** per locale, and an empty suggestion
    /// must produce no dangling "did you mean" fragment in any language.
    #[test]
    fn the_suggestion_clause_is_appended_exactly_once_and_only_when_present() {
        let with = Error::UnknownFunction {
            name: "expp".into(),
            suggestion: "exp".into(),
        };
        // Each locale's suggestion clause has a marker punctuation/phrase; count it directly.
        let marker = |locale: Locale| -> &'static str {
            match locale {
                Locale::En => "did you mean",
                Locale::ZhHans => "你是不是想写",
                Locale::ZhHant => "你是不是想寫",
                Locale::Ja => "ではないですか",
                Locale::Ko => "의도하셨습니까",
                Locale::Fr => "vouliez-vous",
                Locale::Es => "quiso decir",
                Locale::Ru => "может быть",
                Locale::Ar => "هل تقصد",
            }
        };
        for locale in Locale::ALL {
            let text = with.render(locale);
            let count = text.matches(marker(locale)).count();
            assert_eq!(count, 1, "{locale:?} clause count {count}: {text}");
            // The suggested name is quoted exactly once as well.
            assert_eq!(
                text.matches("`exp`").count(),
                1,
                "{locale:?} `exp` count: {text}"
            );
        }

        let without = Error::UnknownFunction {
            name: "zzzznotafunc".into(),
            suggestion: String::new(),
        };
        for locale in Locale::ALL {
            let text = without.render(locale);
            assert!(
                !text.contains(marker(locale)),
                "{locale:?} clause leaked: {text}"
            );
            assert!(!text.contains("``"), "{locale:?} empty backticks: {text}");
        }
    }
}

#[cfg(test)]
mod reachability_tests {
    use crate::i18n::{Error, Locale};
    use crate::{dims_of_set, parse_unit, solve_system, Decl, Expr, FormulaSet};

    /// Variants that used to be catalogue-only must now be produced by a real code path.
    #[test]
    fn formerly_dead_variants_are_reachable_from_the_public_api() {
        // empty-formula: a set with declarations but no items
        let empty = FormulaSet {
            decls: vec![Decl {
                name: "x".into(),
                quantity: "length".into(),
                unit: Some("m".into()),
            }],
            items: vec![],
            uses: vec![],
        };
        assert_eq!(
            crate::check_set_depth(&empty).unwrap_err().code.code(),
            "empty-formula"
        );

        // malformed-unit: unbalanced bracket in a unit string
        assert_eq!(parse_unit("A/(").unwrap_err().code(), "malformed-unit");

        // inconsistent: x = 1 and x = 2
        let err = solve_system(
            &[
                Expr::Sub(Box::new(Expr::Var("x".into())), Box::new(Expr::Num(1.0))),
                Expr::Sub(Box::new(Expr::Var("x".into())), Box::new(Expr::Num(2.0))),
            ],
            &["x".to_string()],
        )
        .unwrap_err();
        assert_eq!(err.code(), "inconsistent");

        // duplicate-declaration: two formulas sharing a name
        let dup = FormulaSet {
            decls: vec![Decl {
                name: "d".into(),
                quantity: "length".into(),
                unit: Some("m".into()),
            }],
            items: vec![
                crate::FormulaItem {
                    name: Some("v".into()),
                    expr: Expr::Var("d".into()),
                    line: 2,
                },
                crate::FormulaItem {
                    name: Some("v".into()),
                    expr: Expr::Var("d".into()),
                    line: 3,
                },
            ],
            uses: vec![],
        };
        assert_eq!(
            dims_of_set(&dup).unwrap_err().code(),
            "duplicate-declaration"
        );

        // Every one of them must render in all nine locales without a placeholder leak.
        for err in [
            Error::EmptyFormula,
            Error::MalformedUnit("A/(".into(), "unbalanced `(`".into()),
        ] {
            for locale in Locale::ALL {
                let text = err.render(locale);
                assert!(!text.trim().is_empty(), "{locale:?} empty");
                assert!(!text.contains("{0}"), "{locale:?}: {text}");
            }
        }
    }
}

#[cfg(test)]
mod catalogue_completeness_tests {
    use super::*;

    /// Every variant must be constructible with a realistic payload AND render cleanly in all
    /// nine locales. This is the exhaustive guard the README's reachability claim rests on: a
    /// variant that cannot be listed here is either dead or leaking a placeholder.
    #[test]
    fn every_variant_renders_cleanly_in_every_locale() {
        let all: Vec<Error> = vec![
            Error::SyntaxError("expected factor".into()),
            Error::UndeclaredSymbol("v".into()),
            Error::DuplicateDeclaration {
                name: "v".into(),
                line: Some(3),
            },
            Error::ReservedName {
                name: "j".into(),
                reason: ReservedReason::ImaginaryUnit,
            },
            Error::UnknownFunction {
                name: "expp".into(),
                suggestion: "exp".into(),
            },
            Error::FunctionArity {
                name: "f".into(),
                expected: 2,
                got: 3,
            },
            Error::EmptyFormula,
            Error::NestingTooDeep {
                got: "300".into(),
                limit: "256".into(),
            },
            Error::InvisibleCharacter("200B (zero-width character)".into()),
            Error::UnknownUnit("zz".into()),
            Error::OffsetUnit("degC".into()),
            Error::LogUnit("dBm".into()),
            Error::AmbiguousUnit {
                name: "cal".into(),
                why: "cal_th vs cal_IT".into(),
                remedy: "write J".into(),
            },
            Error::MalformedUnit("A/(".into(), "unbalanced `(`".into()),
            Error::DimensionMismatch {
                left: "kg".into(),
                right: "m".into(),
            },
            Error::ExponentNotDimensionless("s".into()),
            Error::ExponentNotHalfInteger("0.333".into()),
            Error::ShapeMismatch("(2,2) vs (2,1)".into()),
            Error::MatrixScalarMix("matrix vs scalar".into()),
            Error::MatrixDivisionUnsupported("write inv(A) * b".into()),
            Error::NonlinearEquation {
                name: "x".into(),
                why: "twice in one product".into(),
            },
            Error::Underdetermined("y".into()),
            Error::Inconsistent,
            Error::MatrixCoefficient("x".into()),
            Error::SingularMatrix { column: 1 },
            Error::IllConditioned {
                rcond: 1e-18,
                limit: 1e-14,
            },
            Error::UnstableResidual {
                residual: 1e-3,
                limit: 1e-9,
            },
            Error::NoRealRoots { discriminant: -4.0 },
            Error::NonFiniteValue("inf".into()),
            Error::InvalidInput("a must be nonzero".into()),
            Error::ShapeError("(2,2) vs (2,3)".into()),
            Error::NotEmittable("symbolic differential operator".into()),
            Error::UnsupportedOperation("the imaginary unit `j`".into()),
            Error::IoError("cannot read x.formula".into()),
            Error::DeclarationUnit {
                name: "T".into(),
                unit: Box::new(Error::OffsetUnit("degC".into())),
            },
            Error::ResultDimensionMismatch {
                name: "s".into(),
                got: "kg·s^-2".into(),
                declared: "Pa".into(),
                line: 7,
            },
            Error::FormulaLabel(3),
            Error::Message("staged".into()),
        ];
        assert_eq!(all.len(), 38, "update this list when a variant is added");
        for err in &all {
            for locale in Locale::ALL {
                let text = err.render(locale);
                assert!(!text.trim().is_empty(), "{} in {locale:?}", err.code());
                for slot in ["{0}", "{1}", "{2}", "{3}"] {
                    assert!(
                        !text.contains(slot),
                        "{} leaked {slot} in {locale:?}: {text}",
                        err.code()
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod payload_and_thread_tests {
    use super::*;
    use crate::{dims_of, Decl};

    /// `DeclarationUnit` is produced by `dims_of` and must carry the declared name plus the
    /// inner unit error, rendered in the *requested* locale (not English) in both halves.
    #[test]
    fn declaration_unit_carries_name_and_localizes_the_inner_unit_error() {
        let decls = [Decl {
            name: "T".into(),
            quantity: "temperature".into(),
            unit: Some("degC".into()),
        }];
        let err = dims_of(&decls).unwrap_err();
        assert_eq!(err.code(), "declaration-unit");
        let Error::DeclarationUnit { name, unit } = &err else {
            panic!("{err}")
        };
        assert_eq!(name, "T");
        assert!(
            matches!(&**unit, Error::OffsetUnit(u) if u == "degC"),
            "{unit}"
        );
        for locale in [Locale::Ja, Locale::Ru, Locale::ZhHans] {
            let text = err.render(locale);
            // The OUTER phrase is localized and the INNER sentence is too: no English template words.
            assert!(!text.contains("offset unit"), "{locale:?}: {text}");
            assert!(!text.contains("declaration `"), "{locale:?}: {text}");
            assert!(text.contains("degC"), "{locale:?}: {text}");
        }
        // Unknown unit, same shape.
        let decls = [Decl {
            name: "x".into(),
            quantity: "length".into(),
            unit: Some("zzqq".into()),
        }];
        let err = dims_of(&decls).unwrap_err();
        assert!(
            matches!(&err, Error::DeclarationUnit { unit, .. } if matches!(&**unit, Error::UnknownUnit(_)))
        );
    }

    #[test]
    fn reserved_name_reason_is_translated_not_free_text() {
        let cases = [
            (ReservedReason::BuiltinFunction, "内置函数"),
            (ReservedReason::ImaginaryUnit, "虚数单位"),
        ];
        for (reason, needle) in cases {
            let e = Error::ReservedName {
                name: "j".into(),
                reason,
            };
            let zh = e.render(Locale::ZhHans);
            assert!(zh.contains(needle), "{zh}");
            assert!(
                !zh.contains("rename the declaration") && !zh.contains("pick another name"),
                "{zh}"
            );
        }
    }

    #[test]
    fn duplicate_declaration_omits_the_position_when_unknown_and_names_it_when_known() {
        let unknown = Error::DuplicateDeclaration {
            name: "x".into(),
            line: None,
        };
        let en = unknown.render(Locale::En);
        assert!(!en.contains("line 0") && !en.contains("(line"), "{en}");
        let known = Error::DuplicateDeclaration {
            name: "x".into(),
            line: Some(3),
        };
        assert!(known.render(Locale::En).contains("(line 3)"));
        assert!(known.render(Locale::ZhHans).contains("第 3 行"));
        assert!(known.render(Locale::Ja).contains("3 行目"));
    }
}
