<!-- markdownlint-disable MD033 MD041 -->
<!-- The logo lives in celestia-island.github.io/res/logos/himari.webp; that asset does not
exist yet, and this repository does not keep a copy of org assets. Add the <img> line here
once the logo lands, then drop this comment. -->
<h1 align="center">Himari</h1>

<p align="center"><strong>公式資產：單一真相來源、三種表層語言，編譯成 Rust。</strong></p>

<div align="center">
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
[简体中文](../zh-Hans/README.md) ·
**繁體中文** ·
[日本語](../ja/README.md) ·
[한국어](../ko/README.md) ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>
<!-- markdownlint-enable MD033 MD041 -->

> **NOTICE：本軟體包含由人工智慧產生的程式碼。授權條款見 LICENSE 檔案（Synthetic Source License），其中包含模型揭露要求。**
> 本儲存庫中每一行原始碼都由 AI 模型在人類指導下產生；沒有一行是手寫的。參與的模型列於 [LICENSE.txt](LICENSE.txt) 末尾。
> 本 README 宣稱的每一項能力，都由本儲存庫中的測試作為後盾，並經過六輪獨立的對抗式審查——但**AI 審查不能取代人類審查**，
> 且撰寫當下，數學內容尚未經數學家審閱。
> 名稱取自 Blue Archive 的秋山晴奈（Himari Akayama，千年科學學園 · Veritas）——
> 把點子變成會運轉的機器。crates.io 名稱與組織儲存庫名稱都沒有與任何東西衝突（2026-11-23 查核）。

## 這是什麼

- 你撰寫**一份**公式（LaTeX 子集 / 論文記法 / MATLAB 風格，任你選）；它降階成同一個
  **帶量綱的 IR**，再編譯成帶具名中間值與原始碼註解的 Rust 函式。
- 量綱（七個基本量 kg·m·s·A·K·mol·cd）在編譯期強制執行：`m + s` 就是編譯不過。
- 內建符號微分、CSE（共同子表達式消除）與常數摺疊；解析偏導以測試對中央差分釘住。
- **矩陣是一等公民**：具體矩陣會被完整檢查；**樣式矩陣**（帶省略號）可表達動態尺寸——
  教科書上的 `[b c ⋯ 0; a b c ⋯; ⋯ a b c; 0 ⋯ a b]` 就照字面寫成原始碼。

## 聯邦與多行書寫（2026-11-23 第二號裁決）

**一個巨集區塊可以容納一整組互相引用的公式**（「聯邦」）：

```rust
formula_matlab!{
    decl { F : force [N]; m : mass [kg]; r : radius [m]; }
    accel = F / m
    mech = ( accel * r )^(1/2)
}
// expands to a let chain: let accel = F / m; let mech = (accel * r).sqrt(); mech
```

- **項目分隔符** = 分號或**換行**，而且這套規則**在三條車道上都成立**（token 車道從 proc-macro 的
  span 行號重建換行，而那些行號已對齊原始檔，所以診斷會指向同一行）。
- **後面的項目可以引用前面的項目**：量綱隨名稱一起傳遞（accel 的 m/s² 會加入符號表）；build.rs 車道在生成期把代換內聯進去。
- **名稱就是契約**：當公式重用某個 `decl` 名稱時，其結果量綱必須與該宣告相符
  （`a = F/m` 可以，`s : Pa; s = F/d` 會被拒絕）；同一個公式集或同一個生成目錄中的重複公式名稱一律拒絕。
- **運算子續行模型**：運算子必須位於**行尾**（`x +⏎ y` ✓）；下一行開頭的 `-`/`+` 是新項目的正負號，不是跨行的減法——這就是消歧義裁決。
- LaTeX 表層的隱式乘法（並置）**必須留在同一行**：跨行並置會把下一個公式吞成一個乘法項。

**矩陣多行書寫**（每個列一行實體行；欄間距自由，不需對齊）：

```text
[ 2*k, -k,    ...,   0 ;
  -k,  2*k,   -k,    ... ;
  ..., -k,    2*k,   -k ;
  0,   ...,   -k,    2*k ]
```

- 列分隔符 = `;` 或換行（可混用、可選結尾）；欄分隔符 = 逗號或空格（≥1，任意多個）。
- 儲存格中的省略號 `...` / `⋯` / `…` 表示「此位置省略」（寫回時推斷水平/垂直/對角）。
- **寫在 `-` 前面沒有空格時它是正負號，前面有空格則開始新的一欄**（MATLAB 語意）：`[k-1]` 是一欄，`[k -1]` 是兩欄。

## 複數：從值層一路管線接到程式碼生成（2026-11-23）

- **`j` 是保留的虛數單位**（永遠無量綱、永不宣告、**永不進入參數列**）。`Z = R_s + j*X` 可直接書寫。
- 值層恰好有兩個變體：`Value::Scalar(C)` / `Value::Matrix(Vec<Vec<C>>)`，其中 `C = {re, im}`。
  實數就是虛部為零的複數 ⇒ **複數矩陣、轉置、乘法、加法與減法全部
  免費地逐元素成立**；不需要「實矩陣／複矩陣」的象限切分。
- 超越函式在實數與複數共用一張表：`exp ln sqrt sin cos tan sinh cosh tanh` +
  `abs arg conj re im`（後四個永遠落回實軸）。複數次方走主分支上的 `exp(w·ln z)`。
- **`eval` 對複數值給出明確錯誤**（不會悄悄丟掉虛部）；想要複數值就用 `eval_value`。
- **程式碼生成有獨立的複數車道**：當公式含有 `j` 時，會生成為 `-> (f64, f64)` =（實部, 虛部），
  而區塊內的輔助函式（`__c_mul` / `__c_sqrt` …）**只會生成實際用到的那些**（否則 clippy `-D warnings` 會轉紅）。
  批次入口是 `out: &mut [(f64, f64)]`，可直接用於 EIS 頻率掃描。
  巨集車道（`formula!` 等）以同樣方式分流：`j` **不會**在呼叫端退化成自由變數；
  聯邦 `let` 鏈中已經是複數的綁定會被正確繼承（`emit_block_complex_with`）。
  複數車道**不生成解析導數**：`abs/arg/conj/re/im` 不是複解析的，所以鏈鎖律不成立。
- **生成的程式碼與值層必須逐點一致**是硬性約束：`__c_powf` 複製 `C::pow_real` 的整數分流
  （負實數底數的整數次方必須精確落在實軸上），而 `__c_sqrt` 與值層一樣使用 `hypot`，以避免大數量級時溢位；
  `__c_div` 遵循 IEEE 而值層回報錯誤，這是刻意的——這些在 fixtures 中各有一個逐運算子的對照測試把關。
- 單位字串接受有理數指數：`[V/A*s^(-1/2)]`（方括號是必要的——否則指數中的 `/` 無法與
  單位字串的除號區分）；無法整除的指數（`m^(1/3)`）會被明確拒絕。無量綱量請寫 `[1]`
  （增益 / 計數 / 莫耳分率都必須可以宣告；不能靠「省略單位」偷渡進來）。
- **`himari-fmt --diff <file>`**：唯讀預覽，印出 `--- current / +++ canonical`
  的 unified diff（無變更 → 無輸出且結束碼 0；有變更 → 結束碼 1）。
- **實數與複數之間的語意邊界**（刻意為之，不是疏漏）：`1/0` 以前給出 `inf`，現在**回報錯誤**；
  `ln(0)` 同理；`(-2)^0.5` 回報「結果是複數，請用 `eval_value`」。
  生成的程式碼改為遵循 IEEE 語意（除以零得到 inf/NaN）：熱路徑不攜帶錯誤路徑，
  而公式層能靜態證明為錯的東西，由量綱/形狀門禁攔下。
- **`python3 scripts/fmt_check.py --fix`**：一條指令重新排成標準形式並重新生成 sidecar。

## 單位：SI 字首、換算、無量綱群（2026-11-23）

- **拆字首前先查全名**：`m` 是公尺（不是「毫-什麼」），`mol` 是莫耳，`cd` 是燭光，
  `daN` 是十牛頓（最長字首優先）。`nF` / `GHz` / `mm^2` / `mV/mm` / `µF` / `Ω` 都能用。
  導出單位涵蓋電子領域躲不掉的六個：`F` `Ω`（或 `ohm`）`S` `H` `Wb` `T`。
- **量綱與換算因數分開儲存**：`parse_unit` 只給量綱；`parse_unit_spec` 另外給出 →SI 的換算因數；
  `convert(v, "nF", "F") = 1e-9 F`，量綱不符則報錯。質量的基本單位是 **kg**：
  `g = 1e-3 kg`、`mg = 1e-6 kg`、`Mg = 1000 kg`；字首從公克起算（由一個回歸測試釘住）。
  **生成函式的參數永遠是 SI 基本單位**，宣告中的字首只影響可讀性——生成程式碼的 doc comment
  會針對每個參數寫出換算因數（以下逐字摘自真實的生成 doc comment：
  `⚠ 参数 C_dl 声明为 µF：入参是 SI 基单位（1 µF = 0.000001 …）`）。
- **CGPM 2022 字首**已在表中：`Q` (1e30)、`R` (1e27)、`r` (1e-27)、`q` (1e-30)，與既有的
  `da … Y / d … y` 並列。
- **有精確定義的非 SI 單位**（`unit_base` 帶著各自的因數，好讓 `convert` 在量綱上保持誠實）：`min`、`h`、`d`、`L`
  （或 `l`；`mL` 拆解為 `L` 上的字首）、`deg`、`arcmin`、`arcsec`、`bar`、`atm`、`eV`、`Wh`、`tonne`、`Da`（或 `u`）、`nmi`、`knot`、`ha`、`acre`。
