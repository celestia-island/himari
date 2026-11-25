<!-- markdownlint-disable MD033 MD041 -->
<!-- The logo lives in celestia-island.github.io/res/logos/himari.webp; that asset does not
exist yet, and this repository does not keep a copy of org assets. Add the <img> line here
once the logo lands, then drop this comment. -->
<h1 align="center">Himari</h1>

<p align="center"><strong>公式资产：唯一的事实来源、三种表面语言，编译成 Rust。</strong></p>

<div align="center">
<!-- CI -->
<a href="https://github.com/celestia-island/himari/actions/workflows/ci.yml">
<img src="https://img.shields.io/github/actions/workflow/status/celestia-island/himari/ci.yml?label=CI" alt="CI" />
</a>
<!-- depspec -->
<a href="https://github.com/celestia-island/himari/actions/workflows/depspec-lint.yml">
<img src="https://img.shields.io/github/actions/workflow/status/celestia-island/himari/depspec-lint.yml?label=depspec" alt="depspec" />
</a>
<!-- License -->
<a href="https://github.com/celestia-island/sysl">
<img src="https://img.shields.io/badge/license-SySL%201.0-blue" alt="License: SySL 1.0" />
</a>
<!-- crates.io -->
<a href="https://crates.io/crates/himari">
<img src="https://img.shields.io/crates/v/himari.svg" alt="crates.io" />
</a>
<!-- docs.rs -->
<a href="https://docs.rs/himari">
<img src="https://docs.rs/himari/badge.svg" alt="docs.rs" />
</a>
<!-- Rust -->
<a href="https://www.rust-lang.org/">
<img src="https://img.shields.io/badge/rust-1.90-orange.svg" alt="Rust 1.90" />
</a>
</div>

<div align="center">

[English](../../README.md) ·
**简体中文** ·
[繁體中文](../zh-Hant/README.md) ·
[日本語](../ja/README.md) ·
[한국어](../ko/README.md) ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>
<!-- markdownlint-enable MD033 MD041 -->

> **NOTICE：本软件包含由人工智能生成的代码。许可条款（含模型披露要求）见 LICENSE 文件（Synthetic Source License）。**
> 本仓库的每一行源码都由 AI 模型在人类指导下生成，没有一行是手写的。参与的模型列于 [LICENSE.txt](LICENSE.txt) 末尾。
> 本 README 声称的每一项能力都有本仓库中的测试作为支撑，并经过了六轮独立的对抗式审查——但**AI 审查不能替代人类审查**，
> 且截至撰写时，数学内容尚未经数学家审阅。
> 名字取自 Blue Archive 的秋山阳奈（Himari Akayama，千年科学学园 · Veritas）——
> 把点子变成会运转的机器。crates.io 的名称与组织仓库名都没有与任何东西冲突（2026-11-23 核查）。

## 这是什么

- 你写**一份**公式（LaTeX 子集 / 论文记法 / MATLAB 风格，任选）；它降为同一个
  **带量纲的 IR**，再编译成带具名中间量与源码注释的 Rust 函数。
- 量纲（七个基本量 kg·m·s·A·K·mol·cd）在编译期强制：`m + s` 根本编译不过。
- 内置符号微分、CSE（公共子表达式消除）与常量折叠；解析偏导由测试对着中心差分钉住。
- **矩阵是一等公民**：具体矩阵会被完整检查；**模式矩阵**（带省略号）可表达动态尺寸——
  教科书上的 `[b c ⋯ 0; a b c ⋯; ⋯ a b c; 0 ⋯ a b]` 就按字面写成源码。

## 联邦与多行书写（2026-11-23 第二号裁决）

**一个宏块可以容纳一整组互相引用的公式**（“联邦”）：

```rust
formula_matlab!{
    decl { F : force [N]; m : mass [kg]; r : radius [m]; }
    accel = F / m
    mech = ( accel * r )^(1/2)
}
// expands to a let chain: let accel = F / m; let mech = (accel * r).sqrt(); mech
```

- **条目分隔符** = 分号或**换行**，而且这条规则**在三条车道上都成立**（token 车道从 proc-macro 的
  span 行号重建换行，而那些行号已对齐源文件，所以诊断指向同一行）。
- **后面的条目可以引用前面的条目**：量纲随名字一起传播（accel 的 m/s² 加入符号表）；build.rs 车道在生成期把代换内联进去。
- **名字就是契约**：当公式复用一个 `decl` 名字时，其结果量纲必须与那份声明一致
  （`a = F/m` 可以，`s : Pa; s = F/d` 会被拒绝）；同一个公式集或同一个生成目录中的重名公式一律拒绝。
- **运算符续行模型**：运算符必须位于**行尾**（`x +⏎ y` ✓）；下一行开头的 `-`/`+` 是新条目的正负号，而不是跨行减法——这就是消歧义裁决。
- LaTeX 表面的隐式乘法（并置）**必须留在同一行**：跨行并置会把下一个公式吞成一个乘性项。

**矩阵多行书写**（每一行语义行写一条物理行；列间距自由，不需对齐）：

```text
[ 2*k, -k,    ...,   0 ;
  -k,  2*k,   -k,    ... ;
  ..., -k,    2*k,   -k ;
  0,   ...,   -k,    2*k ]
```

- 行分隔符 = `;` 或换行（可混用、可选结尾）；列分隔符 = 逗号或空格（≥1，任意多个）。
- 单元格中的省略号 `...` / `⋯` / `…` 表示“此处省略”（写回时推水平/垂直/对角）。
- **写在 `-` 前没有空格时它是正负号，前面有空格则开始新的一列**（MATLAB 语义）：`[k-1]` 是一列，`[k -1]` 是两列。

## 复数：从值层一路管道接到代码生成（2026-11-23）

- **`j` 是保留的虚数单位**（永远无量纲、永不声明、**永不进入参数列表**）。`Z = R_s + j*X` 可以直接书写。
- 值层恰好有两个变体：`Value::Scalar(C)` / `Value::Matrix(Vec<Vec<C>>)`，其中 `C = {re, im}`。
  实数就是虚部为零的复数 ⇒ **复矩阵、转置、乘法、加法与减法全部
  免费地逐元素成立**；不需要“实矩阵／复矩阵”的象限切分。
- 超越函数在实数与复数上共用一张表：`exp ln sqrt sin cos tan sinh cosh tanh` +
  `abs arg conj re im`（后四个总是落回实轴）。复数幂走主分支上的 `exp(w·ln z)`。
- **`eval` 对复数值给出明确错误**（不会悄悄丢掉虚部）；想要复数值就用 `eval_value`。
- **代码生成有独立的复数车道**：当公式包含 `j` 时输出为 `-> (f64, f64)` =（实部, 虚部），
  而块内辅助函数（`__c_mul` / `__c_sqrt` …）**只输出实际用到的那些**（否则 clippy `-D warnings` 会变红）。
  批次入口是 `out: &mut [(f64, f64)]`，可直接用于 EIS 频率扫描。
  宏车道（`formula!` 等）也按同样方式分流：`j` 不会在调用处**退化成自由变量**；
  联邦 `let` 链中已经是复数的绑定会被正确继承（`emit_block_complex_with`）。
  复数车道**不生成解析导数**：`abs/arg/conj/re/im` 不是复解析的，所以链式法则不成立。
- **生成的代码与值层必须逐点一致**是硬性约束：`__c_powf` 复制 `C::pow_real` 的整数分流
  （负实底数的整数次幂必须精确落在实轴上），而 `__c_sqrt` 与值层一样使用 `hypot`，以避免大数量级时溢出；
  `__c_div` 遵循 IEEE 而值层报错是刻意的——这些在 fixtures 中各有一个逐运算符的对照测试把关。
- 单位字符串接受有理指数：`[V/A*s^(-1/2)]`（方括号是必需的——否则指数中的 `/` 无法与
  单位字符串的除号区分）；不能整除的指数（`m^(1/3)`）会被明确拒绝。无量纲量请写 `[1]`
  （增益 / 计数 / 摩尔分数都必须可以声明；不能靠“省略单位”偷渡进来）。
- **`himari-fmt --diff <file>`**：只读预览，打印 `--- current / +++ canonical`
  的 unified diff（无改动 → 无输出且退出码 0；有改动 → 退出码 1）。
- **实数与复数之间的语义边界**（刻意为之，不是疏漏）：`1/0` 过去给出 `inf`，现在**报错**；
  `ln(0)` 同理；`(-2)^0.5` 报“结果是复数，请用 `eval_value`”。
  生成的代码则遵循 IEEE 语义（除以零得到 inf/NaN）：热路径不携带错误路径，
  而公式层能静态证明为错的东西由量纲/形状门禁拦下。
- **`python3 scripts/fmt_check.py --fix`**：一条命令重新排成规范形式并重新生成 sidecar。

## 单位：SI 词头、换算、无量纲组（2026-11-23）

- **拆词头前先查全名**：`m` 是米（不是“毫-什么”），`mol` 是摩尔，`cd` 是坎德拉，
  `daN` 是十牛（最长词头优先）。`nF` / `GHz` / `mm^2` / `mV/mm` / `µF` / `Ω` 都能用。
  导出单位覆盖电子领域躲不掉的六个：`F` `Ω`（或 `ohm`）`S` `H` `Wb` `T`。
- **量纲与换算因子分开存储**：`parse_unit` 只给量纲；`parse_unit_spec` 另外给出 →SI 换算因子；
  `convert(v, "nF", "F") = 1e-9 F`，量纲不符则报错。质量的基本单位是 **kg**：
  `g = 1e-3 kg`、`mg = 1e-6 kg`、`Mg = 1000 kg`；词头从克起算（由一个回归测试钉住）。
  **生成函数的参数永远是 SI 基本单位**，声明中的词头只影响可读性——生成代码的 doc comment
  会针对每个参数写出换算因子（以下逐字摘自真实的生成 doc comment：
  `⚠ 参数 C_dl 声明为 µF：入参是 SI 基单位（1 µF = 0.000001 …）`）。
