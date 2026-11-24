#!/usr/bin/env python3
"""生成 fixtures/probe/src/bin/*.rs —— 宏层前端「形式化测试」的用例矩阵。

矩阵是数据，不是散落在几十个文件里的手工残留：
  * CASES   —— 前端行为用例（宏 / 载荷 / 期望 / 绑定 / 说明）
  * SYMBOLS —— 字形扫掠：某字形能不能作为 **Rust 源码** 活下来（只测词法，不经过我们的文法）

期望的三种取值（措辞刻意不 overclaim）：
  TOKEN_FAIL   —— Rust 自己拒绝（词法/语法层），我们的宏根本没跑到
  FORMULA_FAIL —— 宏跑到了，并按我们的规则拒绝（FORMULA-ERROR: …）
  OK           —— 编译通过
  MEASURE      —— 只采集结果、不断言（用于字形扫掠）
"""

import pathlib
import textwrap

ROOT = pathlib.Path(__file__).resolve().parent
OUT = ROOT / "fixtures" / "probe" / "src" / "bin"

# (id, macro, payload, expect, params, note)
CASES = [
    # ---- 裸 token 流：Rust 词法这一关能不能过 ----
    ("bare_tex_frac", "formula_latex", r"\frac{a}{b}", "TOKEN_FAIL", [],
     "LaTeX 反斜杠不是 Rust token —— 这是整个方案的硬边界"),
    ("bare_tex_alpha", "formula_latex", r"\alpha", "TOKEN_FAIL", [],
     "数学命令与上一条同因"),
    ("bare_tex_dollar", "formula_latex", "$ x $", "FORMULA_FAIL", [],
     "实测纠正：`$` 是合法 token（宏元语法字符），能过词法；被拒的是我们的子集"),
    ("bare_tex_left", "formula_latex", r"\left( x \right)", "TOKEN_FAIL", [],
     "\\left/\\right 同因"),
    ("bare_unicode_nabla", "formula_matlab", "∇ u", "TOKEN_FAIL", [],
     "∇(U+2207, Sm) 不是 XID_Start，Rust 不认"),
    ("bare_unicode_partial", "formula_matlab", "∂u", "TOKEN_FAIL", [],
     "∂(U+2202, Sm) 同上"),
    ("bare_unicode_arrow", "formula_matlab", "a → b", "TOKEN_FAIL", [],
     "→(U+2192, Sm) 同上"),
    ("bare_matlab_transpose", "formula_matlab", "decl { a : l [m]; } a'", "TOKEN_FAIL", [],
     "`'` 在 Rust 里是生命周期/字符字面量起点，转置运算符无法进宏"),
    ("bare_matlab_ok", "formula_matlab", "decl { a : l [m]; b : l [m]; } a / b", "OK", ["a", "b"],
     "MATLAB 表达式子集可以整段当 token 流喂进宏"),
    ("bare_mathml", "formula_matlab", "<math><mi>x</mi></math>", "FORMULA_FAIL", [],
     "MathML **能过词法**（< > / 都是 token），但作为源语言被我们的子集拒绝"),
    ("bare_unicode_ident", "formula_matlab", "decl { α : l [m]; } α + 1.0", "FORMULA_FAIL", [],
     "α 是合法 Rust 标识符（XID_Start）；本子集刻意只收 ASCII 标识符，故在语义层拒绝"),
    ("bare_percent_comment", "formula_matlab", "decl { a : l [m]; } a % 注释", "OK", ["a"],
     "`%` 是合法 Rust token，所以 MATLAB 注释在宏层可存活"),
    ("bare_colon_end", "formula_matlab", "decl { a : l [m]; } a(1:end)", "FORMULA_FAIL", [],
     "`:` 与 `end` 都是合法 token（语法层能过），但不在本子集语义内"),

    # ---- 多行宏块（联邦制） ----
    ("federated_multiline_macro", "formula_matlab",
     "decl { F : f [N]; m : m [kg]; r : rad [m]; }\naccel = F / m\nmech = ( accel * r )^(1/2)", "OK",
     ["F", "m", "r"],
     "token 车道**换行分隔**联邦公式（span 行号在 stable 可用，行号已对齐源文件）"),
    ("federated_let_chain_value", "formula_matlab",
     "decl { x : d; }\na = x + 1\nb = a * 2\nb - a", "OK", ["x"],
     "三条联邦（换行分隔）+ 尾条无名：宏产物是 let 链 + 终值"),
    ("matrix_free_spacing_macro", "formula_matlab",
     "decl { b : d; c : d; a : d; }\n[2*b  -b   ... 0\n -b  2*b  -b  ...\n ...  -b  2*b  -b\n 0  ...  -b  2*b]", "FORMULA_FAIL", [],
     "模式矩阵在**内联车道**被拒并引导走文件车道（分诊规则）"),

    ("matrix_newline_rows_in_macro", "formula_matlab",
     "decl { k : d; }\n[2*k, -k, ..., 0\n -k, 2*k, -k, ...\n ..., -k, 2*k, -k\n 0, ..., -k, 2*k]", "FORMULA_FAIL", [],
     "token 车道换行分行矩阵能被解析，但模式矩阵仍需文件车道（分诊规则）"),

    # ---- 字符串车道：语义与诊断 ----
    ("str_tex_frac", "formula_latex", r"decl { a : l [m]; b : l [m]; } \frac{a}{b}", "OK", ["a", "b"],
     "字符串车道让 LaTeX 可用，代价是内层 span 全丢"),
    ("str_tex_sin2", "formula_latex", r"decl { x : a; } \sin^2 x", "FORMULA_FAIL", [],
     "\\sin^2 x 有歧义（(sin x)^2 还是 sin(x^2)），必须拒绝"),
    ("str_tex_dollar_wrap", "formula_latex", r"decl { x : a; } $x$", "FORMULA_FAIL", [],
     "行内数学定界符不属于公式本体，收进源里就是隐式约定"),
    ("str_tex_left_right", "formula_latex", r"decl { x : a; } \left( x \right)", "FORMULA_FAIL", [],
     "\\left/\\right 只影响排版高度，没有数学语义 —— 拒绝"),
    ("str_matlab_implicit", "formula_matlab", "decl { x : a; y : a; } x y", "FORMULA_FAIL", [],
     "MATLAB 语义里 `x y` 不是乘法；并置乘法只属于 LaTeX 的单字母体系"),
    ("str_matlab_dotmul", "formula_matlab", "decl { x : a; y : a; } x .* y", "FORMULA_FAIL", [],
     "`.*` 是逐元素语义，静默当 `*` 会改变含义 —— 必须拒绝"),
    ("str_dim_mismatch", "formula_latex", r"decl { a : l [m]; t : t [s]; } a + t", "FORMULA_FAIL", [],
     "量纲不一致：这是本方案最赚钱的一类拦截"),
    ("str_undeclared", "formula_latex", r"decl { a : l [m]; } a + b", "FORMULA_FAIL", [],
     "未声明符号一律拒绝，不做字形推断"),
    ("str_sqrt_odd", "formula_latex", r"decl { v : v [V]; } \sqrt{v}", "OK", ["v"],
     "半整数量纲后 √V = V^(1/2) 合法（原 FORMULA_FAIL 期望随分数指数特性落地而更新）"),
    ("str_matrix_literal", "formula_matlab", "decl { a : a; b : a; } [1, a; b, 2]", "OK", ["a", "b"],
     "具体矩阵已接入量纲检查（元素同量纲即可）——原 FORMULA_FAIL 期望随矩阵支持落地而更新"),
    ("str_matrix_dim_mismatch", "formula_matlab", "decl { a : l [m]; t : t [s]; } [a; t]", "FORMULA_FAIL", [],
     "矩阵元素量纲不一致必须被拒（m vs s）"),
    ("str_ok_bv", "formula_latex",
     r"decl { i_0 : j [A/m^2]; eta : v [V]; alpha : d; n : d; F : f [C/mol]; R : g [J/mol/K]; T : t [K]; }"
     r" i = i_0 ( \exp( \frac{\alpha n F \eta}{R T} ) - \exp( -\frac{(1 - \alpha) n F \eta}{R T} ) )",
     "OK", ["i_0", "eta", "alpha", "n", "F", "R", "T"],
     "Butler–Volmer：LaTeX 表面 + 量纲全通 + 并置乘法 + 嵌套分式"),
    ("str_ok_arrhenius", "formula_latex",
     r"decl { A : f [Hz]; E_a : e [J/mol]; R : g [J/mol/K]; T : t [K]; } k = A \exp( -\frac{E_a}{R T} )",
     "OK", ["A", "E_a", "R", "T"],
     "Arrhenius：多字母名字只能写 E_a（LaTeX 里 Ea 会被读成 E·a —— 这正是单字母体系的约束）"),
    ("str_ok_greek", "formula_latex", r"decl { alpha : d; eta : d; } \alpha \eta", "OK", ["alpha", "eta"],
     "希腊命令 → ASCII 标识符：LaTeX 表面与 Rust 标识符之间的映射"),
    ("str_ok_matlab", "formula_matlab",
     "decl { F : f [N]; m : m [kg]; a : acc [m/s^2]; r : rad [m]; } a = F / m", "OK", ["F", "m"],
     "MATLAB 表面：多字母标识符、显式乘法、无并置"),
]