- **偏移／對數單位被明確拒絕**：`degC`（偏移）與 `dBm`（一個比值）在拆字首之前就被攔下，
  回報「真正的原因」而非「未知單位」——把它們悄悄當成 K 或無量綱會算出**錯誤的值**。
  **歧義符號同樣被拒絕，而不是猜**：`cal` / `Cal` 在熱化學卡（4.184 J）與 IT 卡（4.1868 J）之間不同，所以兩個都
  被拒絕並提示改寫 `J`；`rpm` 需要一個角度軸（1 rpm = 2π/60 rad/s），而本設施把 `rad` 視為無量綱，所以也被拒絕
  並提示改寫成 `2 * pi * f` 並加上 `[Hz]` 宣告。
- **無量綱群（Buckingham Π）**：`dimensionless_groups` 以**精確有理數**消去（`f64` 會漂移），
  並直接由「變數名稱 + 量綱」給出某組基底的無量綱組合；數量 =（變數個數）− 秩（Π 定理即斷言）。
  經典驗算：`{rho, v, L, mu}` → 1 群 `rho·v·L·mu^-1`（雷諾數）。
  `dim_solve` 是同一方程組的非齊次用法：`{kg, m/s²} → N` 給出指數 `[1, 1]`，
  而無法構成的量綱會回報「無法構成」以及還差幾個量綱。

## 量綱：半整數指數（2026-11-23）

`Dim` 內部以 **1/2** 為單位儲存（`[i16;7]` 存的是真實指數的兩倍），所以分數次方與整數次方
走同一套整數算術：`√s = s^(1/2)`、`f^(-1/2)`、`Ω·s^(-1/2)` 全部合法，而顯示使用括號形式
如 `s^(-7/2)`。因此 Warburg 阻抗回到了教科書形式 `Z = σ·f^(-1/2)`，
不需要參考頻率無因次化。`dim_check` 的次方指數**先摺疊常數**，
所以自然寫法 `f ^ (1/2)` 不必先呼叫 `simplify` 就能通過門禁。

## 函式家族：極值 / 夾取 / 矩陣運算子（2026-11-23）

函式參數個數（arity）的**唯一真相來源**是 `func_arity`（在 core 中）；四條車道（個數檢查 / 求值 / 量綱 / 生成 / 寫回）
都查詢它——以往每條車道各自帶一份 `match`，這正是「`arg` 在實數車道被生成為不存在的方法」能夠撐過一個版本的原因。

| 家族 | 函式 | 備註 |
| --- | --- | --- |
| 一元純量 | `exp ln log sqrt sin cos tan sinh cosh tanh` | 實數與複數共用同一張表 |
| 複數 | `abs arg conj re im` | `arg` 為無量綱；在實數車道，re/im/conj 是恆等 |
| 極值/夾取 | `min(a,b)` `max(a,b)` `clamp(v,lo,hi)` | 形狀同 Rust：`v.max(lo).min(hi)`；三個引數必須同量綱 |
| 矩陣運算子 | `det` `trace` `adjoint` `inv` | **只接受具體矩陣字面量**；`det`/`inv` 最高到 4/3 階（足以涵蓋 2D 與 3D 等參元素），`adjoint` = 共軛轉置 |

`det`/`inv` 使用**餘因子遞迴**（`det` ≤ 4、`inv` ≤ 3）：足以應付 2D 三角形與 3D 四面體的等參映射，
並以 `J·J⁻¹ = I` 與 `det(J)=1` 做數值檢查；4×4 的反矩陣需要 16 個 3×3 餘因子
（約 100 項），會讓生成的程式碼爆掉，所以被明確拒絕並提示改用數值函式庫。`det`/`trace` 在 `diff` 中**先展開再微分**——
展開式是一個普通的可微表達式，所以 FEM 的 `area = det(J)/2` 同時有值與解析導數
（`∂area/∂x₂ = (y₃-y₁)/2`）。
`min/max/clamp` 的分段選擇無法在 IR 中表達（沒有比較運算子），所以**不生成導數**；
生成程式碼中有一行註解說明原因（它不再 panic 了）。極值與夾取遵循 `f64::min/max` 的 NaN 規則
（可能回傳另一個運算元）；需要 NaN 傳播的呼叫端應在邊界檢查 `is_finite`。

## 內建形狀一覽

`identity` / `diag(d)` / `diag_v(d[i])` / `tridiag(a,b,c)` / `tridiag_v(b[i])` /
`bidiag_u(b,c)` / `bidiag_l(a,b)` / **`blockdiag2(p,q)` = diag(p·I₂, q·I₂)** /
**`banded(v, k)` = 半頻寬為 k 的對稱帶狀**（k=2 從 5×5 起才明確；在 4×4 會退化成稠密矩陣）。

這些名稱描述的是**被辨識出來的 IR 形狀**；它們不是你可以在公式中呼叫的函式。入口是
帶省略號的矩陣草圖，任何無法辨識的東西都會被明確拒絕。

區塊內的區塊對角語意 = 純量 × 單位矩陣（對角線上為 p，區塊內的離對角必須為空），區塊外必須為空
（`0` 與 `...` 等價）；若總維度為奇數，中央儲存格歸右下角的區塊。

## 複數車道效能：誠實的那一面（2026-11-23 實測）

`bin/bench.rs` 現在量測四條車道。複數那條，Z_c（EIS 頻率掃描），**比手寫還慢**；照實記錄：

| 形式 | ns/點 | 相對手寫 |
| --- | --- | --- |
| 手寫特化（`σ/√ω·e^{-jπ/4}`，無分支，可向量化） | 2.3 | 1× |
| 同形式轉寫（把生成程式碼的運算子序列抄進一個迴圈） | 2.9 | 1.3× |
| 生成程式碼 · 純量直接呼叫 | 6.4 | 2.8× |
| 生成程式碼 · 批次入口 | 12.8 | 5.5× |

拆解：**泛用複數 √/÷ 比手寫特化多花約 2.2×**（1 次 sqrt 對 3 次 sqrt + 一次除法），
而**批次包裝再多花約 2.5×**（tuple 切片寫入 + 分支讓迴圈無法向量化）。

已完成的優化（每一項都真的縮小了差距）：

1. `__c_sqrt` 從極座標形式（atan2+cos+sin，4 個超越函式）改為**代數閉式**；
1. 純虛快路徑 `√(iy) = √(|y|/2)(1+sign·j)`（只要 1 次 sqrt——EIS 中最常見的形狀）；
1. `__c_div` 在一般數量級使用樸素形式（1 次除法）；
1. 複數輔助函式從**閉包**改為區塊內 `#[inline(always)] fn`（15.5 → 11.9 ns/點）。

**結論（寫進 README 而不是藏起來）**：複數車道適合「算得對 + 夠快」，不值得
去追手寫特化的極限；對於 EIS 掃描真正熱的迴圈，手寫特化，並把公式留在
`.formula` 檔案中當作**真相來源與量綱檢查**。實數車道沒有這個問題（純量 1.11×，與手寫相當）。

## 批次入口：定長勝出，切片落敗（實測）

生成器為每個公式生成**兩個**批次入口：

| 形式 | 簽名 | 實測（BV，100k 點） |
| --- | --- | --- |
| 切片版本 | `bv__batch(p0: &[f64], …, out: &mut [f64])` | 24.1 ns/點 = **純量的 0.82×（慢 22%）** |
| 定長版本 | `bv__batch_n<const N: usize>(p: &[f64; N], …, out: &mut [f64; N])` | 19.3 ns/點 = **純量的 1.03×（快 3%）** |

結論：**想要效能，就把長度放進型別參數**。切片版本每點八次邊界檢查都在迴圈內，
LLVM 無法完全消去；先算一次共同長度（`.min()` 鏈）或改用 zip 迭代器也只能到 0.83×。
切片版本保留為方便形式（變長資料）；熱路徑請用定長版本。
（bench 來源：`fixtures/three/src/bin/bench.rs`；我們踩到的兩個方法學陷阱——在計時區內建立向量，
以及常數向量讓純量路徑被常數摺疊——都記錄在原始碼註解中。）

## 矩陣代數（形狀 + 求值 + 生成）

量綱與形狀是**兩條正交的檢查軸**：量綱管物理意義，形狀管代數合法性。

| 能力 | 行為 |
| --- | --- |
| `shape_of` | 具體矩陣 → `(rows, cols)`；轉置會交換；符號/樣式矩陣 → `None`（不做動態推斷） |
| `shape_check` | 矩陣加/減要求形狀相同，矩陣乘積要求 `left cols = right rows`，鋸齒矩陣拒絕，矩陣除法明確拒絕 |
| `eval_value` | 純量/矩陣的統一求值，矩陣乘積為 `C_ij = Σ_k A_ik·B_kj` |
| `flatten_matrix_value` | 把**可靜態展開**的矩陣表達式摺疊成具體矩陣 IR（轉置/加/減/取負/純量廣播/矩陣乘積） |
| 生成 | 摺疊後的矩陣 → `vec![vec![…], …]` 常數字面量；大矩陣走樣式矩陣 + 迴圈車道 |

```text
[1, 2; 3, 4] * [5, 6; 7, 8]  →  [[19.0, 22.0], [43.0, 50.0]]
[1, 2, 3]'                   →  3×1 column vector (emitted as vec![vec![1.0], vec![2.0], vec![3.0]])
A * [1, 2]                   →  scalar broadcast (a bare symbol is a scalar: matrices only come from literals/pattern generators)
[1, 2, 3] * [1, 2, 3]        →  compile-time rejection: matrix multiply impossible: (1, 3) × (1, 3)
```

> **泛用的執行期 `A,b` 仍然必須這樣宣告**：`decl` 的名稱所帶的量綱（quantity）以 `_matrix`/`_vector` 結尾
> （或等於 `matrix`/`vector`）會成為矩陣/向量參數，下面的執行期車道用的就是這個。所有*其他*裸符號都是純量，
> 所以 `A * x` 這種對形狀不在宣告中的矩陣的運算，仍然要用字面量、可靜態展開的表達式或樣式生成器來表達。
> 一般的大規模 `Ax=b`、執行期尺寸推斷或病態方程組，請用數值線性代數函式庫。

