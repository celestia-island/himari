# Examples

Every directory is a runnable crate: `cargo run -p himari-example-<name>` from the repository
root. Each one compiles against the published-shaped library API and asserts its own expected
numbers, so they double as integration tests.

| crate | demonstrates |
| --- | --- |
| [`quickstart`](quickstart/) | the shortest tour: inline MATLAB and LaTeX macros, a runtime `solve`, and the raw `Expr` + `eval` lane underneath |
| [`units`](units/) | unit conversion (`convert`), dimension algebra through `dim_check`, typed dimension/unit errors, and the same diagnostic rendered in two locales |
| [`symbolic`](symbolic/) | symbolic differentiation on the IR (`diff` + `simplify`), evaluated and checked against closed forms; second derivative; `nondiffable` reporting |
| [`wgsl`](wgsl/) | emitting WGSL kernels with the production emitter, the structural gate, and what a refusal looks like |
| [`errors`](errors/) | the error catalogue: five rejections with their stable codes, rendered in three locales each |
| [`common`](common/) | matrix idioms: rotation, linear systems, least squares, quadratic and cubic roots, complex circuits, inline macros — every number checked against a hand calculation |
| [`reentry`](reentry/) | the full build.rs lane: a cross-file `.formula` library with `use` federation, generated code checked into git, and a physical invariant (terminal-velocity drag = weight) proven by the generated code itself |

## Which lane should I use?

- **Inline macro** (`formula_matlab!` / `formula_latex!`) — simple formulas; dimensions are checked
  at compile time; see `quickstart`.
- **String macro** with a runtime `solve` — when the formula shape is only known at runtime
  (`Result`-returning); see `quickstart` and `common`.
- **`.formula` files + build.rs** — the formal lane for libraries and complex formulas; generated
  code is checked in and guarded by a drift gate; see `reentry` (and `common`, which mixes both
  lanes).

The lane discipline (which surface for which formula) is a repository rule; the long-form guide
lives in [the documentation](https://celestia-island.github.io/himari/).