# 字形扫掠：测的是**纯 Rust 词法**（用 stringify!，不经过我们的文法）。
# 类别按 Unicode 预测，允许被实测推翻。
SYMBOLS = [
    # Ll / Lu —— 预测：能当标识符
    ("03b1", "alpha", "letter"),
    ("03b2", "beta", "letter"),
    ("0393", "Gamma", "letter"),
    ("0394", "Delta", "letter"),
    ("03c0", "pi", "letter"),
    ("03b8", "theta", "letter"),
    ("03bb", "lambda", "letter"),
    ("03bc", "mu", "letter"),
    ("03c1", "rho", "letter"),
    ("03c3", "sigma", "letter"),
    ("03c4", "tau", "letter"),
    ("03c6", "phi", "letter"),
    ("03c8", "psi", "letter"),
    ("03c9", "omega", "letter"),
    ("03a9", "Omega_cap", "letter"),
    ("03a3", "Sigma_cap", "letter"),
    ("ff21", "fullwidth_A", "letter"),
    # Sm / So / No / Po —— 预测：词法不认
    ("2202", "partial", "op"),
    ("2207", "nabla", "op"),
    ("2211", "sum", "op"),
    ("220f", "prod", "op"),
    ("222b", "integral", "op"),
    ("221a", "sqrt", "op"),
    ("00b1", "plusminus", "op"),
    ("00d7", "times", "op"),
    ("00f7", "divide", "op"),
    ("00b7", "centerdot", "op"),
    ("2264", "le", "op"),
    ("2265", "ge", "op"),
    ("2260", "ne", "op"),
    ("2248", "approx", "op"),
    ("221e", "infty", "op"),
    ("2192", "to", "op"),
    ("21d2", "implies", "op"),
    ("2208", "in", "op"),
    ("222a", "union", "op"),
    ("2229", "inter", "op"),
    ("2218", "circ", "op"),
    ("2297", "otimes", "op"),
    ("00b0", "degree", "op"),
    ("00b2", "sup2", "op"),
    ("00b3", "sup3", "op"),
    ("2044", "fraction_slash", "op"),
    ("27e8", "langle", "op"),
    ("2225", "parallel", "op"),
    ("2200", "forall", "op"),
    ("2203", "exists", "op"),
    ("2212", "minus_sign", "op"),
    ("2261", "equiv", "op"),
    # 组合附加符：附在字母上 → 整串变 XID_Continue，可能"假装"成数学符号
    ("0064+0338", "d_stroke", "combining"),
    ("0078+0307", "x_dot", "combining"),
    ("0078+0305", "x_bar", "combining"),
    ("0061+0302", "a_hat", "combining"),
    ("03b1+0308", "alpha_diaeresis", "combining"),
]