**非負整數矩陣次方**：`[a, b; c, d] ^ 2` 由生成器摺疊成顯式的矩陣乘法（0–8 次），
`M ^ 0` 是同階單位矩陣（回傳型別仍是矩陣，不是純量 `1`）。端到端數值檢查在
`matrix_power.formula`。摺疊失敗（負/分數次方、超出範圍、非方陣）會生成 `compile_error!`
而不是像 `vec![…] * vec![…]` 那樣的編譯意外；值層（`himari-eval`）對同一形式給出相同結果。

**兩個獨立實作必須一致**：矩陣表達式的程式碼生成（`flatten_matrix_value` + 字面量生成）
與值層求值（`eval_value`）是分別撰寫的，所以用一個**差分測試**讓兩邊對齊——
求值摺疊後的字面量必須逐格等於求值未摺疊的表達式（`mat-expr-fold`）。

**沒有省略號的矩陣草圖就是具體矩陣**：它走字面量車道（常數摺疊，沒有配置迴圈），
這比生成器快；要用生成器車道（`n` 是執行期參數）就必須寫出省略號。

## 執行期矩陣/向量車道（2026-11-23）

上面一切都是靜態的：形狀在公式編譯時就知道。執行期車道是針對
**尺寸只在執行期才存在的參數**的另一個答案——消費使用者資料的求解器、FEM 組裝、任何把 `A` 與 `b` 當作值接收的東西。

- **宣告是唯一的開關**：`decl` 的量綱以 `_matrix`/`_vector` 結尾（或等於 `matrix`/`vector`）時，它會成為
  矩陣/向量參數——`A : stiffness_matrix [N/m]` 以 `&[Vec<f64>]` 傳入，`b : load_vector [N]` 以 `&[f64]` 傳入。
  從不選擇加入的名稱保持純量，所以既有公式不受影響（`Decl::runtime_kind`）。
- **生成的函式是可失敗的**：它們回傳 `Result<_, himari::runtime::NumericError>`，單一結構化錯誤型別
  （`InvalidInput` / `ShapeError` / `NonFiniteValue` / `SingularMatrix` / `IllConditioned` / `UnstableResidual` / `NoRealRoots`）。這條車道上
  **沒有靜默 NaN 契約**。
- **`solve(A,b)`** 是帶部分選主元的 LU，然後在回傳任何答案前有兩道獨立的門禁：
  一個 **rcond 估計**（`1/(‖A‖∞·‖A⁻¹‖∞)`，要求 `rcond ≥ f64::EPSILON·64·n`）與一個**相對殘差檢查**
  （`≤ 1e-9`）。反矩陣只為條件數的估計而構成，從不是取得 `x` 的途徑。奇異主元、病態矩陣與殘差過大的解
  是三種不同的錯誤，不是一個籠統的「失敗」。
- **這條車道也支援**：`inv`、`det`（只接受字面量引數——執行期 `det` 被拒絕，因為其量綱需要靜態的
  矩陣階數）、`A*x`、`dot`、`norm`，以及用 **1 為基底**索引的 `A(i, j)`（`[I,J]`；越界與零是錯誤，不是 panic）。
- **它生成什麼**：主體是生成函式內部對作用域內 `let` 鏈的受檢表達式，回傳型別跟隨
  值的種類（`Result<f64, _>`、`Result<Vec<f64>, _>`、`Result<Vec<Vec<f64>>, _>`）。執行期車道的公式**不**生成解析 Jacobian、**不**生成
  `__f32` 變體、**不**生成批次入口（那些需要純量的、有尺寸的簽名）。
- **答案對照獨立參考檢查**：`runtime_matrix.formula` 以另一套帶選主元的
  高斯消去法驗證 `solve(A,b)`，以 `A*x` 對照 `b`，以 `A·inv(A)` 對照單位矩陣的各欄，檢驗一為基底的索引，以及一個刻意設計的極小 epsilon 方程組
  （`[[1e-17, 1], [1, 1]]`）——只有選主元才解得對——全部容差為 2e-12/2e-15。
- **巨集車道**：`formula_matlab!` 也接受執行期公式，但**每個區塊只能有一個公式**（執行期鏈留在 build.rs 車道
  並附明確訊息）；複數執行期矩陣參數目前拒絕。

## 程式碼生成品質（2026-11-23）

- **整數次方生成 `powi`**：`x^3` → `(x).powi(3)`（以往它生成一個不存在的內部方法，
  所以任何 `x^3` 都編譯不過，錯誤還指向生成的程式碼）。fixture 為 `x^3` 設了**編譯門禁**。
- **未知函式在編譯期失敗**：實數車道的函式是白名單；`arg(x)` 在實數上沒有意義 →
  `compile_error!`；`re/im/conj` 在實數上是恆等（精確，不是近似）。
  多引數呼叫在解析期就被拒絕（生成器以前只看 `args[0]`，悄悄丟掉其餘引數會算出錯誤的值）。
- **系統 Jacobian**：每個 `.formula` 檔額外生成 `{filename}__jacobian(all params)`，
  列 = **參與微分的實數純量公式**（矩陣值 / 複數 / 不可微的項目會跳過），欄 = 參數（宣告順序），
  而不依賴某參數的列在該欄全程為 **0**。這正是隱式求解器 / 牛頓迭代 / FEM 組裝需要的形狀。
- **非有限常數硬失敗**：生成 `inf` / `NaN` 會產生非法識別字，
  所以在生成前就 panic，並指出來源（`contains_nan_expr` 是唯一真相來源）。
- **次方外層的括號**：`(x^2)^3` 與 `x^(2^3)` 在兩個表層都會加上括號——
  `x ^ 2 ^ 3` 會被重新解析成 `(x^2)^3`（x⁸ → x⁶，也就是標準格式化會**改變數學**）。

## 格式化（標準形式）

`himari-fmt`：`cargo run -p himari-check --bin himari-fmt -- [--check|--write] <file>`。

- 多行 decl（每行一個符號）；每行一個公式；**矩陣欄位對齊**（每欄 = 最寬內容 + ≥1 空格，逗號為欄邊界）。
- 獨立的註解行全部浮到檔案開頭（順序保留）——註解是給人看的，永不進入 IR。
- `--check` = 漂移門禁（已接入 `just fmt-check` / CI）；標準輸出往返冪等。
- **標準形式保留逗號**：純空隔分離與二元減法在寫回時無法區分（`-k` 會被吞成一個減法項），
  所以逗號是明確的邊界；手寫時不寫逗號沒關係，格式化會把它們補回去。

## 車道（2026-11-23 使用者裁決）

| 車道 | 用法 | 適用於 |
| --- | --- | --- |
| **內嵌巨集** | `formula_matlab!{ decl { a : l [m]; b : l [m]; } a / b }` | 簡單公式 |
| **字串巨集** | `formula_latex!{ r"decl { … } \frac{a}{b}" }` | 簡單公式（LaTeX 表層） |
| **外部檔案巨集** | `formula_file!("formulas/bv.formula", matlab)` | 複雜公式（方言必須明確給出） |
| **build.rs** | 見 `packages/check/fixtures/three/` | 入庫的生成程式碼 + 漂移門禁（正式車道） |

巨集內的 `decl` 只提供量綱檢查所需的符號資訊；它**不會建立 Rust 區域變數**：呼叫端的 `a/b`
必須已由呼叫者寫成 `let a = …; let b = …;`。完整的可編譯範例見
`inline_macro_example()`（位於 `examples/common/src/main.rs`）。

> ⚠️ 已知限制：讀取檔案的 proc-macro **不被 cargo 追蹤**（`tracked_path` 仍是 nightly），
> 所以編輯 `.formula` 後需要 `touch` 才會觸發重建；正式使用請走 build.rs 車道（`rerun-if-changed`）。
> ⚠️ 樣式矩陣/轉置的**建構子車道**只存在於檔案 / build.rs（它需要檔名與生成器）；
> 內嵌巨集遇到它們時會回報明確錯誤並附提示，永不靜默降級。

## 表層語言（三種，一個 IR）

| | 論文 | LaTeX | MATLAB 風格 |
| --- | --- | --- | --- |
| 希臘字母 | `α·x` | `\alpha \cdot x` | `alpha*x` |
| 省略號 | `⋯ …` | `\cdots \vdots \ddots` | `...` |
| 矩陣 | `[a, ⋯, b; ⋮ ⋱ ⋮; c, ⋯, d]` | `\begin{bmatrix}…\end{bmatrix}` | `[a, ..., b; ...]` |
| 轉置 | `A'`、`A′`、`A^T` | `A^T` | 同左 |
| 量綱宣告 | `decl { v : velocity [m/s]; }` | 同左 | 同左 |

規則（拒絕即特性）：

- 每個符號都必須先連同量綱宣告；未宣告 = 編譯錯誤，不做字形推斷。
- `\sin^2 x`、`\left(\right)`、`$…$`、`x .* y` 這類歧義或純排印記法一律直接拒絕。
- 隱式乘法只在 LaTeX/論文表層成立（單字母系統在構造上就無歧義）；MATLAB 表層要求明確的 `*`。
- 樣式矩陣只支援三種可證明正確的形狀：`identity(n)`、`diag(d, n)`、
  `tridiag(a, b, c, n)`；任何無法辨識的東西 = 錯誤 + 這三種的清單。
- 原始檔可以手寫（組合符號等），但 **IR 必須正規化**——組合字元永不進入 IR。

## 向量代數 / Horner / no_std（2026-11-23）

**`dot(a,b)` / `norm(v)`**：只在**具體字面量**上展開（形狀在編譯期已知）；
`dot` 的量綱是兩邊之積，`norm` 與其元素同量綱。`dot([x,y],[x,y])`（x=3,y=4）→ 25，
`norm` → 5。形狀不符一律直接拒絕。

