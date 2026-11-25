set shell := ["bash", "-c"]
# `which()` (used by the devtools just module) needs list support, which is an
# unstable setting on the pinned just 1.59 — the org convention (kirino/tairitsu/plana)
# is `set unstable` + `set lists` together.
set unstable
set lists

import? "./.just/celestia-devtools.just"

_default:
    @just --list

# Type-check all targets.
check:
    cargo check --workspace --all-targets

# Run the full unit-test suite.
test:
    cargo test --workspace

# Compile every emitted WGSL kernel with naga (real shader compiler; CPU-only, no GPU needed).
test-wgsl:
    cargo test -p himari-check --features wgsl-compile --test wgsl_compile

# Everything a change should pass: tests + naga WGSL gate + harness.
verify: test test-wgsl nostd i18n-check staging-budget harness
    @echo "verify: all gates green"

# Structural gate for the nine localized docs (headings/tables/fences/lists must match English).
i18n-check:
    python3 scripts/i18n_check.py

# Uncatalogued-error budget: staging may only shrink (see scripts/staging_budget.py).
staging-budget:
    python3 scripts/staging_budget.py --against-git

# Cross-build the no_std surface for real embedded targets (no std at all).
nostd:
    python3 scripts/nostd_check.py

# Execute the emitted WGSL on a real Vulkan device and compare the numbers with the CPU lane.
# Needs a Vulkan loader and a compute device; Mesa lavapipe (software) counts, a real GPU is
# better. Exits 2 without printing a verdict when no device exists — absence of a device is not
# evidence that the kernels are correct.
gpu-verify *args:
    cargo run -q -p himari-gpu-verify -- {{args}}

# Generate the Lean proof obligation and check it with a real toolchain.
# Needs `lean` on PATH or HIMARI_LEAN_BIN pointing at it (pinned: leanprover/lean4:v4.36.0).
# Without one this exits non-zero on purpose: no toolchain means nothing was checked.
# Writes build/lean/det3.lean so the obligation can be reviewed or checked elsewhere.
lean-verify:
    mkdir -p build/lean
    echo '{"theorem":"det3","values":[1,2,3,4,5,6,7,8,10]}' | \
        cargo run -q -p himari-lean --bin himari-lean -- --emit build/lean/det3.lean
    cargo test -p himari-lean --test cli_real_lean -- --ignored

# Evaluate one standards request: just standard standards/examples/brinell.json
standard input:
    cargo run -q -p himari-standards --bin himari-standard -- --input {{input}}

# Run the conformance harness (item count is printed by the harness itself).
harness:
    cargo run -p himari-check --release

# Canonical-format gate for .formula files + pest-comment guard (container lane).
# The sandbox mounts THIS checkout (`$PWD` — works from the main checkout and from any worktree).
fmt-check:
    python3 /mnt/codespace/_tools/sandbox_run.py run --label himari-hygiene --reuse-target \
        --src "$PWD" -- python3 scripts/fmt_check.py

# Canonical-format a single formula file to stdout.
fmt file:
    cargo run -p himari-check --bin himari-fmt -- {{file}}

# Evaluate a formula file: just eval formulas/x.formula "--set T=298.15"
eval file *args:
    cargo run -q -p himari-check --bin himari-eval -- {{file}} {{args}}

# Regenerate the content lock for a formula file's `use`d libraries.
lock file:
    cargo run -q -p himari-check --bin himari-fmt -- --lock {{file}}

# Diagnose a formula file (parse + dims + shape; silent when clean).
diagnose file:
    cargo run -q -p himari-check --bin himari-fmt -- --diagnose {{file}}

# List declared symbols and formula names (editor completion data).
symbols file:
    cargo run -q -p himari-check --bin himari-fmt -- --symbols {{file}}

# Clippy across the workspace (warnings deny).
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Regenerate checked-in codegen artifacts (fixtures + runnable examples).
# 编译与 git 比对分开：前者生成，后者证明生成物与真源一致。
regen:
    python3 /mnt/codespace/_tools/sandbox_run.py run --label himari-regen --reuse-target \
        --src "$PWD" -- cargo build -p himari-fixture-three --offline
    python3 /mnt/codespace/_tools/sandbox_run.py run --label himari-regen --reuse-target \
        --src "$PWD" -- cargo build -p himari-example-reentry --offline
    python3 /mnt/codespace/_tools/sandbox_run.py run --label himari-regen --reuse-target \
        --src "$PWD" -- cargo build -p himari-example-common --offline
    python3 /mnt/codespace/_tools/sandbox_run.py run --label himari-regen --reuse-target \
        --src "$PWD" -- cargo build -p himari-fixture-nostd --offline

# Run the quickstart example (formula file → generated code → table + invariants).
example:
    python3 /mnt/codespace/_tools/sandbox_run.py run --label himari-example --reuse-target \
        --src "$PWD" -- cargo run -q -p himari-example-reentry --offline

# Run the common matrix / linear-system / least-squares / roots examples.
example-common:
    python3 /mnt/codespace/_tools/sandbox_run.py run --label himari-example --reuse-target \
        --src "$PWD" -- cargo run -q -p himari-example-common --offline

# Drift gate: regenerate first, then compare checked-in code with its sources (host-side git).
drift: regen
    @git diff --exit-code -- packages/check/fixtures/three/src/generated.rs \
        examples/reentry/src/generated.rs \
        examples/common/src/generated.rs \
        packages/check/fixtures/nostd/src/generated.rs \
        && echo "drift: clean" || (echo "drift: 生成物过期 —— just regen 后提交"; exit 1)

# Devtools hygiene: markdown format, toml sort, depspec, staged justfile, py-style.
hygiene:
    PYTHONPATH=/mnt/codespace/celestia-devtools python3 -m celestia_devtools format-markdown --check . || exit 1
    PYTHONPATH=/mnt/codespace/celestia-devtools python3 -m celestia_devtools.repo.toml_sort --check . || exit 1
    PYTHONPATH=/mnt/codespace/celestia-devtools python3 -m celestia_devtools depspec-lint --strict || exit 1
    PYTHONPATH=/mnt/codespace/celestia-devtools python3 -m celestia_devtools init --check || exit 1
    PYTHONPATH=/mnt/codespace/celestia-devtools python3 -m celestia_devtools py-style check scripts/ || exit 1

# Everything CI runs locally.
gate: check test harness fmt-check drift hygiene