- **CGPM 2022 词头**已在表中：`Q` (1e30)、`R` (1e27)、`r` (1e-27)、`q` (1e-30)，与既有的
  `da … Y / d … y` 并列。
- **有精确定义的非 SI 单位**（`unit_base` 带着各自的因子，好让 `convert` 在量纲上保持诚实）：`min`、`h`、`d`、`L`
  （或 `l`；`mL` 拆解为 `L` 上的词头）、`deg`、`arcmin`、`arcsec`、`bar`、`atm`、`eV`、`Wh`、`tonne`、`Da`（或 `u`）、`nmi`、`knot`、`ha`、`acre`。
- **偏移／对数单位被明确拒绝**：`degC`（偏移）与 `dBm`（一个比值）在拆词头之前就被拦下，
  报的是“真正的原因”而不是“未知单位”——把它们悄悄当成 K 或无量纲会算出**错误的值**。
  **歧义符号同样被拒绝，而不是猜**：`cal` / `Cal` 在热化学卡（4.184 J）与 IT 卡（4.1868 J）之间不同，所以两者都
  被拒绝并提示改写为 `J`；`rpm` 需要一个角度轴（1 rpm = 2π/60 rad/s），而本设施把 `rad` 视为无量纲，所以也被拒绝，
  并提示改写成 `2 * pi * f` 并加上 `[Hz]` 声明。
- **无量纲组（Buckingham Π）**：`dimensionless_groups` 用**精确有理数**消元（`f64` 会漂移），
  直接由“变量名 + 量纲”给出一个基的无量纲组合；个数 =（变量个数）− 秩（Π 定理即断言）。
  经典核对：`{rho, v, L, mu}` → 1 组 `rho·v·L·mu^-1`（雷诺数）。
  `dim_solve` 是同一方程组非齐次的用法：`{kg, m/s²} → N` 给出指数 `[1, 1]`，
  而无法构成的量纲会报“无法构成”以及还差几个量纲。

## 量纲：半整数指数（2026-11-23）

`Dim` 内部以 **1/2** 为单位存储（`[i16;7]` 存的是真实指数的两倍），所以分数次幂与整数次幂
走同一套整数运算：`√s = s^(1/2)`、`f^(-1/2)`、`Ω·s^(-1/2)` 全部合法，显示用括号形式
如 `s^(-7/2)`。因此 Warburg 阻抗回到教科书形式 `Z = σ·f^(-1/2)`，
不需要参考频率无量纲化。`dim_check` 的幂指数**先折叠常量**，
所以自然写法 `f ^ (1/2)` 不必先调用 `simplify` 就能过门禁。

## 函数族：极值 / 钳制 / 矩阵运算符（2026-11-23）

函数元数（arity）的**唯一事实来源**是 `func_arity`（在 core 中）；四条车道（元数检查 / 求值 / 量纲 / 生成 / 写回）
都查它——以前每条车道各带一份 `match`，这正是“`arg` 在实数车道被生成为不存在的方法”能撑过一个版本的原因。

| 族 | 函数 | 备注 |
| --- | --- | --- |
| 一元标量 | `exp ln log sqrt sin cos tan sinh cosh tanh` | 实数与复数共用同一张表 |
| 复数 | `abs arg conj re im` | `arg` 无量纲；在实数车道，re/im/conj 是恒等 |
| 极值/钳制 | `min(a,b)` `max(a,b)` `clamp(v,lo,hi)` | 形状同 Rust：`v.max(lo).min(hi)`；三个参数必须同量纲 |
| 矩阵运算符 | `det` `trace` `adjoint` `inv` | **只接受具体矩阵字面量**；`det`/`inv` 最高到 4/3 阶（足以覆盖 2D 与 3D 等参单元），`adjoint` = 共轭转置 |

`det`/`inv` 使用**余子式递归**（`det` ≤ 4、`inv` ≤ 3）：足以应付 2D 三角形与 3D 四面体的等参映射，
并有 `J·J⁻¹ = I` 与 `det(J)=1` 的数值检查；4×4 的逆需要 16 个 3×3 余子式
（约 100 项），会让生成的代码膨胀，所以被明确拒绝并提示改用数值库。`det`/`trace` 在 `diff` 中**先展开再微分**——
展开结果是一个普通的可微表达式，所以 FEM 的 `area = det(J)/2` 既有值也有解析导数
（`∂area/∂x₂ = (y₃-y₁)/2`）。
`min/max/clamp` 的分段选择无法在 IR 中表达（没有比较运算符），所以**不生成导数**；
生成代码中有一行注释说明原因（它不再 panic 了）。极值与钳制遵循 `f64::min/max` 的 NaN 规则
（可能返回另一个操作数）；需要 NaN 传播的调用方应在边界检查 `is_finite`。

## 内置形状一览

`identity` / `diag(d)` / `diag_v(d[i])` / `tridiag(a,b,c)` / `tridiag_v(b[i])` /
`bidiag_u(b,c)` / `bidiag_l(a,b)` / **`blockdiag2(p,q)` = diag(p·I₂, q·I₂)** /
**`banded(v, k)` = 半带宽为 k 的对称带状**（k=2 从 5×5 起才唯一；在 4×4 会退化成稠密矩阵）。

这些名字描述的是**被识别出来的 IR 形状**；它们不是你能在公式里调用的函数。入口是
带省略号的矩阵草图，任何无法识别的东西都会被明确拒绝。

块内的块对角语义 = 标量 × 单位矩阵（对角线上是 p，块内的非对角为空），块外必须为空
（`0` 与 `...` 等价）；若总维数为奇数，中心单元格归右下块。

## 复数车道性能：诚实的那一面（2026-11-23 实测）

`bin/bench.rs` 现在测四条车道。复数那条 Z_c（EIS 频率扫描）**比手写更慢**；照实记录：

| 形式 | ns/点 | 相对手写 |
| --- | --- | --- |
| 手写特化（`σ/√ω·e^{-jπ/4}`，无分支，可向量化） | 2.3 | 1× |
| 同形式转写（把生成代码的运算符序列抄进一个循环） | 2.9 | 1.3× |
| 生成代码 · 标量直接调用 | 6.4 | 2.8× |
| 生成代码 · 批次入口 | 12.8 | 5.5× |

拆解：**通用复数 √/÷ 比手写特化多花约 2.2×**（1 次 sqrt 对 3 次 sqrt + 一次除法），
而**批次包装再多花约 2.5×**（元组切片写入 + 分支让循环无法向量化）。

已完成的优化（每一项都真的缩小了差距）：

1. `__c_sqrt` 从极坐标形式（atan2+cos+sin，4 个超越函数）改为**代数闭式**；
1. 纯虚快速路径 `√(iy) = √(|y|/2)(1+sign·j)`（只要 1 次 sqrt——EIS 中最常见的形状）；
1. `__c_div` 在一般数量级使用朴素形式（1 次除法）；
1. 复数辅助函数从**闭包**改为块内 `#[inline(always)] fn`（15.5 → 11.9 ns/点）。

**结论（写进 README 而不是藏起来）**：复数车道适合“算得对 + 够快”，不值得
去追手写特化的极限；对于 EIS 扫描真正热的循环，手写特化，并把公式留在
`.formula` 文件中作为**事实来源与量纲检查**。实数车道没有这个问题（标量 1.11×，与手写相当）。

## 批次入口：定长胜出，切片落败（实测）

生成器为每个公式输出**两个**批次入口：

| 形式 | 签名 | 实测（BV，100k 点） |
| --- | --- | --- |
| 切片版 | `bv__batch(p0: &[f64], …, out: &mut [f64])` | 24.1 ns/点 = **标量的 0.82×（慢 22%）** |
| 定长版 | `bv__batch_n<const N: usize>(p: &[f64; N], …, out: &mut [f64; N])` | 19.3 ns/点 = **标量的 1.03×（快 3%）** |

结论：**想要性能，就把长度放进类型参数**。切片版每点的八次边界检查都在循环里，
LLVM 无法完全消除；先算一次公共长度（`.min()` 链）或改用 zip 迭代器也只能到 0.83×。
切片版保留为方便形式（变长数据）；热路径请用定长版。
（bench 源码：`fixtures/three/src/bin/bench.rs`；我们踩到的两个方法论陷阱——在计时区内构造向量，
以及常量向量让标量路径被常量折叠——都记录在源码注释中。）

## 矩阵代数（形状 + 求值 + 生成）

量纲与形状是**两条正交的检查轴**：量纲管物理意义，形状管代数合法性。

| 能力 | 行为 |
| --- | --- |
| `shape_of` | 具体矩阵 → `(rows, cols)`；转置交换；符号/模式矩阵 → `None`（不做动态推断） |
| `shape_check` | 矩阵加/减要求形状相同，矩阵乘积要求 `left cols = right rows`，锯齿矩阵拒绝，矩阵除法明确拒绝 |
| `eval_value` | 标量/矩阵的统一求值，矩阵乘积为 `C_ij = Σ_k A_ik·B_kj` |
| `flatten_matrix_value` | 把**可静态展开**的矩阵表达式折叠成具体矩阵 IR（转置/加/减/取负/标量广播/矩阵乘积） |
| 生成 | 折叠后的矩阵 → `vec![vec![…], …]` 常量字面量；大矩阵走模式矩阵 + 循环车道 |

```text
[1, 2; 3, 4] * [5, 6; 7, 8]  →  [[19.0, 22.0], [43.0, 50.0]]
[1, 2, 3]'                   →  3×1 column vector (emitted as vec![vec![1.0], vec![2.0], vec![3.0]])
A * [1, 2]                   →  scalar broadcast (a bare symbol is a scalar: matrices only come from literals/pattern generators)
[1, 2, 3] * [1, 2, 3]        →  compile-time rejection: matrix multiply impossible: (1, 3) × (1, 3)
```

> **通用运行时 `A,b` 仍然必须这样声明**：`decl` 的量纲以 `_matrix`/`_vector` 结尾（或等于 `matrix`/`vector`）时
> 它就会成为矩阵/向量参数，下面的运行时车道用的就是它。所有*其他*裸符号都是标量，所以 `A * x` 这种对形状不在声明中的
> 矩阵的运算，仍然要用字面量、可静态展开的表达式或模式生成器来表达。
> 一般的大规模 `Ax=b`、运行时尺寸推断或病态方程组，请用数值线性代数库。