**自動 Horner 轉換**：原始碼中寫成 `c₀ + c₁x + c₂x² + …` 的**顯式多項式**在生成前會被轉成
`(…((cₙx + cₙ₋₁)x + …) + c₀)`——乘法次數從 `2n-1` 降到 `n`，而像 `x^n` 的大次方會消失
（數值上更穩定）。`a*x^3 + b*x` → `(a * x * x + b) * x`。
**緊湊形式不展開**：`(u-1)^3` 保持原樣（展開只會更難讀）；導數也是對
**Horner 之前的原式**取（對 Horner 形式微分會得到一串互相抵消的乘積）。

**`no_std` 車道**：`himari_buildhelper::generate_dir_libm` 產生的程式碼只用 `core` + `libm`
（超越函式 → `libm::exp` 之類，整數次方 → `libm::pow`；`f64::powi` 是 std 的東西）。
這指的是**生成的程式碼**可作為 `no_std` 使用——而自 2026-11-23 起，編譯期堆疊也是：
`himari`、`himari-latex` 與 `himari-matlab` 是 `no_std + alloc`，放在預設 `std` feature 之後（除非你關掉，`std` 是開著的）。
loader、buildhelper、macros 與 check crates（檔案 IO、proc-macro 宿主、工具）仍為 `std`，並明確開啟該 feature。
生成的程式碼保留**自己的 libm 車道**，那才是裸機消費者實際連結的東西。
門禁：一個 `#![no_std]` fixture crate（`[lib] test = false`，因為測試框架需要 std），它既編譯
生成的程式碼，**又**在沒有 std 的情況下跑過解析器鏈（`himari_matlab::parse` → `dims_of_set` → `shapes_of_set` → `dim_check_runtime` → `format_set`）。
**`cargo build` 通過 = 生成的程式碼與前端沒有偷偷伸手去拿 std。**
已驗證的裸機目標：`riscv32imc-unknown-none-elf`（只有 core——沒有可用的 atomics）、`thumbv7em-none-eabi`，以及
`x86_64-unknown-none`（解析器鏈）。這條車道不生成 Jacobian（需要 `alloc`），也不生成 f32 變體
（f32 超越函式是 libm 的 `*f` 家族，而我們拒絕只交付半條車道）；要偏導數，請用每個公式的 `__d_*` 函式
（純純量，零配置）。

## 跨檔公式庫：`use "..."`（2026-11-23）

```text
use "lib/atmosphere.formula"      % file header, may span several lines
decl { ... }
v_terminal = sqrt(2 * m * g / (rho * Cd * A))
```

- **解析只負責收集，IO 屬於 loader**：前端保持純字串 API（`FormulaSet.uses`），
  而讀檔、遞迴內聯、循環偵測與衝突解決由 `himari-loader` 處理。
- 合併語意（簡單且可預測）：被引用的 decls/items **排在最前面** → 聯邦照常運作（後面的項目可引用前面的項目）；
  完全相同的重複 decl 會去重，拼寫不同則報錯；重複的項目名稱一律報錯（**沒有「後寫覆蓋」**）；
  循環會報錯，並印出循環周圍的檔名鏈。
- **庫是「被代換」而不是「被呼叫」**：一旦 `rho` 展開到呼叫端，參數列就包含 `rho_0 / h / H`。
  這是刻意的（沒有隱藏狀態，可靜態微分）；代價是參數列變長。
- 目錄慣例：`generate_dir` 只掃描**最上層**的 `*.formula`，所以 `formulas/lib/*.formula`
  自然只以庫的形式存在，永遠不會被當成最上層公式再生成一次。
- **方言來自巨集，不靠猜**（2026-11-24 使用者裁決，「巨集就是方言」）：包住公式的是哪個巨集，
  就決定哪個文法去讀它。

  | 巨集 | 方言 |
  | --- | --- |
  | `formula!(…)` | **matex**（標準表層；入口名稱保留，語意已定） |
  | `formula_latex!(…)` | LaTeX |
  | `formula_matlab!(…)` | matex（與 `formula!` 同義，讀起來更自明） |
  | `formula_file!("f.formula", latex)` | 由**必填**的第二個引數決定 |

  `formula_file!` 不把方言引數設為選填：檔案內容無法從巨集名稱推斷，而讓巨集去猜
  只是把歧義藏到別的地方。

  **為什麼這不只是風格偏好**——它消除了一個真實的靜默錯誤值。當 `si` 已宣告時，`si * x`：
  matex 讀成 `si·x`，LaTeX 讀成 `s·i·x`。對同一個檔案、同一組輸入實測（s=3, i=5, x=2, si=100）：

  ```text
  old AUTO (try matex first, use if it succeeds)  →  200    without reporting any error
  explicit LaTeX                                  →  30
  ```

  兩者相差將近七倍。AUTO 無從知道作者指的是哪一個——它「猜」到的只是哪個解析器先成功。
  這就是為什麼巨集車道的 `Dialect` 列舉**沒有** `Auto` 變體：型別系統保證猜測無法回來。

## 快速開始：`examples/reentry`（最小可執行專案）

```text
examples/reentry/
  formulas/entry.formula       % terminal velocity / dynamic pressure / drag (aerospace side)
  formulas/structures.formula  % static displacement = solve(k·x − F, x)
  build.rs                     % three lines: generate_dir("formulas", "src/generated.rs")
  src/main.rs                  % prints the table + physical invariant self-checks
  src/generated.rs             % the checked-in generated code (reviewable, covered by the drift gate)
```

`cargo run -p himari-example-reentry` 會印出一張速度→動壓→阻力的表，並驗證：

- **物理不變量**：`drag(v_terminal) == m·g`（相對差 1.6e-16——這不是擬合，這是定義）
- **解析導數對差分商**：`∂drag/∂v` 在兩條路徑間以 1.2e-8 的相對差吻合
- **符號求解**：`x_static` 來自 `solve(k·x − F, x)`，且 `∂x/∂F = 1/k` 自動成立
- **f32 變體**與**庫內聯公式的系統 Jacobian**都在同一個生成檔中

消費者現在只需要 `himari-buildhelper::generate_dir(dir, out)`——整條 build.rs 車道
（解析 / 聯邦 / 量綱 / 形狀 / 導數 / f32 / 批次入口 / Jacobian / 原始碼 LaTeX）收斂成一個函式，
每個專案不必再抄 300 行。

## 常見範例：`examples/common`（真實生成 + 獨立數值參考）

```bash
cargo run -p himari-example-common
cargo test -p himari-example-common
```

| 公式檔 | 範例 | 生成的入口與驗證 |
| --- | --- | --- |
| `matrices.formula` | 旋轉、轉置、`AᵀA`、2×2 反矩陣、`A·A⁻¹`、矩陣乘行向量 | `rotation_orthogonality` / `N` / `stiffness_inverse` / `nodal_forces`；逐格對照手算 |
| `springs.formula` | 三節點彈簧平衡方程組 `solve([…],[u₁,u₂,u₃])` | `displacements` 對照另一套帶部分選主元的 Rust 高斯消去法 |
| `runtime_matrix.formula` | 執行期 `A`/`b`：`solve(A,b)`、`A*x`、`inv(A)`、一為基底的索引 | `runtime_displacements` / `runtime_traction` / `runtime_inverse` / `runtime_element`；每次呼叫皆可失敗（`Result`），選主元在 `[[1e-17,1],[1,1]]` 上得到證明 |
| `least_squares.formula` | 直線擬合的正規方程 | `line_fit` 對照直接的最小平方閉式，給出 `y=1.04+1.99x` |
| `roots.formula` | 二次式，以及 `b²≫4ac` 時的小根 | `quadratic_roots` 檢查 `1,2` 與 `10⁻⁸,10⁸`（防止相消）；回傳 `Result`，負判別式為 `NumericError::NoRealRoots` |
| `circuit.formula` | `I=V/(R+j·2πfL)` | `current` / `current_magnitude` 檢查 `|I|=0.995037 A`；`2π` 明確寫出 |
| `dynamic_matrix.formula` | 動態 `n×n` 三對角模板 | `diffusion_stencil(k,n)` 逐格對照以迴圈建構的參考 |
| 內嵌巨集（`src/main.rs`） | 帶 `a/b` 的 `formula_matlab!`，以及執行期形狀的 `solve(a, b)` | 真正傳入 Rust 區域變數；`m/m=1` 在編譯期檢查；執行期求解做數值驗證 |

這些範例證明**目前支援的形式**真的能從 `.formula` 走到可執行的 Rust；它們不把
`solve` 冒充成通用的數值求解器，而靜態範例也不需要執行期矩陣參數。
生成的 `src/generated.rs` 已入庫，並受漂移門禁保護。

## 標準車道：`himari-standards`（2026-11-23）

`packages/standards` 把少數幾份**已發布的工程標準**變成具型別的計算器，一個模組對應一條條文層級的關係，每個都
帶自己的 `PROVENANCE`（標準 id、版次、公式出自的條文，以及*未*建模的部分）。這裡沒有一處引用
標準原文，也沒有一處宣稱符合該標準——它只是把公式重述為普通表達式。單位換算
刻意缺席：呼叫端傳入 SI 基本單位。

| 模組 | 標準 | 版次 | 計算內容 |
| --- | --- | --- | --- |
| `iso_6506_1_2014` | ISO 6506-1 | 2014 | 布氏硬度 `HBW`（外加壓痕平均壓力，單位 Pa） |
| `iso_6892_1_2019` | ISO 6892-1 | 2019 | 抗拉強度 `Rm`、伸長率 `A`、彈性模數 `E` |
| `iso_2533_1975` | ISO 2533 | 1975 | 對流層 `T`、`p`、`rho` 與位勢高度換算（0 … 11 000 m，超出即拒絕，不做外推） |
| `gb_t_231_1_2018` | GB/T 231.1 | 2018 | 布氏硬度 `HBW`；現行版，取代 GB/T 231.1-2009，修改並採用 ISO 6506-1:2014 |
| `gb_t_228_1_2021` | GB/T 228.1 | 2021 | 抗拉強度 `Rm`、伸長率 `A`、彈性模數 `E`；現行版（2022-07-01 生效） |
| `gb_t_8170_2008` | GB/T 8170 | 2008 | 十進位捨入，**四捨六入五成雙**，在十進位字串上決定——`f64` 永不參與捨入決定 |

