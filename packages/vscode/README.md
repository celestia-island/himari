# himari-formula (VS Code)

`.formula` 文件的语言支持骨架：语法高亮（decl 块 / 矩阵 / 算子 ∂∇∑∫ / 希腊 /
省略号）、注释、括号配对（含 `\begin{bmatrix}` 自动闭合）、decl 折叠。

安装：`code --install-extension` 打包（`vsce package`）或 symlink 到
`~/.vscode/extensions/`。

**snippets**：`decl` / `decln`（声明块）、`tridiag` / `tridiags`（模式矩阵向量/标量形态）、
`ident`（单位阵）、`bmat`（bmatrix）、`ddx` / `ddt`（偏微分两面）、`sum` / `suml`、`intg`、
`fed`（联邦公式块）。

**hover 公式渲染（sidecar 车道）**：`himari-fmt --sidecar <file>` 生成
`<file>.json`（latex+mathml，checked-in、漂移门覆盖）；hover 时扩展把该公式的
latex 塞进 `$$…$$`，由 VS Code 内置 KaTeX 渲染成排版公式 —— **零 wasm**。
改了 `.formula` 后跑 sidecar 重生成（进 `just` 门）即可。