**非负整数矩阵次幂**：`[a, b; c, d] ^ 2` 由生成器折叠成显式的矩阵乘法（0–8 次），
`M ^ 0` 是同阶单位矩阵（返回类型仍是矩阵，不是标量 `1`）。端到端数值检查在
`matrix_power.formula`。折叠失败（负/分数次幂、越界、非方阵）输出 `compile_error!`，
而不是像 `vec![…] * vec![…]` 那样的编译事故；值层（`himari-eval`）对同一形式给出相同结果。

**两个独立实现必须一致**：矩阵表达式的代码生成（`flatten_matrix_value` + 字面量输出）
与值层求值（`eval_value`）是分开写的，所以用一个**差分测试**让两边对齐——
求值折叠后的字面量必须逐格等于求值未折叠的表达式（`mat-expr-fold`）。

**没有省略号的矩阵草图就是具体矩阵**：它走字面量车道（常量折叠，没有分配循环），
这比生成器快；要用生成器车道（`n` 是运行时参数）就必须写出省略号。

## 运行时矩阵/向量车道（2026-11-23）

上面一切都是静态的：形状在公式编译时就知道。运行时车道是针对
**尺寸只在运行时才存在的参数**的另一个答案——消费用户数据的求解器、FEM 装配、任何把 `A` 与 `b` 当作值接收的东西。

- **声明是唯一的开关**：`decl` 的量纲以 `_matrix`/`_vector` 结尾（或等于 `matrix`/`vector`）时，它会成为
  矩阵/向量参数——`A : stiffness_matrix [N/m]` 以 `&[Vec<f64>]` 传入，`b : load_vector [N]` 以 `&[f64]` 传入。
  从不选择加入的名字保持标量，所以既有公式不受影响（`Decl::runtime_kind`）。
- **生成的函数是可失败的**：返回 `Result<_, himari::runtime::NumericError>`，单一结构化错误类型
  （`InvalidInput` / `ShapeError` / `NonFiniteValue` / `SingularMatrix` / `IllConditioned` / `UnstableResidual` / `NoRealRoots`）。这条车道上
  **没有静默 NaN 契约**。
- **`solve(A,b)`** 是带部分主元的 LU，然后在返回任何答案前有两道独立的门禁：
  一个 **rcond 估计**（`1/(‖A‖∞·‖A⁻¹‖∞)`，要求 `rcond ≥ f64::EPSILON·64·n`）与一个**相对残差检查**
  （`≤ 1e-9`）。逆矩阵只为条件数估计而构造，从不是求 `x` 的途径。奇异主元、病态矩阵与残差过大的解
  是三种不同的错误，而不是一个笼统的“失败”。
- **这条车道还支持**：`inv`、`det`（只接受字面量参数——运行时 `det` 被拒绝，因为它的量纲需要静态的
  矩阵阶数）、`A*x`、`dot`、`norm`，以及用 **1 为起点**索引的 `A(i, j)`（`[I,J]`；越界与零是错误，不是 panic）。
- **它生成什么**：主体是生成函数内部对作用域内 `let` 链的受检表达式，返回类型跟随
  值的种类（`Result<f64, _>`、`Result<Vec<f64>, _>`、`Result<Vec<Vec<f64>>, _>`）。运行时车道的公式**不**输出解析 Jacobian、**不**输出
  `__f32` 变体、**不**输出批次入口（那些需要标量的、带尺寸的签名）。
- **答案对照独立参考检查**：`runtime_matrix.formula` 用另一套带主元的高斯
  消元验证 `solve(A,b)`，用 `A*x` 对照 `b`，用 `A·inv(A)` 对照单位矩阵的各列，检验 1 为起点的索引，以及一个刻意设计的极小 epsilon 方程组
  （`[[1e-17, 1], [1, 1]]`）——只有带主元才解得对——容差全部为 2e-12/2e-15。
- **宏车道**：`formula_matlab!` 也接受运行时公式，但**每个块只能有一个公式**（运行时链留在 build.rs 车道
  并附明确消息）；复数运行时矩阵参数目前被拒绝。

## 代码生成质量（2026-11-23）

- **整数次幂输出 `powi`**：`x^3` → `(x).powi(3)`（以前它输出一个不存在的内部方法，
  所以任何 `x^3` 都编译不过，错误还指向生成的代码）。fixture 为 `x^3` 设了**编译门禁**。
- **未知函数在编译期失败**：实数车道的函数是白名单；`arg(x)` 在实数上没有意义 →
  `compile_error!`；`re/im/conj` 在实数上是恒等（精确，不是近似）。
  多参数调用在解析期就被拒绝（生成器过去只看 `args[0]`，悄悄丢掉其余参数会算出错误的值）。
- **系统 Jacobian**：每个 `.formula` 文件额外输出 `{filename}__jacobian(all params)`，
  行 = **参与微分的实数标量公式**（矩阵值 / 复数 / 不可微的条目跳过），列 = 参数（声明顺序），
  而不依赖某参数的行使在该列上全程为 **0**。这正是隐式求解器 / 牛顿迭代 / FEM 装配需要的形状。
- **非有限常量硬失败**：输出 `inf` / `NaN` 会产生非法标识符，
  所以在生成前 panic，并指出源头（`contains_nan_expr` 是唯一事实来源）。
- **幂外的括号**：`(x^2)^3` 与 `x^(2^3)` 在两个表面都会加括号——
  `x ^ 2 ^ 3` 会被重新解析成 `(x^2)^3`（x⁸ → x⁶，也就是规范化格式化会**改变数学**）。

## 格式化（规范形式）

`himari-fmt`：`cargo run -p himari-check --bin himari-fmt -- [--check|--write] <file>`。

- 多行 decl（每行一个符号）；每行一个公式；**矩阵列对齐**（每列 = 最宽内容 + ≥1 空格，逗号是列边界）。
- 独立的注释行全部上浮到文件头部（保留顺序）——注释是给人看的，永不进入 IR。
- `--check` = 漂移门禁（接入 `just fmt-check` / CI）；规范输出往返幂等。
- **规范形式保留逗号**：纯空格分隔与二元减法在写回时无法区分（`-k` 会被吞成一个减项），
  所以逗号是明确的边界；手写时不写逗号没问题，格式化会把它们补回去。

## 车道（2026-11-23 用户裁决）

| 车道 | 用法 | 适用于 |
| --- | --- | --- |
| **内联宏** | `formula_matlab!{ decl { a : l [m]; b : l [m]; } a / b }` | 简单公式 |
| **字符串宏** | `formula_latex!{ r"decl { … } \frac{a}{b}" }` | 简单公式（LaTeX 表面） |
| **外部文件宏** | `formula_file!("formulas/bv.formula", matlab)` | 复杂公式（方言必须明确给出） |
| **build.rs** | 见 `packages/check/fixtures/three/` | 入库的生成代码 + 漂移门禁（正式车道） |

宏内的 `decl` 只提供量纲检查所需的符号信息；它**不会创建 Rust 局部变量**：调用处的 `a/b`
必须已经由调用方写成 `let a = …; let b = …;`。完整的可编译示例见
`inline_macro_example()`（位于 `examples/common/src/main.rs`）。

> ⚠️ 已知限制：读取文件的 proc-macro **不被 cargo 跟踪**（`tracked_path` 仍是 nightly），
> 所以编辑 `.formula` 后需要 `touch` 才触发重建；生产请用 build.rs 车道（`rerun-if-changed`）。
> ⚠️ 模式矩阵/转置的**构造子车道**只存在于文件 / build.rs（它需要文件名与生成器）；
> 内联宏遇到它们时给出带提示的明确错误，永不静默降级。

## 表面语言（三种，一个 IR）

| | 论文 | LaTeX | MATLAB 风格 |
| --- | --- | --- | --- |
| 希腊字母 | `α·x` | `\alpha \cdot x` | `alpha*x` |
| 省略号 | `⋯ …` | `\cdots \vdots \ddots` | `...` |
| 矩阵 | `[a, ⋯, b; ⋮ ⋱ ⋮; c, ⋯, d]` | `\begin{bmatrix}…\end{bmatrix}` | `[a, ..., b; ...]` |
| 转置 | `A'`、`A′`、`A^T` | `A^T` | 同左 |
| 量纲声明 | `decl { v : velocity [m/s]; }` | 同左 | 同左 |

规则（拒绝就是特性）：

- 每个符号都必须先连同量纲声明；未声明 = 编译错误，不做字形推断。
- `\sin^2 x`、`\left(\right)`、`$…$`、`x .* y` 这类歧义或纯排印记法一律直接拒绝。
- 隐式乘法只在 LaTeX/论文表面成立（单字母体系在构造上无歧义）；MATLAB 表面要求显式 `*`。
- 模式矩阵只支持三种可证明正确的形状：`identity(n)`、`diag(d, n)`、
  `tridiag(a, b, c, n)`；任何无法识别的东西 = 错误 + 这三种的清单。
- 源文件可以手写（组合符等），但 **IR 必须规范化**——组合字符永不进入 IR。

## 向量代数 / Horner / no_std（2026-11-23）

**`dot(a,b)` / `norm(v)`**：只在**具体字面量**上展开（形状在编译期已知）；
`dot` 的量纲是两边的乘积，`norm` 与其元素同量纲。`dot([x,y],[x,y])`（x=3,y=4）→ 25，
`norm` → 5。形状不符一律直接拒绝。

**自动 Horner 转换**：源码中写成 `c₀ + c₁x + c₂x² + …` 的**显式多项式**在输出前会被转成
`(…((cₙx + cₙ₋₁)x + …) + c₀)`——乘法次数从 `2n-1` 降到 `n`，像 `x^n` 这样的大幂会消失
（数值上更稳定）。`a*x^3 + b*x` → `(a * x * x + b) * x`。
**紧凑形式不展开**：`(u-1)^3` 保持原样（展开只会更难读）；导数也是对
**Horner 之前的原式**取的（对 Horner 形式求导会得到一串相互抵消的乘积）。