兩個 GB 力學試驗模組重用 ISO 的算術（同一套受測的求值）但保留**自己的** id、版次與出處，所以
回應永遠不會被標上錯誤的標準。對於 `GB/T 228.1-2021`，它對任何 ISO 文件的採用程度**並未**獨立
驗證，所以該模組明確不宣稱等效——只實作了這三個定義。版次已對照
國家標準資訊公共服務平台查核，每個模組都在其出處備註中記錄了這次查核。

CLI `himari-standard` 求值一個請求並印出一份形狀穩定的回應文件（JSON），把請求讀成 JSON 或 TOML——
從 `--input <path>` 或 stdin，`--format json|toml` 強制指定解析器，`--list` 列出確切的 id 與版次：

```bash
himari-standard --input packages/standards/examples/brinell.json
himari-standard --input packages/standards/examples/tensile.toml
himari-standard --list
```

`packages/standards/examples/` 下的這三個範例請求由測試套件實際執行，
所以文件中的呼叫方式不會腐壞。

未知的鍵會被拒絕而非忽略；每一次失敗都回傳同一份文件，`values` 設為 `null`，原因放在 `errors`
（結束碼 1，stderr 上一行診斷）；結束碼 2 保留給命令列誤用。小數會參與**捨入決定**的請求，
必須把那些數字以十進位字串傳入，因為二進位浮點數在 crate 看到它之前就已經丟掉末位了。

## 證明車道：`himari-lean`（2026-11-23）

`packages/lean` 是一個 Lean 4 **協同程式**：它生成一個自足的證明義務，經 stdin 交給真正的 `lean` 執行檔
（`lean --stdin --json -T <heartbeats> -M <MB>`），並以一份結構化文件回報發生了什麼。沒有 Lake 專案、沒有套件索引、
沒有網路；生成的檔案沒有任何 import。

- **動態資料只會變成字面整數**，出現在*具名的範例定理陳述*中——永不變成資料定義、公理或識別字。
- **由 `decide` 經核心檢查**：義務在 `Int` 上封閉，並由 `by decide` 解除。不生成 `native_decide`、`#eval`、
  `implemented_by`、`extern`、`partial`、`sorry` 或使用者公理，而有一個測試針對這份清單斷言生成的文字。
- **公理門禁**：解析 `#print axioms` 的輸出，只容忍 `propext`、`Classical.choice` 與 `Quot.sound`；無法讀取的報告即為
  違規，永不視為通過。沒有報告不是乾淨的證據。
- **獨立神諭**：同一個行列式在 Rust 中以*不同*的展開式計算，所以 Lean 永遠不是唯一的證人。
- **`lean_missing` 是一等公民的非成功**：它表示工具鏈不在，**什麼都沒檢查**。它永不呈現為
  「已驗證」，也永不以 0 結束。狀態與結束碼：`success` 0、`lean_missing` 3、`invalid_request` 4、`proof_failed` 5、
  `resource_exhausted` 6、`axiom_violation` 7、`oracle_mismatch` 8。

**已對真正的工具鏈驗證（Lean 4.36.0，commit `611c117b`）。** 里程碑義務被核心接受，且完全未回報任何
公理：

```text
status = success            verified = true       exit = 0
lean_message: 'det3_example' does not depend on any axioms
oracle:       rust-i128-permutation det3([1,2,3,4,5,6,7,8,10]) = -3
```

負對照在同一個執行檔下如設計般運作：一個斷言 `expected = -306` 的請求回傳 `oracle_mismatch`（結束碼 8），
永不回報為已驗證，而 stub-process 測試套件涵蓋 `proof_failed` / `axiom_violation` / `resource_exhausted` 路徑。

佈建在本儲存庫之外，且是一次性的：安裝 `elan`、釘住 `leanprover/lean4:v4.36.0`，然後把 `lean` 放到 `PATH` 上，
或用 `HIMARI_LEAN_BIN` 指向它。`just lean-verify` 生成義務並執行真實 Lean 測試。**沒有工具鏈時，這條車道降級為
`lean_missing`（非成功），而 `--emit` 仍會寫出可審閱的 `.lean` 檔**——Lean 的缺席永不回報為驗證。

## 健壯性與對抗式輸入（2026-11-23）

- **深度上限 2048**：每個解析出口檢查一次 IR 深度（`check_depth`），超過上限的東西會回報
  「formula nested too deeply (N levels > limit 2048)」，而不是讓某個下游遞迴爆疊（SIGSEGV）。
  pest 自帶的堆疊上限也會觸發，其訊息同樣被翻成平實字句。**已知邊界**：長鏈 `a + b + c + …` 在
  IR 中是左結合嵌套，所以深度等於項數；單一公式的項數因此上限約 2048。
  另有一道獨立的確定性括號嵌套守衛（256 層，`MAX_PARSE_NESTING`）在解析前執行，因為它必須在 pest 的
  選用 `stack-guard` feature 不可用時（`no_std`）也成立。
- **不可見字元一律拒絕**：零寬空格 / 雙向覆寫 / 零寬連接符 / BOM / NUL 全部
  在解析前具名拒絕（`scan_source`）——它們能讓公式「在審閱時看起來與實際不同」。
- **Σ/∫ 步數上限 10^7**：`Σ(i,1,1e9)` 從「跑一整夜」變成一行錯誤訊息。
- **缺引數的函式呼叫**不再透過 `args[0]` 索引 panic：三條車道（求值/生成/量綱）都明確拒絕。
- **結構化屬性測試**（不是隨機字元亂燉）：600 個隨機表達式各過三道門禁——標準寫回重新解析為
  **相同的樹**、LaTeX 寫回重新解析為**相同的樹**、且 `simplify` **保持值不變**。
  三者目前都顯示 0 個不等價（`prop-roundtrip`）；一個加總 1500 項的大輸入在 100 ms 內完成。

## 單精度（f32）車道（2026-11-23）

每個**實數純量**公式額外生成 `{name}__f32(p: f32) -> f32`——同一個公式、
同一套生成規則，只把字面量換成 `f32` 後綴（`Fx::F32`，貫穿生成器的唯一開關）。
實測 f32/f64 = **1.44×**（`SIMD` 8 條 vs 4 條 + 一半的頻寬），
遊戲引擎 / 音訊 DSP / 即時 SDR 可直接受益。

複數與矩陣值公式**不**生成 f32 變體：複數車道的 `(f64, f64)` tuple 與矩陣的
`Vec<Vec<f64>>` 在單精度下是不同的型別契約；那可以日後再做（我們拒絕只交付半條車道）。

## 開發者工具（2026-11-23）

三個入口一起交付（都在 `himari-check` 套件中，共用同一個解析器）：

| 入口 | 用途 |
| --- | --- |
| `himari-fmt --diff` | 只顯示標準形式與現行形式的差異（不重寫） |
| `himari-fmt --diagnose` | 只做解析 + 量綱 + 形狀診斷（乾淨時無輸出，結束碼 0） |
| `himari-fmt --symbols` | 列出 `decl` 名稱與公式名稱（編輯器補完的資料來源） |
| `himari-eval <file> --set k=v …` | **求值**：`Re = 200000 [1]`；`--groups` 另印 Buckingham Π 群；`--json` 給機器讀 |
| `python3 scripts/fmt_check.py --fix` | 一條指令重排標準形式 + 重新生成 sidecar |

**錯誤帶有脫字符**：`FrontError::render_with_source` 印出原始碼行，並在出錯欄位下方畫一個 `^`
（三條車道——build.rs / fmt / eval——都經過它）——不用再自己數到第 17 欄。

**VS Code 擴充**（`packages/vscode`）有兩種通道：

| 通道 | 觸發時機 | 資料來源 |
| --- | --- | --- |
| CLI（預設，零安裝） | 開檔 / 存檔 | `himari-fmt --diagnose` + `--symbols` |
| **LSP**（選用） | **打字即時錯誤** | `himari-lsp`（stdio JSON-RPC） |

`himari-lsp` 是手寫的最小伺服器（不依賴任何 LSP 框架）：`initialize` /
`didOpen` / `didChange` / `completion` / `codeAction` / `shutdown`，支援**增量文字同步**。
其診斷攜帶與庫相同的 `himari::Error`，以 `HIMARI_LANG`（回退 `LC_ALL`、`LANG`，再回退英文）選擇的語言渲染；九種專案語言全部可用。
協議層把狀態與 JSON-RPC 參數當作純函式處理，並有一個端到端測試，真的啟動一個行程並真的送出
`Content-Length` 訊框。在設定中設 `himari.lspPath` 即可切換（留空 = CLI 通道）。

補完候選 = 檔案中的 `decl` 名稱 + 公式名稱 + 一元函式表（含 `det/inv/solve/dot/norm` 等）。

## LaTeX 表層：多字母名稱需要 `\mathrm`（2026-11-23）

在 LaTeX 數學模式中，裸寫 `sigma` 是**五個相乘的變數 s·i·g·m·a**（隱式乘法），
所以這個方言中的多字母名稱必須寫成 `\mathrm{sigma_0}`——而寫回時也以同樣方式輸出。
希臘字母名稱（`sigma`、`omega`……）寫回為**真正的字形** `\sigma`，並保留下標
（`\sigma_0`）。實作以往是子字串替換，所以 `alphabet` 變成 `\alphabet`、
`betamax` 變成 `b\etamax`，而 `sigma` 被渲染成五個斜體字母——三個地方同時錯（懸停 / sidecar / 文件真相來源）。已修正（`latex_var`）。

## 錯誤：一個 enum，九種語言

crate 裡每個失敗都是同一個 enum `himari::Error`。它不是一堆字串：每個失敗是一個**自帶資料**的變體，
因此同時提供三件事。

- **穩定碼** —— `err.code()` 回傳 kebab-case 識別碼（`dimension-mismatch`、`undeclared-symbol`、
  `no-real-roots`……）。測試與呼叫方斷言碼，所以改措辭或換語言都不會敲壞它們。