def render_case(case):
    cid, macro, payload, expect, params, note = case
    payload = payload if not cid.startswith("str_") else 'r#"' + payload + '"#'
    binds = "".join(f"    let {p} = 1.5f64;\n" for p in params)
    head = (
        f"// EXPECT: {expect}\n"
        f"// NOTE: {note}\n"
        f"// 自动生成，勿手改 —— 见 make_probes.py\n"
    )
    body = textwrap.dedent(
        f"""\
        #![allow(warnings)]
        use himari_macros::*;

        fn main() {{
        {binds}    let _v = {macro}!{{ {payload} }};
            println!("{{_v:?}}");
        }}
        """
    )
    return head + body


def render_symbol(code, name, klass):
    glyph = "".join(chr(int(part, 16)) for part in code.split("+"))
    return (
        "// EXPECT: MEASURE\n"
        f"// GLYPH: {glyph}\n"
        f"// CLASS: {klass}\n"
        f"// NOTE: {name} U+{code.upper()}\n"
        "// 自动生成，勿手改 —— 见 make_probes.py\n"
        "#![allow(warnings)]\n"
        "fn main() {\n"
        "    // 只测词法：这个字形能不能作为 Rust 源码活下来\n"
        f'    println!("{{}}", stringify!({glyph}));\n'
        "}\n"
    )


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for old in OUT.glob("probe_*.rs"):
        old.unlink()
    for case in CASES:
        (OUT / f"probe_{case[0]}.rs").write_text(render_case(case), encoding="utf-8")
    for code, name, klass in SYMBOLS:
        (OUT / f"probe_sym_{name}.rs").write_text(
            render_symbol(code, name, klass), encoding="utf-8"
        )
    print(f"wrote {len(CASES)} 行为用例 + {len(SYMBOLS)} 字形探针 → {OUT}")


if __name__ == "__main__":
    main()