**`no_std` 车道**：`himari_buildhelper::generate_dir_libm` 生成的代码只用 `core` + `libm`
（超越函数 → `libm::exp` 之类，整数次幂 → `libm::pow`；`f64::powi` 是 std 的东西）。
这指的是**生成的代码**可用于 `no_std`——而自 2026-11-23 起，编译期栈也是：
`himari`、`himari-latex` 与 `himari-matlab` 是 `no_std + alloc`，放在默认 `std` feature 背后（除非你关掉，`std` 是开着的）。
loader、buildhelper、macros 与 check crates（文件 IO、proc-macro 宿主、工具）仍为 `std`，并显式开启该 feature。
生成的代码保留**自己的 libm 车道**，那才是裸机消费者实际链接的东西。
门禁：一个 `#![no_std]` fixture crate（`[lib] test = false`，因为测试框架需要 std），它既编译
生成的代码，**又**在没有 std 的情况下跑通解析器链（`himari_matlab::parse` → `dims_of_set` → `shapes_of_set` → `dim_check_runtime` → `format_set`）。
**`cargo build` 通过 = 生成的代码与前端没有偷偷伸手去拿 std。**
已验证的裸机目标：`riscv32imc-unknown-none-elf`（只有 core——没有可用的原子操作）、`thumbv7em-none-eabi`，以及
`x86_64-unknown-none`（解析器链）。这条车道不输出 Jacobian（需要 `alloc`），也不输出 f32 变体
（f32 超越函数是 libm 的 `*f` 家族，而我们拒绝只交付半条车道）；要偏导，请用每个公式的 `__d_*` 函数
（纯标量，零分配）。

## 跨文件公式库：`use "..."`（2026-11-23）

```text
use "lib/atmosphere.formula"      % file header, may span several lines
decl { ... }
v_terminal = sqrt(2 * m * g / (rho * Cd * A))
```

- **解析只负责收集，IO 属于 loader**：前端保持纯字符串 API（`FormulaSet.uses`），
  而读文件、递归内联、环检测与冲突解决由 `himari-loader` 负责。
- 合并语义（简单可预测）：被引用的 decls/items **排在最前面** → 联邦照常工作（后面的条目可引用前面的条目）；
  完全相同的重复 decl 去重，拼写不同则报错；重复的条目名一律报错（**没有“后写覆盖”**）；
  环报错，并打印环绕环的文件名链。
- **库是“被代入”而不是“被调用”**：`rho` 一旦展开到调用方，参数表里就有 `rho_0 / h / H`。
  这是刻意的（没有隐藏状态，可静态微分）；代价是参数表变长。
- 目录约定：`generate_dir` 只扫描**最顶层**的 `*.formula`，所以 `formulas/lib/*.formula`
  自然只作为库存在，永远不会被当作顶层公式再生成一次。
- **方言来自宏，不靠猜**（2026-11-24 用户裁决，“宏就是方言”）：包住公式的是哪个宏，
  就由哪个文法来读它。

  | 宏 | 方言 |
  | --- | --- |
  | `formula!(…)` | **matex**（规范表面；入口名保留，语义已定） |
  | `formula_latex!(…)` | LaTeX |
  | `formula_matlab!(…)` | matex（与 `formula!` 同义，读起来更自明） |
  | `formula_file!("f.formula", latex)` | 由**必填**的第二个参数决定 |

  `formula_file!` 不把方言参数设为可选：文件内容无法从宏名推断，而让宏去猜
  只会把歧义藏到别处。

  **为什么这不只是风格偏好**——它消除了一处真实的静默错误值。当 `si` 已声明时，`si * x`：
  matex 读成 `si·x`，LaTeX 读成 `s·i·x`。对同一文件、同一组输入实测（s=3, i=5, x=2, si=100）：

  ```text
  old AUTO (try matex first, use if it succeeds)  →  200    without reporting any error
  explicit LaTeX                                  →  30
  ```

  两者相差近七倍。AUTO 无从知道作者指的是哪一个——它“猜”到的只是哪个解析器先成功。
  这就是宏车道的 `Dialect` 枚举**没有** `Auto` 变体的原因：类型系统保证猜测回不来。

## 快速开始：`examples/reentry`（最小可运行项目）

```text
examples/reentry/
  formulas/entry.formula       % terminal velocity / dynamic pressure / drag (aerospace side)
  formulas/structures.formula  % static displacement = solve(k·x − F, x)
  build.rs                     % three lines: generate_dir("formulas", "src/generated.rs")
  src/main.rs                  % prints the table + physical invariant self-checks
  src/generated.rs             % the checked-in generated code (reviewable, covered by the drift gate)
```

`cargo run -p himari-example-reentry` 打印一张速度→动压→阻力的表，并验证：

- **物理不变量**：`drag(v_terminal) == m·g`（相对差 1.6e-16——这不是拟合，这是定义）
- **解析导数对照差商**：`∂drag/∂v` 在两条路径间以 1.2e-8 的相对差吻合
- **符号求解**：`x_static` 来自 `solve(k·x − F, x)`，且 `∂x/∂F = 1/k` 自动成立
- **f32 变体**与**库内联公式的系统 Jacobian** 都在同一个生成文件中

消费者现在只需要 `himari-buildhelper::generate_dir(dir, out)`——整条 build.rs 车道
（解析 / 联邦 / 量纲 / 形状 / 导数 / f32 / 批次入口 / Jacobian / 源码 LaTeX）收进一个函数，
每个项目不必再抄 300 行。

## 常见示例：`examples/common`（真实生成 + 独立数值参考）

```bash
cargo run -p himari-example-common
cargo test -p himari-example-common
```

| 公式文件 | 示例 | 生成的入口与验证 |
| --- | --- | --- |
| `matrices.formula` | 旋转、转置、`AᵀA`、2×2 逆矩阵、`A·A⁻¹`、矩阵乘列向量 | `rotation_orthogonality` / `N` / `stiffness_inverse` / `nodal_forces`；逐格对照手算 |
| `springs.formula` | 三节点弹簧平衡方程组 `solve([…],[u₁,u₂,u₃])` | `displacements` 对照另一套带部分主元的 Rust 高斯消元 |
| `runtime_matrix.formula` | 运行时 `A`/`b`：`solve(A,b)`、`A*x`、`inv(A)`、1 为起点的索引 | `runtime_displacements` / `runtime_traction` / `runtime_inverse` / `runtime_element`；每次调用都可失败（`Result`），主元在 `[[1e-17,1],[1,1]]` 上得到证明 |
| `least_squares.formula` | 直线拟合的正规方程 | `line_fit` 对照直接的最小二乘闭式，给出 `y=1.04+1.99x` |
| `roots.formula` | 二次式，以及 `b²≫4ac` 时的小根 | `quadratic_roots` 检查 `1,2` 与 `10⁻⁸,10⁸`（防止相消）；返回 `Result`，负判别式为 `NumericError::NoRealRoots` |
| `circuit.formula` | `I=V/(R+j·2πfL)` | `current` / `current_magnitude` 检查 `|I|=0.995037 A`；`2π` 显式写出 |
| `dynamic_matrix.formula` | 动态 `n×n` 三对角模板 | `diffusion_stencil(k,n)` 逐格对照用循环构造的参考 |
| 内联宏（`src/main.rs`） | 带 `a/b` 的 `formula_matlab!`，以及运行时形状的 `solve(a, b)` | 真正传入 Rust 局部变量；`m/m=1` 在编译期检查；运行时求解做数值验证 |

这些示例证明**当前支持的形式**真的能从 `.formula` 走到可执行的 Rust；它们不把
`solve` 冒充成通用数值求解器，静态示例也不需要运行时矩阵参数。
生成的 `src/generated.rs` 已入库，并受漂移门禁保护。

## 标准车道：`himari-standards`（2026-11-23）

`packages/standards` 把少数几份**已发布的工程标准**变成带类型的计算器，一个模块对应一条条款级关系，每个都
带自己的 `PROVENANCE`（标准 id、版次、公式出自的条款，以及*未*建模的部分）。这里没有任何一处引用
标准原文，也没有任何一处声称符合该标准——它只是把公式重述成一个普通表达式。单位换算
刻意缺席：调用方传入 SI 基本单位。

| 模块 | 标准 | 版次 | 计算内容 |
| --- | --- | --- | --- |
| `iso_6506_1_2014` | ISO 6506-1 | 2014 | 布氏硬度 `HBW`（外加压痕平均压力，单位 Pa） |
| `iso_6892_1_2019` | ISO 6892-1 | 2019 | 抗拉强度 `Rm`、断后伸长率 `A`、弹性模量 `E` |
| `iso_2533_1975` | ISO 2533 | 1975 | 对流层 `T`、`p`、`rho` 与位势高度换算（0 … 11 000 m，超出即拒绝，不做外推） |
| `gb_t_231_1_2018` | GB/T 231.1 | 2018 | 布氏硬度 `HBW`；现行版，代替 GB/T 231.1-2009，修改采用 ISO 6506-1:2014 |
| `gb_t_228_1_2021` | GB/T 228.1 | 2021 | 抗拉强度 `Rm`、伸长率 `A`、弹性模量 `E`；现行版（2022-07-01 生效） |
| `gb_t_8170_2008` | GB/T 8170 | 2008 | 十进制舍入，**四舍六入五成双**，在十进制字符串上决定——`f64` 永不参与舍入决定 |

两个 GB 力学试验模块复用 ISO 的算术（同一套受测的求值），但保留**自己的** id、版次与出处，所以
响应永远不会被贴上错误的标准。对于 `GB/T 228.1-2021`，它对任何 ISO 文件的采用程度**并未**独立
验证，所以该模块明确不声称等效——只实现了这三个定义。版次已对照
国家标准信息公共服务平台核查，每个模块都在其出处备注中记录了这次核查。