- **依語言渲染** —— `err.render(locale)` 與 `Display`（英文）從 [i18n.rs](packages/himari/src/i18n.rs)
  的目錄產生句子。CLI 與 LSP 邊緣用 `HIMARI_LANG` / `LC_ALL` / `LANG` 選擇；九語即組織的
  （`en`、`zh-Hans`、`zh-Hant`、`ja`、`ko`、`fr`、`es`、`ru`、`ar`）。
- **與 `anyhow` 相容** —— `Error` 實作 `core::error::Error`（`std` 下即 `std::error::Error`），
  且是 `Send + Sync + 'static`，不需要任何包裝：

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

`tests/anyhow_compat.rs` 把這幾件事都釘住：`?` 傳播、`Send + Sync + 'static` 邊界、
`anyhow::Context` 之後仍能 `downcast_ref::<himari::Error>()` 拿回穩定碼，以及「同一錯誤在九語下
各不相同、而碼始終不變」。

尚未編目的訊息保留在 `Error::Message(String)`：它在任何語言下都原樣輸出，所以分期遷移在**型別上**
可見，而不是假裝已翻譯。這筆債**由門禁計量**而非散文：`python3 scripts/staging_budget.py` 統計工作區所有生產樹（`packages/`、`examples/`、`tools/`）裡全部未編目的生產者（`Error::from(`、`Error::Message(`、前端的 `FrontError::plain("…")` / `plain(format!(…))` / `FrontError::new(…, "…")`），排除測試與目錄本身，超過記錄上限（目前 175）即失敗。門禁是一次 AST 遍歷（`tools/staging-budget`，基於 `syn`），計數不依賴行佈局、空白或註解，`matches!(e, Error::Message(m) if …)` 這類**模式**絕不會被誤判為建構。上限只能下調，且這一點是強制的：CI 以 `--against-git` 執行門禁，從 `HEAD` 讀取 `tools/staging-budget/src/ceiling.txt`，工作樹的上限更高即失敗。全部 38 個變體由 `catalogue_completeness_tests` 釘住——逐一建構並在九語下渲染（非空、無洩漏的 `{n}`）。LSP 快速修復的*標題*仍是中文；其診斷已本地化。

```text
python3 scripts/staging_budget.py                # counts + ceiling
python3 scripts/staging_budget.py --against-git  # also: the ceiling may only go down vs HEAD
```

### 編輯器整合（LSP）

`himari-lsp` 是手寫的 LSP 服務（協議層是純函式，附單元測試 + 一個端到端 stdio 測試）：

| 能力 | 說明 |
| --- | --- |
| 診斷 | 在 `didOpen`/`didChange` 之後推送 `publishDiagnostics`（未宣告符號、量綱不符、語法錯誤） |
| 補完 | 檔案中的 `decl` 名稱 + 公式名稱 + 一元函式表（`det/inv/solve/roots/...`） |
| **快速修正** | `textDocument/codeAction`：未宣告符號 → ① 改寫成最接近的已宣告名稱 ② 在 decl 區塊加一行；函式名拼錯 → 從登錄檔給出正確名稱 |
| 同步 | **增量同步**（`textDocumentSync: 2`，按範圍切片）；也接受全量變更，所以舊客戶端不會壞掉 |

快速修正只在名稱**恰好出現一次**時提供替換（出現多次時只提供「新增宣告」，永不
猜哪一處）；超出範圍的值一律**夾取**而非丟棄變更（丟棄一個會讓緩衝區與編輯器永久
不同步）。

### 公式庫內容鎖：庫一變就必須出聲

`use "lib/…"` 會把庫檔案**內聯**，所以庫一改，下游生成的程式碼也跟著改——正是那種
最容易漏掉的回歸。`--lock` 把每個被引用檔案的指紋寫進 `<entry>.lock`，此後**每次載入都驗證鎖**：

```text
$ himari-fmt --lock formulas/entry.formula
已写入 formulas/entry.formula.lock（4 行）

$ himari-eval formulas/entry.formula         # someone quietly changed the library
himari-eval: 公式库与锁文件不一致（formulas/entry.formula.lock）：
  内容变了（声明版本 1.4.2 → 2.0.0）：lib/atmosphere.formula
              复核改动后运行 `himari-fmt --lock <入口>` 重新生成锁。
```

（CLI 輸出逐字引用：工具目前仍以中文印出。英文意思是：庫與其鎖檔不一致——
`lib/atmosphere.formula` 的宣告版本從 1.4.2 變成 2.0.0——審閱變更後重新執行
`himari-fmt --lock <entry>`。）

- **sha256 內容雜湊**（不是 semver 範圍）：對於「內聯原始碼」這種依賴形態，內容雜湊是
  更強的保證——任何變更都會被抓到，不需要版本解析策略，也沒有預發布規則。
- **純文字、確定性、路徑相對於入口目錄**：可以進 git，diff 可讀，且跨機器可比。
- **閉合門禁**：`scripts/fmt_check.py` 會執行 `--lock-check`，對象是有 `.lock` 的檔案
  （並在 `--fix` 下自動重新上鎖）——過期的鎖在門禁中轉紅，不必等有人載入該檔案。
- **選擇加入**：沒有 `.lock` 就不驗證（對既有儲存庫零影響）。一旦有鎖，**新的引用也會失敗**
  （白名單語意）；重新上鎖才能接受。
- **無法讀取的鎖也是失敗**：不是 UTF-8（被編輯器存成 UTF-16）、權限不足，或是一個目錄路徑
  都是硬錯誤——這些永不可被當成「沒有鎖」，否則會悄悄關掉保護。
- **`--fix` 有三個階段**（先全部格式化 → 再全部重新上鎖 → 最後重新生成每個 sidecar）：
  sidecar 載入時會驗證鎖，所以鎖必須先定案；兩者也都依賴被引用庫的最終內容，
  把它們混進同一個迴圈會留下過期產物。
- `%! version: 1.2.0` 只是給人看的欄位（它也被上鎖，版本變更會回報）；語意保證靠的是雜湊。
- 為什麼不用 semver 範圍：那需要整套解析／釘版／預發布策略，而本儲存庫的依賴形態是
  「內聯原始碼」。要表達「允許小幅漂移」，重新上鎖是更直接的宣告。

### 方言指示：檔案車道上對應「巨集就是方言」的規則

檔案沒有巨集可以包住它，所以對應的規則是：**預設 matex；LaTeX 明確宣告。** 兩條車道共用一個原則——
不猜，作者（而不是解析器）決定語意。

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

- 支援 `%! dialect: latex`（或 `tex`）與 `%! dialect: matex`（或 `matlab`）。
- **沒有指示 = matex**。這裡以往有一個 AUTO 後備（matex 解析失敗且原始碼含 `\` 就試 LaTeX），
  它正是產生上面那個 200 對 30 的靜默錯誤值的來源——已移除。
- 沒有方言指示的 LaTeX 專屬記法 → 以預設 matex 解析 → **錯誤**（絕不會是靜默的錯誤值）；
  錯誤會提供出路（寫 `%! dialect: latex`），但**不會替你猜**。
- 只在**第一個非註解行之前**生效（body 註解中的 `%!` 不會改變方言）。
- 無法辨識的值是錯誤。
- 有指示時，錯誤以該方言回報：LaTeX 指示下的語法錯誤就是 LaTeX 錯誤。
- 它只在一個地方實作（`himari_loader::parse_source`），所以 build.rs 車道、`himari-fmt`、
  `himari-eval` 與 LSP 診斷全部一致。（LSP 診斷按 `HIMARI_LANG` 選擇的語言渲染，見「錯誤」一節。）

### LaTeX 表層的除法（曾經缺失）

`a / b` 與 `a \div b` 現在是真正的除法，與 `\frac{a}{b}` 同義，優先序同乘法且左結合。
以往 LaTeX 表層**根本沒有除法運算子**——只接受 `\frac{a}{b}`，而 `a / b` 是語法錯誤。
那是把洗澡水和嬰兒一起倒掉：`/` 是寫除法最平常的方式，沒有理由拒絕它。
（上面的 AUTO 後備其實一直在遮掩這個缺陷：它讓 `a / b`「碰巧能用」，同時把靜默錯誤值一起帶進來。
修好缺陷，後備就沒有存在的理由。）

## 符號線性求解 `solve`（2026-11-23）

單變數形式 `solve(equation, variable)` 把 `equation = 0` 解成 `variable = -b/a`（方程式必須對該變數是**線性**的）。

```text
ES_ss = solve(k_on * E * S - (k_off + k_cat) * ES, ES)
      → ES_ss = k_on·E·S/(k_off + k_cat)          % Michaelis–Menten steady state
```

- 解**仍然是 IR**：所以它保留量綱檢查、求值、程式碼生成、解析導數與
  Jacobian——不需要第二套機制。`ES_ss` 的偏導與 Jacobian 列會自動產生。
- **參數列 = 生成表達式實際用到的變數**：被求解的 `ES` 不會出現在 `ES_ss` 的參數裡
  （「要求它卻又必須傳它」已修正）。Sidecar / 懸停 / 補完讀的是同一份參數列。
- **非線性被明確拒絕**：`x·x`、`1/x`、`x^2`、`sin(x)` **在解析出口**報錯，並指出涉及的種類；
  沒有數值近似，沒有可疑的答案。把化學穩態算錯的代價，遠高於多打幾個字元。

### 方程組形式 `solve([equations…], [unknowns…])`

當兩個引數都是列表時，進入**線性方程組**車道，解是一個**行向量**（矩陣表達式值車道，
回傳 `Vec<Vec<f64>>`）：

```text
x = solve([(k1 + k2) * x1 - k2 * x2 - F1,
           -k2 * x1 + (k2 + k3) * x2 - F2], [x1, x2])
  → x = 2×1 column vector, dimension [m]           % two-mass spring statics