CLI `himari-standard` 求值一个请求并打印一份形状稳定的响应文档（JSON），把请求读成 JSON 或 TOML——
从 `--input <path>` 或 stdin，`--format json|toml` 强制指定解析器，`--list` 列出确切的 id 与版次：

```bash
himari-standard --input packages/standards/examples/brinell.json
himari-standard --input packages/standards/examples/tensile.toml
himari-standard --list
```

`packages/standards/examples/` 下的这三个示例请求由测试套件实际执行，
所以文档里的调用方式不会腐坏。

未知键会被拒绝而不是忽略；每次失败都返回同一份文档，`values` 置为 `null`，原因放在 `errors`
（退出码 1，stderr 上一行诊断）；退出码 2 保留给命令行误用。小数会参与**舍入决定**的请求，
必须把那些数字以十进制字符串传入，因为二进制浮点在 crate 看到它之前就已经丢掉了末位。

## 证明车道：`himari-lean`（2026-11-23）

`packages/lean` 是一个 Lean 4 **协同进程**：它生成一个自足的证明义务，通过 stdin 交给真正的 `lean` 可执行文件
（`lean --stdin --json -T <heartbeats> -M <MB>`），并用一份结构化文档报告发生了什么。没有 Lake 项目、没有包索引、
没有网络；生成的文件没有任何 import。

- **动态数据只会变成字面整数**，出现在*具名的示例定理陈述*中——永不变成数据定义、公理或标识符。
- **由 `decide` 经内核检查**：义务在 `Int` 上封闭，并由 `by decide` 解除。不输出 `native_decide`、`#eval`、
  `implemented_by`、`extern`、`partial`、`sorry` 或用户公理，而有一个测试针对这份清单断言生成的文本。
- **公理门禁**：解析 `#print axioms` 的输出，只容忍 `propext`、`Classical.choice` 与 `Quot.sound`；无法读取的报告即为
  违规，永不视为通过。没有报告不是干净的证据。
- **独立神谕**：同一个行列式在 Rust 中用*不同*的展开式计算，所以 Lean 永远不是唯一的证人。
- **`lean_missing` 是一等的非成功**：它表示工具链不在，**什么都没检查**。它永不渲染为
  “已验证”，也永不以 0 退出。状态与退出码：`success` 0、`lean_missing` 3、`invalid_request` 4、`proof_failed` 5、
  `resource_exhausted` 6、`axiom_violation` 7、`oracle_mismatch` 8。

**已对真实工具链验证（Lean 4.36.0，commit `611c117b`）。** 里程碑义务被内核接受，且完全未报告任何
公理：

```text
status = success            verified = true       exit = 0
lean_message: 'det3_example' does not depend on any axioms
oracle:       rust-i128-permutation det3([1,2,3,4,5,6,7,8,10]) = -3
```

负对照在同一二进制下如设计般运作：一个断言 `expected = -306` 的请求返回 `oracle_mismatch`（退出码 8），
永不报告为已验证，而 stub-process 测试套件覆盖 `proof_failed` / `axiom_violation` / `resource_exhausted` 路径。

供应在本仓库之外，且是一次性的：安装 `elan`、钉住 `leanprover/lean4:v4.36.0`，然后把 `lean` 放到 `PATH` 上，
或用 `HIMARI_LEAN_BIN` 指向它。`just lean-verify` 生成义务并运行真实 Lean 测试。**没有工具链时，这条车道降级为
`lean_missing`（非成功），而 `--emit` 仍会写出可审阅的 `.lean` 文件**——Lean 的缺席永不报告为验证。

## 健壮性与对抗式输入（2026-11-23）

- **深度上限 2048**：每个解析出口检查一次 IR 深度（`check_depth`），超过上限的东西会报告
  “formula nested too deeply (N levels > limit 2048)”，而不是让某个下游递归爆栈（SIGSEGV）。
  pest 自带的栈上限也会触发，其消息同样被翻译成平实字句。**已知边界**：长链 `a + b + c + …` 在
  IR 中是左结合嵌套，所以深度等于项数；单个公式的项数因此上限约 2048。
  另有一道独立的确定性括号嵌套护栏（256 层，`MAX_PARSE_NESTING`）在解析前运行，因为它必须在 pest 的
  可选 `stack-guard` feature 不可用时（`no_std`）也成立。
- **不可见字符一律拒绝**：零宽空格 / 双向覆盖字符 / 零宽连接符 / BOM / NUL 全部
  在解析前具名拒绝（`scan_source`）——它们能让公式“审阅时看起来与实际情况不同”。
- **Σ/∫ 步数上限 10^7**：`Σ(i,1,1e9)` 从“跑一整夜”变成一行错误消息。
- **缺参数的函数调用**不再通过 `args[0]` 索引 panic：三条车道（求值/生成/量纲）都明确拒绝。
- **结构化属性测试**（不是随机字符大杂烩）：600 个随机表达式各过三道门禁——规范写回重新解析为
  **同一棵树**、LaTeX 写回重新解析为**同一棵树**、且 `simplify` **保持值不变**。
  三者目前都显示 0 个不等价（`prop-roundtrip`）；一个累加 1500 项的大输入在 100 ms 内完成。

## 单精度（f32）车道（2026-11-23）

每个**实数标量**公式额外输出 `{name}__f32(p: f32) -> f32`——同一个公式、
同一套生成规则，只把字面量换成 `f32` 后缀（`Fx::F32`，贯穿生成器的唯一开关）。
实测 f32/f64 = **1.44×**（`SIMD` 8 条 vs 4 条 + 一半带宽），
游戏引擎 / 音频 DSP / 实时 SDR 可直接受益。

复数与矩阵值公式**不**输出 f32 变体：复数车道的 `(f64, f64)` 元组与矩阵的
`Vec<Vec<f64>>` 在单精度下是不同的类型契约；那可以以后再做（我们拒绝只交付半条车道）。

## 开发者工具（2026-11-23）

三个入口一起交付（都在 `himari-check` 包里，共用一个解析器）：

| 入口 | 用途 |
| --- | --- |
| `himari-fmt --diff` | 只显示规范形式与当前形式的差异（不重写） |
| `himari-fmt --diagnose` | 只做解析 + 量纲 + 形状诊断（干净时无输出，退出码 0） |
| `himari-fmt --symbols` | 列出 `decl` 名字与公式名字（编辑器补全的数据源） |
| `himari-eval <file> --set k=v …` | **求值**：`Re = 200000 [1]`；`--groups` 另打印 Buckingham Π 组；`--json` 给机器读 |
| `python3 scripts/fmt_check.py --fix` | 一条命令重排规范形式 + 重新生成 sidecar |

**错误带有脱字符**：`FrontError::render_with_source` 打印源码行，并在出错列下方画一个 `^`
（三条车道——build.rs / fmt / eval——都经过它）——不用再自己数到第 17 列。

**VS Code 扩展**（`packages/vscode`）有两条通道：

| 通道 | 触发时机 | 数据源 |
| --- | --- | --- |
| CLI（默认，零安装） | 打开 / 保存 | `himari-fmt --diagnose` + `--symbols` |
| **LSP**（可选） | **打字时的错误** | `himari-lsp`（stdio JSON-RPC） |

`himari-lsp` 是手写的最小服务器（不依赖任何 LSP 框架）：`initialize` /
`didOpen` / `didChange` / `completion` / `codeAction` / `shutdown`，支持**增量文本同步**。
其诊断携带与库相同的 `himari::Error`，以 `HIMARI_LANG`（回退 `LC_ALL`、`LANG`，再回退英文）选择的语言渲染；九种项目语言全部可用。
协议层把状态与 JSON-RPC 参数当作纯函数处理，并有一个端到端测试，真的启动一个进程并真的发送
`Content-Length` 帧。在设置里设 `himari.lspPath` 即可切换（留空 = CLI 通道）。

补全候选 = 文件中的 `decl` 名字 + 公式名字 + 一元函数表（含 `det/inv/solve/dot/norm` 等）。

## LaTeX 表面：多字母名字需要 `\mathrm`（2026-11-23）

在 LaTeX 数学模式中，裸写 `sigma` 是**五个相乘的变量 s·i·g·m·a**（隐式乘法），
所以这个方言中的多字母名字必须写成 `\mathrm{sigma_0}`——写回时也这样输出。
希腊字母名（`sigma`、`omega`……）写回为**真实字形** `\sigma`，保留下标
（`\sigma_0`）。实现以前是子串替换，所以 `alphabet` 变成 `\alphabet`、
`betamax` 变成 `b\etamax`，而 `sigma` 被渲染成五个斜体字母——三处同时出错（悬停 / sidecar / 文档事实来源）。已修复（`latex_var`）。

## 错误：一个枚举，九种语言

crate 里每个失败都是同一个枚举 `himari::Error`。它不是一堆字符串：每个失败是一个**自带数据**的变体，
因此同时给出三件事。

- **稳定码** —— `err.code()` 返回 kebab-case 标识（`dimension-mismatch`、`undeclared-symbol`、
  `no-real-roots`……）。测试与调用方断言码，所以改措辞或换语言都不会敲碎它们。
- **按语言渲染** —— `err.render(locale)` 与 `Display`（英文）从 [i18n.rs](packages/himari/src/i18n.rs)
  的目录生成句子。CLI 与 LSP 边缘用 `HIMARI_LANG` / `LC_ALL` / `LANG` 选择；九语即组织的
  （`en`、`zh-Hans`、`zh-Hant`、`ja`、`ko`、`fr`、`es`、`ru`、`ar`）。
- **与 `anyhow` 兼容** —— `Error` 实现 `core::error::Error`（`std` 下即 `std::error::Error`），
  且是 `Send + Sync + 'static`，不需要任何包装：

```rust
  use anyhow::Context as _;

  fn run() -> anyhow::Result<f64> {
      let dim = himari::parse_unit("N/m")?; // `?` converts automatically
      let e = himari::Error::NoRealRoots { discriminant: -4.0 };
      let err = anyhow::Error::from(e).context("solving the loop equation");
      let root = err.downcast_ref::<himari::Error>().expect("root cause kept");
      assert_eq!(root.code(), "no-real-roots");
      assert_eq!(dim, himari::parse_unit("kg/s^2")?);
      Ok(1.0)
  }

  fn main() {
      assert_eq!(run().unwrap(), 1.0);
  }
```

`tests/anyhow_compat.rs` 把这几件事都钉住：`?` 传播、`Send + Sync + 'static` 边界、
`anyhow::Context` 之后仍能 `downcast_ref::<himari::Error>()` 拿回稳定码，以及「同一错误在九语下
各不相同、而码始终不变」。

`Error::Message(String)` 保留给永不进入目录的消息：它在任何语言下都原样输出。它所代表的分期债**由门禁计量**而非散文——而这笔债目前为零：

```text
python3 scripts/staging_budget.py                # counts + ceiling
python3 scripts/staging_budget.py --against-git  # also: the ceiling may only go down vs the merge base
```

门禁是一次 AST 遍历（`tools/staging-budget`，基于 `syn`），计数不依赖行布局、空白或注释，`matches!(e, Error::Message(m) if …)` 这类**模式**绝不会被误判为构造。门禁对工作区每个生产树（`packages/`、`examples/`、`tools/`）统计三个家族的十二个桶：六种字面构造形态（`Error::from(`、`Error::Message(`、前端的 `FrontError::plain("…")` / `plain(format!(…))` / `FrontError::new(…, "…")`）、价值洗白漏斗（携带运行期值载荷的 `plain` / `new`，以及作为函数引用传入的 `FrontError::plain`，两者皆为全仓范围）、以及目录 crate 自身内部的字符串暂存形态（产生暂存字符串的 `ok_or`/`ok_or_else`/`map_err`，以及 `Err(format!(…))`）。仅在测试与目录下编译的项被跳过；带字符串载荷的已编目变体（如 `Error::SyntaxError(text)`）不是暂存，不计入。若总数超过记录上限，门禁即失败（上限记录在 `tools/staging-budget/src/ceiling.txt`，只降不升；当前为 `0`）。上限只能下调，且这一点是强制的：CI（及 `just verify`）以 `--against-git` 运行门禁，读取**与远端 trunk 的 merge base**处的 `tools/staging-budget/src/ceiling.txt`，工作树的上限更高即失败——所以新点位不可能随抬高上限一起混进来。上面的裸命令只检查计数。全部 51 个变体由 `catalogue_completeness_tests` 钉住——50 个目录变体加上暂存用的 `Message`，逐个构造并在九语下渲染（非空、无泄漏的 `{n}`）。LSP 快速修复的*标题*仍是中文；其诊断已本地化。

### 编辑器集成（LSP）

`himari-lsp` 是手写的 LSP 服务（协议层是纯函数，附单元测试 + 一个端到端 stdio 测试）：

| 能力 | 说明 |
| --- | --- |
| 诊断 | 在 `didOpen`/`didChange` 之后推送 `publishDiagnostics`（未声明符号、量纲不符、语法错误） |
| 补全 | 文件中的 `decl` 名字 + 公式名字 + 一元函数表（`det/inv/solve/roots/...`） |
| **快速修复** | `textDocument/codeAction`：未声明符号 → ① 改写成最近的已声明名字 ② 在 decl 块加一行；函数名拼错 → 从注册表给出正确名字 |
| 同步 | **增量同步**（`textDocumentSync: 2`，按范围切片）；也接受全量变更，所以旧客户端不会坏 |

快速修复只在名字**恰好出现一次**时提供替换（出现多次时只提供“添加声明”，永不
猜是哪一处）；越界的值一律**钳制**而不是丢弃改动（丢掉一个会让缓冲区与编辑器永久
失步）。

### 公式库内容锁：库一变就必须出声

`use "lib/…"` 会把库文件**内联**进来，所以库一改，下游生成的代码也跟着改——正是那种
最容易漏掉的回归。`--lock` 把每个被引用文件的指纹写进 `<entry>.lock`，此后**每次加载都校验锁**：

```text
$ himari-fmt --lock formulas/entry.formula
已写入 formulas/entry.formula.lock（4 行）

$ himari-eval formulas/entry.formula         # someone quietly changed the library
himari-eval: 公式库与锁文件不一致（formulas/entry.formula.lock）：
  内容变了（声明版本 1.4.2 → 2.0.0）：lib/atmosphere.formula
              复核改动后运行 `himari-fmt --lock <入口>` 重新生成锁。
```

（CLI 输出逐字引用：工具目前仍以中文打印。英文意思是：库与其锁文件不一致——
`lib/atmosphere.formula` 的声明版本从 1.4.2 变成 2.0.0——审阅改动后重新运行
`himari-fmt --lock <entry>`。）

- **sha256 内容哈希**（不是 semver 范围）：对于“内联源码”这种依赖形态，内容哈希是
  更强的保证——任何改动都能抓到，不需要版本解析策略，也没有预发布规则。
- **纯文本、确定性、路径相对入口目录**：可以进 git，diff 可读，且跨机器可比。
- **闭环门禁**：`scripts/fmt_check.py` 会运行 `--lock-check`，对象是带 `.lock` 的文件
  （并在 `--fix` 下自动重新加锁）——过期的锁在门禁里变红，不必等有人加载该文件。
- **选择加入**：没有 `.lock` 就不校验（对既有仓库零影响）。一旦有锁，**新的引用也会失败**
  （白名单语义）；重新加锁才能接受。
- **无法读取的锁也是失败**：不是 UTF-8（被编辑器存成 UTF-16）、权限不足，或是一个目录路径
  都是硬错误——这些永不可被当成“没有锁”，否则会悄悄关掉保护。
- **`--fix` 分三个阶段**（先把所有东西格式化 → 再把所有东西重新加锁 → 最后重新生成每个 sidecar）：
  sidecar 加载时会校验锁，所以锁必须先定稿；两者也都依赖被引用库的最终内容，
  把它们混进同一个循环会留下过期产物。
- `%! version: 1.2.0` 只是给人看的列（它也被加锁，版本变更会被报告）；语义保证靠的是哈希。
- 为什么不用 semver 范围：那需要一整套解析／钉版／预发布策略，而本仓库的依赖形态是
  “内联源码”。要表达“允许小幅漂移”，重新加锁是更直接的说法。

### 方言指示：文件车道上对应“宏就是方言”的规则

文件没有宏来包住它，所以对应的规则是：**默认 matex；LaTeX 显式声明。** 两条车道共用一个原则——
不猜，作者（而不是解析器）决定语义。

```text
%! dialect: latex
decl {
    s : d;
    i : d;
    si : d;
    x : d;
}
q = si * x        % LaTeX semantics: s·i·x (not the declared si·x)
```

- 支持 `%! dialect: latex`（或 `tex`）与 `%! dialect: matex`（或 `matlab`）。
- **没有指示 = matex**。这里以前有一个 AUTO 兜底（matex 解析失败且源码含 `\` 就试 LaTeX），
  它恰恰制造了上面那个 200 对 30 的静默错误值——已移除。
- 没有方言指示的 LaTeX 专属记法 → 按默认 matex 解析 → **错误**（绝不会是静默的错误值）；
  错误会给出出路（写 `%! dialect: latex`），但**不会替你猜**。
- 只在**第一个非注释行之前**生效（正文注释中的 `%!` 不改变方言）。
- 无法识别的值是错误。
- 有指示时，错误以该方言报告：LaTeX 指示下的语法错误就是 LaTeX 错误。
- 它只在一个地方实现（`himari_loader::parse_source`），所以 build.rs 车道、`himari-fmt`、
  `himari-eval` 与 LSP 诊断全部一致。（LSP 诊断按 `HIMARI_LANG` 选择的语言渲染，见“错误”一节。）

### LaTeX 表面的除法（曾经缺失）

`a / b` 与 `a \div b` 现在是真正的除法，与 `\frac{a}{b}` 同义，优先级与乘法相同且左结合。
以前 LaTeX 表面**完全没有除法运算符**——只接受 `\frac{a}{b}`，而 `a / b` 是语法错误。
那是把洗澡水和孩子一起倒掉：`/` 是写除法最平常的方式，没有理由拒绝它。
（上面那个 AUTO 兜底其实一直在遮掩这个缺陷：它让 `a / b`“碰巧能用”，同时把静默错误值一起带进来。
修好缺陷，兜底就没有存在的理由了。）

## 符号线性求解 `solve`（2026-11-23）

单变量形式 `solve(equation, variable)` 把 `equation = 0` 解成 `variable = -b/a`（方程必须对该变量是**线性**的）。

```text
ES_ss = solve(k_on * E * S - (k_off + k_cat) * ES, ES)
      → ES_ss = k_on·E·S/(k_off + k_cat)          % Michaelis–Menten steady state
```

- 解**仍然是 IR**：所以它保留量纲检查、求值、代码生成、解析导数与
  Jacobian——不需要第二套机制。`ES_ss` 的偏导与 Jacobian 行会自动产出。
- **参数表 = 生成表达式实际用到的变量**：被求解的 `ES` 不会出现在 `ES_ss` 的参数里
  （“要求它却又必须传它”已修复）。Sidecar / 悬停 / 补全读的是同一份参数表。
- **非线性被明确拒绝**：`x·x`、`1/x`、`x^2`、`sin(x)` **在解析出口**报错，并指出涉及的类别；
  没有数值近似，没有可疑的答案。把化学稳态算错的代价，远高于多打几个字符。

### 方程组形式 `solve([equations…], [unknowns…])`

当两个参数都是列表时，进入**线性方程组**车道，解是一个**列向量**（矩阵表达式值车道，
返回 `Vec<Vec<f64>>`）：

```text
x = solve([(k1 + k2) * x1 - k2 * x2 - F1,
           -k2 * x1 + (k2 + k3) * x2 - F2], [x1, x2])
  → x = 2×1 column vector, dimension [m]           % two-mass spring statics