```

- **符號消去**：高斯–喬丹消去在**表達式環**上進行（係數是 IR 子表達式，不是數），
  每一步都化簡到標準形式。解出的每個分量仍是 IR，照常走量綱/求值/生成。
  **但方程組的解是矩陣值的**：矩陣值公式不生成解析導數（逐元素偏導不是矩陣；Jacobian 才是），
  也不生成 f32 變體——這是矩陣值車道既有的限制，不是 `solve` 特有。
- **形狀慣例**：方程式與未知數列表都按列主序**攤平**，所以 `[e1, e2]` 與 `[e1; e2]` 等價——
  使用者不必記方向。未知數列表的項目必須是裸變數名稱。
- **未知數需要已宣告的量綱**：這換來的是**結果量綱的檢查**（彈簧範例解 `m = N/(N/m)`），
  以及方程式本身的**齊次性**檢查（`x1 − F1`，把 m 加到 N 上，在解析期就被拒絕）。
  已解出的未知數不進參數列（生成的函式只接收真正的參數）。
- **拒絕即特性**：欠定（非滿秩）、不相容（消去後出現 `0 = nonzero`）、非線性（未知數出現在某個乘法的兩邊 /
  分母或指數中 / 函式引數內）**全部在解析出口回報**。
  與量綱相關的拒絕發生在 `dims_of_set` / 生成階段（解析器本身不做量綱檢查）。
  冗餘方程式以「可摺疊意義」移除：係數為符號且無法摺成零的冗餘列，保守地
  回報為不相容——寧可拒絕，也不回傳錯誤的解。
- **主元不為零與數值邊界**：與單變數 `solve` 同慣例，無法證明為零的就當作
  非零（我們信任 `a ≠ 0`）。消去在編譯期按表達式順序進行，**執行期不會按 |主元| 重排列**；
  在極端尺度下閉式會相消，奇異參數在值層回報除以零，生成的程式碼依 IEEE 給出 `inf/NaN`
  ——那不是例外。病態方程組，或需要選主元策略的方程組，必須交給數值函式庫。
- **邊界（四條，全部在解析出口或作為編譯期錯誤拒絕）**：

  1. **矩陣值**的係數或常數項（`solve(M*x - b, x)`、`solve([M*x - b], [x])`）→ 明確拒絕，
     並提示改用 `inv(A) * b`——否則純量車道會生成 `vec!` 除法，編譯不過。
  1. **方程式可以有不同量綱**（N 的力平衡加上 m 的幾何約束是合法的方程組），但每個方程式本身必須
     齊次；且解出的每個分量必須同屬**一個量綱**（混合量綱的未知數——同時解電流與
     電阻——尚不支援；那是矩陣值車道的限制）。
  1. 複數車道（帶 `j`）在**值層**與**生成層**都能運作（2026-11-23 完成）：複數行向量
     生成為 `Vec<Vec<(f64, f64)>>`，元素為（實部, 虛部）。交流電路節點分析與 EIS 風格的「複數方程組」
     因此可以直接生成成程式碼。
  1. **冗餘列的零測試是充分條件**：同類項合併能證明的零（`k−k`、`k·a−a·k`、`x·x²−x³`）
     會被辨識；無法證明為零的複雜恆等式會當作非零係數（保守——可能誤報
     欠定/不相容，但不會回傳錯誤的解）。

### 多項式根 `roots(poly, x)`

`solve` 給出線性閉式；`roots` 給出**多項式的所有根**（作為行向量）：

```text
roots_quad = roots(a * x ^ 2 + b * x + c, x)
% q = −(b + sign(b)·√(b²−4ac))/2; large root = q/a, small root = c/q
% Return order still matches the original formula (−b−√Δ first); it does not depend on sorting the roots by size.
```

- **只支援 1–2 次**：1 次與 `solve` 是同一門數學；2 次是閉式求根公式。
  **3 次及以上明確拒絕**，並回報實際次數——Cardano/Ferrari 在浮點下條件數極差，
  本儲存庫不會發一個「看起來很精確」的近似。三次及以上的根，請先數值算出係數，
  再用數值函式庫。
- **重根按重數重複**（`x² − 2x + 1` 給出兩個 1），與代數基本定理一致；
  零根時的 `q=0` 後備是 `-b/(2a)`，不會走 `0/0`。
- **二次公式的相消規避**：直接使用 `(−b±√Δ)/(2a)` 在 `b²≫4ac` 時會丟掉小根
  （`x²−10⁸x+1` 曾把小根 `10⁻⁸` 算成約 `7.45×10⁻⁹`）。現在使用穩定的 `q/a` 與 `c/q`，而生成程式碼中
  小根相對誤差的驗收線是 `1e-12`；兩個正負方向與複數車道都有回歸測試。
- **量綱自動得出**：方程式必須量綱齊次，而齊次性直接蘊含「根的量綱 = 變數的
  量綱」，所以不需額外推導量綱（有一個契約測試守住這個推理）。
- **實數車道的負判別式**：生成的程式碼回傳具型別錯誤，**不是 NaN**——實數二次 `roots` 入口現在回傳
  `Result<_, himari::runtime::NumericError>`，負判別式是 `NumericError::NoRealRoots { discriminant }`。不再有靜默
  NaN 契約，所以 `is_nan`/`is_finite` 檢查不再是偵測這種情況的方法；編譯期可證明為負的常數判別式
  仍然直接拒絕。`eval_value` 是通用的複數值層，對複數車道中的同一表達式**仍會回傳複數根**；
  複數車道需要明確的 `j`。
- **相消的高次項會被剝除**：`x³ − x³ + x − 1` 被當成 1 次（而不是「因為多項式中出現 x³ 就拒絕」）。
- **非多項式明確拒絕**：`x` 出現在函式引數內、分母中，或非整數次方——各自附上理由。
- **矩陣值係數**被拒絕（與 `solve` 同慣例，否則純量車道會生成 `vec!` 除法）。

### 係數向量 `poly_coeffs(poly, x)`

`roots` 的互補：它給出**係數**（由低次到高次），回傳 `1×k` 行向量。

```text
coeffs = poly_coeffs(2 * s ^ 2 - 3 * s + 1, s)   → [[1, −3, 2]]
```

- **變數必須無量綱**。齊次多項式第 k 次係數的量綱是 `dim(poly)/dim(x)^k`，而它們各不相同；
  但矩陣要求**元素同量綱**——所以對有量綱的變數取係數，在那個形式下代數上不可能。
  這道檢查在診斷階段直接說明，並提供出路（除以一個特徵量，或改用 `roots`），而不是讓
  下游回報「矩陣元素量綱不同」。
- 高次零係數會被剝除（`x³ − x³ − 2x + 1` 是 1 次）；零多項式被拒絕。
- 非多項式（`sin(x)`、`1/x`、非整數次方）被拒絕。
- 它共用同一個 `poly_coeffs` 提取器，`roots` 也不例外——係數語意只有一個定義，兩個使用者。

### 三次以上的根：`himari-eval --roots`

在語言中拒絕 ≥ 3 次是必要的取捨，但**數值**求根對任何次數都有效，所以它放在工具層：

```text
$ himari-eval cubic.formula --roots
cubic = 1, 2, 3  [1] (degree 3, 3 roots)
cplx  = -1, 0.5-0.8660254037844387j, 0.5+0.8660254037844386j  [1] (degree 3, 3 roots)
```

- **Durand–Kerner 迭代**（重用 core 的 `C` 做複數算術，不引入第三方數值函式庫）：初值是
  `0.4 + 0.9j` 的冪，半徑是**根量級的幾何平均** `|c₀/cₙ|^(1/n)`（只在退化情況才退回
  Cauchy 界 `1+max|cᵢ|`）；最多 1000 次迭代；DK 準則是 `|Δz| ≤ 1e-10·radius`（相對於根的量級，
  不是相對於 1）；隨後是 **60 步牛頓拋光**（相對停止準則 `1e-15·|z|`）。
- **不收斂是錯誤**，永不把尚未收斂的近似當作結果回傳；迭代中出現 `inf`/`NaN` 也是立即錯誤
  （`f64::max` 對 NaN 回傳另一邊，這曾讓一個 NaN 步看起來「已收斂」並回傳 20 個 NaN）。
- **殘差驗收**：迭代後，相對後向誤差 `|p(z)| / Σ|aₖ|·|z|ᵏ` 必須夠小（門檻 1e-8），
  否則報錯——DK 不保證收斂到根，而準則必須獨立於迭代本身。
  分母**必須包含 `k=n` 項**：沒有它，常數 `|a_n|` 就會在小的 `|z|` 下成為分母，
  而 `x²−1e-100` 會通過，卻錯了 4e25 倍（真實的回歸）。
- **牛頓拋光**（相對準則）：單根可達機器精度；**重根只有線性收斂**，所以 `x²−2x+1` 之類
  的相對誤差在 `1e-9` 範圍，且不報錯——那是演算法的性質，不是 bug；不要把重根當成
  單根。
- **次數上限 16**（`poly_coeffs` 的多項式展開上限；`numeric_roots` 本身支援到 64）。
- 迭代前做 **monic 正規化**（除以首項係數）以避免量級溢位；次數上限 64；`inf`/`NaN` 係數拒絕。
- **雜訊清零只清虛部，且準則是相對的**：實根會帶 `1e-16` 量級的虛部殘餘，那是
  迭代的產物；準則是 `|im| ≤ 1e-12·|z|`。**實部永不觸碰**（`x²−1e-50` 的根是 ±1e-25），
  而且**絕對門檻也不行**——對量級低於 1e-12 的複數根，虛部就是該根的全部資訊
  （`x³−1e-60` 曾因此被抹成三個實根，相對誤差 0.87，卻沒報錯）。
- 係數在 `--set` 給定的參數下必須求值成數；省略變數時，推斷為「公式中唯一宣告的
  符號」，有多個候選時需要 `--var`。
- 輸出帶有次數與根數；複數根寫成 `a+bj`。

## 複數矩陣與複數方程組（2026-11-23）

複數性不只限於純量：矩陣元素與解向量也可以是複數，而且**值層與生成層一致**。
生成函式的回傳型別是 `Vec<Vec<(f64, f64)>>`（元素 =（實部, 虛部））：

```rust
// I = solve([Z*I1 - V, Z*I1 - R*I2], [I1, I2]) with Z = R + jωL
pub fn I(R: f64, w: f64, L: f64, V: f64) -> Vec<Vec<(f64, f64)>> { … }
```

- 典型用途是**交流電路節點分析**與 EIS 建模：未知數是複數電流/電位。
- 轉置就是**純轉置**（不取共軛）——共軛是獨立的 `conj` 運算子；別混淆。
- `adjoint` 是共軛轉置；只有形狀在編譯期已知的字面量才能展開。
- 在複數車道，`det/trace/adjoint/inv/solve/roots/dot/norm` **同樣**在生成前於 IR 層展開（一個歷史上的不對稱：
  實數車道會展開，而複數車道直接跳到 `compile_error!`）。
- 仍不支援：複數公式的 **no_std/libm 車道**（那些閉包依賴 std）、複數公式的**解析導數**
  （`abs/arg/conj/re/im` 不是複解析的，那需要 Wirtinger 微積分），以及複數公式的 **f32 變體**。

## WGSL（GPU）車道：子集 + 驗證邊界（2026-11-23）

`himari-fmt --wgsl <file.formula>` 把實數純量公式生成為 f32 kernel：

```wgsl
// Generated by himari (f32 scalar lane; really compiled and validated by naga)
fn q(rho: f32, v: f32) -> f32 {
    return 0.5f * rho * (v * v);
}
```

**支援的子集（刻意很小）**：`+ - * /`、`sqrt`、`exp`/`log`（本儲存庫的 `ln`）`sin cos tan
sinh cosh tanh abs min max clamp`，加上次方 2/3、平方根，以及次方 -1。
**明確拒絕**：複數（`j`）、矩陣、`det/inv/solve`、Σ/∫、符號運算子、其他次方，
以及與 WGSL 保留字衝突的符號——各自附理由（我們不生成編譯不過的 kernel）。

**驗證有兩層**（自 2026-11-24 起，WGSL 車道有**編譯器層級**的證據）：

1. **真正的編譯門禁**（權威）：`cargo test -p himari-check --features wgsl-compile`
   把 fixtures 中每個生成的 kernel 交給 **naga**（wgpu 的著色器編譯器，WGSL 規格的權威實作之一），做
   真正的解析 + 真正的驗證。naga 是純 CPU 的 dev-dependency——**不需要 GPU**，且永不進入執行期依賴樹。
1. **靜態門禁**（快速篩檢，`wgsl::validate_wgsl`）：括號平衡、`-> f32` 簽名、恰好一個
   `return`、浮點字面量帶 `f`、沒有 `^`/`**`、沒有非有限常數。它的價值在於「在沒有 naga 的環境中抓粗錯
   + 錯誤訊息更貼近這條車道」；**它不是正確性的證明**。

**為什麼真正的編譯門禁是必要的——一個具體教訓**：這條 WGSL 車道的每個 kernel 以往都是**無效的**。
WGSL 沒有表達式主體函式；主體必須是 `return expr;`，而生成器一直只生成裸表達式；
更糟的是，靜態門禁**禁止 `return`**——因為那條規則是在不知道規格那部分的情況下寫的，把誤解
變成了「規則」。靜態門禁的結構性盲點不是「它會漏掉東西」，而是「**它不知道自己漏了什麼，
甚至可能把正確的程式碼判成錯的**」。第一次 naga 執行就暴露了整個類別。
另一個被糾正的信念：`return 2.5;` 在 naga 中是**合法**的（抽象浮點由上下文落成 f32）——所以「浮點必須帶 `f`」是這條車道的
風格慣例，不是 WGSL 規格。

**驗證邊界（誠實說明）**：naga 只驗證**語法與型別**。「kernel 在真正的 GPU 上算出的數字對不對」仍然
未經驗證——那需要 wgpu 執行期 + 一張 GPU（node-3 有一張 RTX 5880，待接線）。
另外，tint（Chrome/Dawn 的實作）與 naga 在少數邊界規則上不同；本儲存庫只對 naga 負責。

## 按領域選擇車道（一張表）

| 情境 | 建議車道 | 備註 |
| --- | --- | --- |
| 大規模科學 HPC / 數值函式庫 | `.formula` + build.rs，實數純量 | 與手寫相當（1.0×）；定長批次變體可去掉邊界檢查 |
| 遊戲引擎 / 音訊 DSP | 同上 + **`{name}__f32`** | 實測 SIMD 增益 1.44×；用 `clamp/min/max` 夾取 |
| 即時 SDR | 實數或複數車道 + 批次 | 卷積/通道矩陣用字面量（常數摺疊） |
| EIS / 電化學 | 複數車道（`j` + `(f64,f64)`） | 相位是物理承諾；掃描用批次；**熱迴圈手寫** |
| FEM / 有限元素 | 矩陣運算子 `det/trace/inv` + Jacobian | `det` 到 4 階、`inv` 到 3 階；更高階改用數值函式庫 |
| 執行期尺寸的線性方程組 | 執行期車道（`_matrix`/`_vector` 宣告） | `solve(A,b)` 有檢查（rcond + 殘差）；回傳 `Result` |
| 量綱探索 | `himari-eval --groups` | Buckingham Π 群：不用手推雷諾數 |

> 另一邊也還有些中文殘留：生成程式碼的 doc comment 與診斷字串，因為這條車道的
> 訊息目前以中文撰寫（見上面引用的 doc comment）。這些是單行字串字面量。

**帶脫字符的錯誤**、**函式名拼錯的建議**（`closest_func`，編輯距離 ≤ 2）、
**拒絕不可見字元**、**深度/步數上限**——這些不是裝飾，而是「敢讓同事用這個」的門檻。

## 狀態

在 `dev` 分支上迭代（本機儲存庫，尚未推送到遠端）。crates.io / docs.rs 徽章是預留的入口；
**目前還沒發布任何東西**；正式 crates 上的 `publish = false` 是人工審查鎖，而範例與 fixtures 也維持不可發布。
內部依賴已經帶著 `version = "0.1"` 與本機路徑，一旦解除審查，必須按拓撲順序發布：
`himari → himari-latex / himari-matlab → himari-loader → himari-buildhelper → himari-macros`。
`himari-check` 工具發行另行決定。`himari-standards` 與 `himari-lean` 是工作區中新的 `publish = false` crates；
它們分別依賴 `himari-macros` 與 `himari-loader`，所以不改變那個發布順序。
路線圖與取捨的歷史：`_reports/math-formula-20261122/`（REPORT / SPIKE-S0 / SYMBOLS-AND-PAPER）。

## 驗證

```bash
cargo test --workspace --offline
cargo test -p himari-check --features wgsl-compile --test wgsl_compile --offline
cargo run -p himari-check --release --offline
python3 scripts/fmt_check.py
cargo test -p himari-example-common --offline
```

門禁涵蓋核心代數、實數生成程式碼的編譯與數值不變量、獨立撰寫的消費端範例、
WGSL 的真正 naga 編譯，以及標準形式/sidecar/鎖與生成程式碼的漂移。確切的測試數量以當前 `cargo test --workspace` 的輸出為準；
效能數字只描述本儲存庫的 bench 環境，不是跨平台的保證。

## 未宣稱

這些是範圍限制，不是一份快要完成的待辦清單。寫「未宣稱」的地方就是「未宣稱」。

- **符號車道沒有執行期選主元**：符號 `solve` 按表達式順序消去並信任 `a ≠ 0`；它不會按
  主元量級重排列。病態或對選主元敏感的方程組，屬於執行期車道（上面的 `solve(A,b)`）或數值函式庫。
- **樣式矩陣只能走檔案車道**：樣式矩陣的建構子車道需要檔名與生成器，所以內嵌巨集
  會附提示拒絕它們；沒有任意樣式矩陣的自動展開（那是研究級——內建形狀之外的東西一律明確拒絕，
  並附上支援清單）。
- **WGSL 數值驗證仍待完成**：naga 證明語法與型別，不證明 kernel 在 GPU 上算出的數字正確。這裡尚未做過任何 GPU 執行。
- **Lean 驗證尚未成功**：本主機沒有安裝工具鏈，所以只跑了 stub-process 測試。沒有宣稱 Lean 接受
  生成的證明義務。
- **`publish = false` 是人工審查門**：crates 刻意不可發布，直到人工審查解除；見「狀態」。
- **`no_std` 範圍**：`himari`、`himari-latex` 與 `himari-matlab` 是 `no_std + alloc`；loader/buildhelper/macros/check crates 則不是。
  生成的程式碼保留自己的 libm 車道，而複數公式不屬於 `no_std` 車道。
- **沒有複數解析導數、沒有複數或矩陣值公式的 f32 變體、沒有矩陣值公式的解析導數。**
- **`roots` 在語言中只涵蓋 1–2 次**；更高次只能透過 `himari-eval --roots` 做數值求解。
- 一般 LaTeX（巨集/catcode/排版）——語意不可判定；排版屬於呈現層。
- 不從字形推斷語意，MATLAB 表層也沒有 `xy` 乘法。沒有 `\sin^2 x`、沒有 `.*`、沒有 `$…$`（歧義或純
  排印記法：解析與呈現是兩件不同的工作，一個解析器不該兩件都做）。
- 即時/安全關鍵路徑上沒有執行期 JIT；永不引入 AGPL 依賴（evalexpr ≥12 之類已出局）。
- **`himari-standards`** 模組是條文層級的計算器，**不是符合性宣稱**：它們不建模試片幾何、力值級別的選擇、
  持壓時間、不確定度預算，或標準本身的報告規則，也不是通用的單位函式庫。
- **`himari-lean`** 車道證明一個小小的封閉整數義務（一個 3×3 行列式範例）；它不是通用定理證明器，而它的公理
  門禁是對 Lean 自身輸出的解析器，不是健全性論證。