```

- **符号消元**：高斯–约当消元在**表达式环**上运行（系数是 IR 子表达式，不是数），
  每一步都化简到规范形式。解出的每个分量仍是 IR，照常走量纲/求值/生成。
  **但方程组的解是矩阵值的**：矩阵值公式不生成解析导数（逐元素偏导不是矩阵；Jacobian 才是），
  也不生成 f32 变体——这是矩阵值车道既有的限制，不是 `solve` 特有。
- **形状约定**：方程与未知数列表都按行主序**展平**，所以 `[e1, e2]` 与 `[e1; e2]` 等价——
  用户不必记住方向。未知数列表的条目必须是裸变量名。
- **未知数需要已声明的量纲**：换来的是**结果量纲的检查**（弹簧示例解 `m = N/(N/m)`），
  以及方程本身的**齐次性**检查（`x1 − F1`，把 m 加到 N 上，在解析期就被拒绝）。
  解出的未知数不进入参数表（生成的函数只接收真正的参数）。
- **拒绝就是特性**：欠定（非满秩）、不相容（消元后出现 `0 = nonzero`）、非线性（未知数出现在某个乘法的两边 /
  分母或指数中 / 函数参数内）**全部在解析出口报告**。
  与量纲相关的拒绝发生在 `dims_of_set` / 生成阶段（解析器本身不做量纲检查）。
  冗余方程按“可折叠意义”移除：系数为符号、无法折叠到零的冗余行，保守地
  报为不相容——宁可拒绝，也不返回错误的解。
- **主元非零与数值边界**：与单变量 `solve` 同一约定，无法证明为零的就当作
  非零（我们信任 `a ≠ 0`）。消元在编译期按表达式顺序进行，**运行时不会按 |主元| 重排行**；
  在极端尺度下闭式会相消，奇异参数在值层报告除以零，生成的代码按 IEEE 给出 `inf/NaN`
  ——那不是异常。病态方程组，或需要主元策略的方程组，必须交给数值库。
- **边界（四条，全部在解析出口或作为编译期错误拒绝）**：

  1. **矩阵值**的系数或常数项（`solve(M*x - b, x)`、`solve([M*x - b], [x])`）→ 明确拒绝，
     并提示改用 `inv(A) * b`——否则标量车道会生成 `vec!` 除法，编译不过。
  1. **方程可以有不同的量纲**（N 的力平衡加上 m 的几何约束是合法的方程组），但每个方程本身必须
     齐次；且解的每个分量必须同属**一个量纲**（混合量纲的未知数——同时解电流与
     电阻——尚不支持；那是矩阵值车道的限制）。
  1. 复数车道（带 `j`）在**值层**与**生成层**都能工作（2026-11-23 完成）：复数列向量
     输出为 `Vec<Vec<(f64, f64)>>`，元素为（实部, 虚部）。交流电路节点分析与 EIS 风格的“复数方程组”
     因此可以直接生成成代码。
  1. **冗余行的零检验是充分条件**：同类项合并能证明的零（`k−k`、`k·a−a·k`、`x·x²−x³`）
     会被识别；无法证明为零的复杂恒等式会当作非零系数（保守——可能误报
     欠定/不相容，但不会返回错误的解）。

### 多项式根 `roots(poly, x)`

`solve` 给出线性闭式；`roots` 给出**多项式的所有根**（作为列向量）：

```text
roots_quad = roots(a * x ^ 2 + b * x + c, x)
% q = −(b + sign(b)·√(b²−4ac))/2; large root = q/a, small root = c/q
% Return order still matches the original formula (−b−√Δ first); it does not depend on sorting the roots by size.
```

- **只支持 1–2 次**：1 次与 `solve` 是同一门数学；2 次是闭式求根公式。
  **3 次及以上明确拒绝**，并报告实际次数——Cardano/Ferrari 在浮点下条件数极差，
  本仓库不会发一个“看起来很精确”的近似。三次及以上的根，请先数值算出系数，
  再用数值库。
- **重根按重数重复**（`x² − 2x + 1` 给出两个 1），与代数基本定理一致；
  零根时的 `q=0` 兜底是 `-b/(2a)`，不会走 `0/0`。
- **二次公式的相消规避**：直接使用 `(−b±√Δ)/(2a)` 在 `b²≫4ac` 时会丢掉小根
  （`x²−10⁸x+1` 曾把小根 `10⁻⁸` 算成约 `7.45×10⁻⁹`）。现在使用稳定的 `q/a` 与 `c/q`，而生成代码中
  小根相对误差的验收线是 `1e-12`；两个符号方向与复数车道都有回归测试。
- **量纲自动得出**：方程必须量纲齐次，而齐次性直接蕴含“根的量纲 = 变量的
  量纲”，所以不需要额外推导量纲（有一个契约测试守着这个推理）。
- **实数车道的负判别式**：生成的代码返回带类型的错误，**不是 NaN**——实数二次 `roots` 入口现在返回
  `Result<_, himari::runtime::NumericError>`，负判别式是 `NumericError::NoRealRoots { discriminant }`。不再有静默
  NaN 契约，所以 `is_nan`/`is_finite` 检查不再是检测这种情况的方法；编译期可证明为负的常数判别式
  仍然直接拒绝。`eval_value` 是通用复数值层，对复数车道中的同一表达式**仍会返回复数根**；
  复数车道需要显式的 `j`。
- **相消的高次项会被剥掉**：`x³ − x³ + x − 1` 当作 1 次处理（而不是“多项式中出现 x³ 就拒绝”）。
- **非多项式明确拒绝**：`x` 出现在函数参数内、分母中，或非整数次幂——各自附上理由。
- **矩阵值系数**被拒绝（与 `solve` 同一约定，否则标量车道会生成 `vec!` 除法）。

### 系数向量 `poly_coeffs(poly, x)`

`roots` 的互补：它给出**系数**（由低次到高次），返回 `1×k` 行向量。

```text
coeffs = poly_coeffs(2 * s ^ 2 - 3 * s + 1, s)   → [[1, −3, 2]]
```

- **变量必须无量纲**。齐次多项式第 k 次系数的量纲是 `dim(poly)/dim(x)^k`，而它们各不相同；
  但矩阵要求**元素同量纲**——所以对有量纲的变量取系数，在那个形式下代数上不可能。
  这道检查在诊断阶段直接说明，并给出出路（除以一个特征量，或改用 `roots`），而不是让
  下游报“矩阵元素量纲不同”。
- 高次零系数会被剥掉（`x³ − x³ − 2x + 1` 是 1 次）；零多项式被拒绝。
- 非多项式（`sin(x)`、`1/x`、非整数次幂）被拒绝。
- 它共用同一个 `poly_coeffs` 提取器，`roots` 也不例外——系数语义只有一个定义，两个使用者。

### 三次以上的根：`himari-eval --roots`

在语言中拒绝 ≥ 3 次是必要的取舍，但**数值**求根对任何次数都有效，所以它放在工具层：

```text
$ himari-eval cubic.formula --roots
cubic = 1, 2, 3  [1] (degree 3, 3 roots)
cplx  = -1, 0.5-0.8660254037844387j, 0.5+0.8660254037844386j  [1] (degree 3, 3 roots)
```

- **Durand–Kerner 迭代**（复用 core 的 `C` 做复数算术，不引入第三方数值库）：初值是
  `0.4 + 0.9j` 的幂，半径是**根量级的几何平均** `|c₀/cₙ|^(1/n)`（只在退化情况才退回
  Cauchy 界 `1+max|cᵢ|`）；最多 1000 次迭代；DK 判据是 `|Δz| ≤ 1e-10·radius`（相对根的量级，
  而不是相对 1）；随后是 **60 步牛顿抛光**（相对停止判据 `1e-15·|z|`）。
- **不收敛是错误**，永不把尚未收敛的近似当作结果返回；迭代中出现 `inf`/`NaN` 也是立即错误
  （`f64::max` 对 NaN 返回另一边，这曾让一个 NaN 步看起来“已收敛”并返回 20 个 NaN）。
- **残差验收**：迭代后，相对后向误差 `|p(z)| / Σ|aₖ|·|z|ᵏ` 必须足够小（阈值 1e-8），
  否则报错——DK 不保证收敛到根，而判据必须独立于迭代本身。
  分母**必须包含 `k=n` 项**：没有它，常数 `|a_n|` 就会在小的 `|z|` 下成为分母，
  而 `x²−1e-100` 会通过，却错了 4e25 倍（真实的回归）。
- **牛顿抛光**（相对判据）：单根可达到机器精度；**重根只有线性收敛**，所以 `x²−2x+1` 之类
  的相对误差在 `1e-9` 量级，且不报错——那是算法的性质，不是 bug；不要把重根当成
  单根。
- **次数上限 16**（`poly_coeffs` 的多项式展开上限；`numeric_roots` 本身支持到 64）。
- 迭代前做 **monic 规范化**（除以首项系数）以避免量级溢出；次数上限 64；`inf`/`NaN` 系数拒绝。
- **噪声清零只清虚部，且判据是相对的**：实根会带 `1e-16` 量级的虚部残余，那是
  迭代的产物；判据是 `|im| ≤ 1e-12·|z|`。**实部永不动**（`x²−1e-50` 的根是 ±1e-25），
  而且**绝对阈值也不行**——对量级低于 1e-12 的复根，虚部就是该根的全部信息
  （`x³−1e-60` 曾因此被抹成三个实根，相对误差 0.87，却没报错）。
- 系数在 `--set` 给定的参数下必须求值成数；省略变量时，推断为“公式中唯一声明的
  符号”，有多个候选时需要 `--var`。
- 输出带有次数与根的个数；复根写成 `a+bj`。

## 复矩阵与复数方程组（2026-11-23）

复数性不只限于标量：矩阵元素与解向量也可以是复数，而且**值层与生成层一致**。
生成函数的返回类型是 `Vec<Vec<(f64, f64)>>`（元素 =（实部, 虚部））：

```rust
// I = solve([Z*I1 - V, Z*I1 - R*I2], [I1, I2]) with Z = R + jωL
pub fn I(R: f64, w: f64, L: f64, V: f64) -> Vec<Vec<(f64, f64)>> { … }
```

- 典型用途是**交流电路节点分析**与 EIS 建模：未知数是复数电流/电位。
- 转置就是**纯转置**（不取共轭）——共轭是独立的 `conj` 运算符；别混淆。
- `adjoint` 是共轭转置；只有形状在编译期已知的字面量才能展开。
- 在复数车道，`det/trace/adjoint/inv/solve/roots/dot/norm` **同样**在生成前于 IR 层展开（一个历史上的不对称：
  实数车道会展开，而复数车道直接跳到 `compile_error!`）。
- 仍不支持：复数公式的 **no_std/libm 车道**（那些闭包依赖 std）、复数公式的**解析导数**
  （`abs/arg/conj/re/im` 不是复解析的，那需要 Wirtinger 微积分），以及复数公式的 **f32 变体**。

## WGSL（GPU）车道：子集 + 验证边界（2026-11-23）

`himari-fmt --wgsl <file.formula>` 把实数标量公式生成为 f32 kernel：

```wgsl
// Generated by himari (f32 scalar lane; really compiled and validated by naga)
fn q(rho: f32, v: f32) -> f32 {
    return 0.5f * rho * (v * v);
}
```

**支持的子集（刻意很小）**：`+ - * /`、`sqrt`、`exp`/`log`（本仓库的 `ln`）`sin cos tan
sinh cosh tanh abs min max clamp`，加上 2/3 次幂、平方根，以及 -1 次幂。
**明确拒绝**：复数（`j`）、矩阵、`det/inv/solve`、Σ/∫、符号运算符、其他次幂，
以及与 WGSL 保留字冲突的符号——各自附理由（我们不生成编译不过的 kernel）。

**验证有两层**（自 2026-11-24 起，WGSL 车道有**编译器级**证据）：

1. **真编译门禁**（权威）：`cargo test -p himari-check --features wgsl-compile`
   把 fixtures 中每个生成的 kernel 交给 **naga**（wgpu 的着色器编译器，WGSL 规范的权威实现之一），做
   真解析 + 真验证。naga 是纯 CPU 的 dev-dependency——**不需要 GPU**，且永不进入运行时依赖树。
1. **静态门禁**（快速筛查，`wgsl::validate_wgsl`）：括号平衡、`-> f32` 签名、恰好一个
   `return`、浮点字面量带 `f`、没有 `^`/`**`、没有非有限常量。它的价值在于“在没有 naga 的环境里抓粗错
   + 错误消息更贴近这条车道”；**它不是正确性的证明**。

**为什么真编译门禁是必须的——一个具体教训**：这条 WGSL 车道的每个 kernel 以前都是**无效的**。
WGSL 没有表达式体函数；主体必须是 `return expr;`，而生成器一直只生成裸表达式；
更糟的是，静态门禁**禁止 `return`**——因为那条规则是在不知道规范那部分的情况下写的，把误解
变成了“规则”。静态门禁的结构性盲点不是“它会漏掉东西”，而是“**它不知道自己漏了什么，
甚至可能把正确的代码判成错的**”。第一次 naga 运行就暴露了整个类别。
另一个被纠正的信念：`return 2.5;` 在 naga 中是**合法**的（抽象浮点由上下文落成 f32）——所以“浮点必须带 `f`”是这条车道的
风格约定，不是 WGSL 规范。

**验证边界（诚实说明）**：naga 只验证**语法与类型**。“kernel 在真 GPU 上算出的数对不对”仍然
未验证——那需要 wgpu 运行时 + 一块 GPU（node-3 有一块 RTX 5880，待接线）。
另外，tint（Chrome/Dawn 的实现）与 naga 在少数边界规则上不同；本仓库只对 naga 负责。

## 按领域选择车道（一张表）

| 场景 | 建议车道 | 备注 |
| --- | --- | --- |
| 大规模科学 HPC / 数值库 | `.formula` + build.rs，实数标量 | 与手写相当（1.0×）；定长批次变体可去掉边界检查 |
| 游戏引擎 / 音频 DSP | 同上 + **`{name}__f32`** | 实测 SIMD 增益 1.44×；用 `clamp/min/max` 钳制 |
| 实时 SDR | 实数或复数车道 + 批次 | 卷积/信道矩阵用字面量（常量折叠） |
| EIS / 电化学 | 复数车道（`j` + `(f64,f64)`） | 相位是物理承诺；扫描用批次；**热循环手写** |
| FEM / 有限元 | 矩阵运算符 `det/trace/inv` + Jacobian | `det` 到 4 阶、`inv` 到 3 阶；更高阶改用数值库 |
| 运行时尺寸的线性方程组 | 运行时车道（`_matrix`/`_vector` 声明） | `solve(A,b)` 有检查（rcond + 残差）；返回 `Result` |
| 量纲探索 | `himari-eval --groups` | Buckingham Π 组：不用手推雷诺数 |

> 本仓库**产出**的一切——生成代码的 doc comment、`compile_error!` 的载荷、proc-macro 的 `FORMULA-ERROR` 诊断、WGSL 文件头——都是英文。仅剩的一个例外是 LSP 快速修复的*标题*（其诊断本身已本地化）；那是 UI 字符串，不是错误目录的路径。目录的分期债为零，而上面的 staging-budget 门禁（十二个构造家族，包含条件分支与产生字符串的 `map_err` 引用，上限对照 merge base 强制执行）正是把它守在零的原因。

**带脱字符的错误**、**函数名拼错的建议**（`closest_func`，编辑距离 ≤ 2）、
**拒绝不可见字符**、**深度/步数上限**——这些不是装饰，而是“敢让同事用这个”的门槛。

## 状态

仓库在组织内（`celestia-island/himari`，单一 squash master）。crates.io / docs.rs 徽章是预留的入口。
库 crates 是**可发布的**：`cargo publish -p himari --dry-run` 会打包、用自己的源码重建归档并走到上传那一步，且全部九个 crate 名在 crates.io 上仍然空闲。
剩下的是上传本身——它需要 registry token，而且不可逆（yank 永远不会释放版本号），所以这是一个刻意的、独立的决定，而不是一个构建步骤。
示例、fixtures 与工具类 crates 保持不可发布。
内部依赖已经带着 `version = "0.1"` 与本地路径，因此上传必须按拓扑顺序进行：
`himari → himari-latex / himari-matlab → himari-loader → himari-buildhelper → himari-macros`。
`himari-check` 工具的发行另行决定。`himari-standards` 与 `himari-lean` 依赖 `himari-macros` 与 `himari-loader`，所以不改变那个顺序。
路线图与取舍的历史：`_reports/math-formula-20261122/`（REPORT / SPIKE-S0 / SYMBOLS-AND-PAPER）。

## 验证

```bash
cargo test --workspace --offline
cargo test -p himari-check --features wgsl-compile --test wgsl_compile --offline
cargo run -p himari-check --release --offline
python3 scripts/fmt_check.py
cargo test -p himari-example-common --offline
```

门禁覆盖核心代数、实数生成代码的编译与数值不变量、独立编写的消费方示例、
WGSL 的真 naga 编译，以及规范形式/sidecar/锁与生成代码的漂移。确切的测试数量以当前 `cargo test --workspace` 的输出为准；
性能数字只描述本仓库的 bench 环境，不是跨平台保证。

## 未声称

这些是范围限制，不是一份快要完成的待办清单。写“未声称”的地方就是“未声称”。

- **符号车道没有运行时主元选取**：符号 `solve` 按表达式顺序消元并信任 `a ≠ 0`；它不会按
  主元量级重排行。病态或对主元敏感方程组，属于运行时车道（上面的 `solve(A,b)`）或数值库。
- **模式矩阵只能走文件车道**：模式矩阵的构造子车道需要文件名与生成器，所以内联宏
  会带提示拒绝它们；没有任意模式矩阵的自动展开（那是研究级——内置形状之外的东西一律明确拒绝，
  并附上支持清单）。
- **WGSL 数值验证已测量，但是在软件设备上**：`just gpu-verify` 把生成的 kernel 编译为 SPIR-V、经 Vulkan 执行并与 f64 CPU 通道比较结果（四个公式、十六个点、最坏相对误差 3.2e-7，Mesa lavapipe；替换任一算子都会使该通道失败）。那是执行真实 SPIR-V 的真实驱动路径，因此能抓住错误的索引、常量与优先级——但它**不**测量特定厂商的 f32 舍入、workgroup 调度或驱动 bug，也没有做过任何硬件 GPU 运行。
- **Lean 验证尚未成功**：这台主机没有安装工具链，所以只跑了 stub-process 测试。没有声称 Lean 接受
  生成的证明义务。
- **发布是单独的、不可逆的决定**：库 crates 可发布且 dry-run 干净，但尚无任何上传；第一次上传就是版本号的不归点。见“状态”。
- **`no_std` 范围**：`himari`、`himari-latex` 与 `himari-matlab` 是 `no_std + alloc`；loader/buildhelper/macros/check crates 则不是。
  生成的代码保留自己的 libm 车道，而复数公式不属于 `no_std` 车道。
- **没有复数解析导数、没有复数或矩阵值公式的 f32 变体、没有矩阵值公式的解析导数。**
- **`roots` 在语言中只覆盖 1–2 次**；更高次只能通过 `himari-eval --roots` 数值求解。
- 一般 LaTeX（宏/catcode/排版）——语义不可判定；排版属于渲染层。
- 不从字形推断语义，MATLAB 表面也没有 `xy` 乘法。没有 `\sin^2 x`、没有 `.*`、没有 `$…$`（歧义或纯
  排印记法：解析与渲染是两件不同的工作，一个解析器不该两件都做）。
- 实时/安全关键路径上没有运行时 JIT；永不引入 AGPL 依赖（evalexpr ≥12 之类已出局）。
- **`himari-standards`** 模块是条款级计算器，**不是符合性声称**：它们不对试样几何、力值级别的选择、
  保压时间、不确定度预算，或标准自身的报告规则建模，也不是通用单位库。
- **`himari-lean`** 车道证明一个小小的封闭整数义务（一个 3×3 行列式示例）；它不是通用定理证明器，而它的公理
  门禁是对 Lean 自身输出的解析器，不是健全性论证。
