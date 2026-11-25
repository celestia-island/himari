<!-- markdownlint-disable MD033 MD041 -->
<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/celestia-island.github.io/main/res/logos/himari.webp" alt="Himari" width="200" /></p>

<h1 align="center">Himari</h1>

<p align="center"><strong>Formula assets: one source of truth, three surfaces, compiled into Rust.</strong></p>

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

**English** ·
[简体中文](./docs/zh-Hans/README.md) ·
[繁體中文](./docs/zh-Hant/README.md) ·
[日本語](./docs/ja/README.md) ·
[한국어](./docs/ko/README.md) ·
[Français](./docs/fr/README.md) ·
[Español](./docs/es/README.md) ·
[Русский](./docs/ru/README.md) ·
[العربية](./docs/ar/README.md)

</div>
<!-- markdownlint-enable MD033 MD041 -->

<!-- This README is a short entry stub. The full English document lives in docs/en/README.md
and is the canonical source every locale is checked against by scripts/i18n_check.py. -->

Himari turns **one formula asset** — written in a LaTeX subset, paper notation, or MATLAB style —
into checked Rust code. The source lowers to a single dimensioned IR, so the same formula can be
evaluated, differentiated, formatted, and emitted as an f64/f32 Rust function, a WGSL kernel, or a
checked-in build.rs artifact — with the seven base quantities (kg·m·s·A·K·mol·cd) checked at
compile time: `m + s` does not compile. The capability list, the language guide and the error
catalogue live in [the documentation](https://celestia-island.github.io/himari/).

## Quickstart

```bash
cargo add himari himari-macros
```

```rust
use himari_macros::formula_matlab;

let v0 = 12.0f64;
let g = 9.80665f64;
let t = 3.0f64;
let v = formula_matlab! {
    decl { v0 : speed [m/s]; g : acceleration [m/s^2]; t : time [s]; }
    v0 + g * t
};
assert!((v - 41.41995).abs() < 1e-9); // dimensions checked at compile time
```

Seven runnable examples live in [`examples/`](examples/README.md) — inline macros, unit
conversion, symbolic differentiation, WGSL emission, the error catalogue, matrix idioms and a
cross-file `.formula` library with a build.rs lane:

```bash
cargo run -p himari-example-quickstart
```

## Documentation

The full documentation is built from [`docs/`](docs/) and published at
<https://celestia-island.github.io/himari/>. The full English document — the canonical source every
translation is checked against — is [`docs/en/README.md`](docs/en/README.md); each locale mirrors it:

[English](./docs/en/README.md) ·
[简体中文](./docs/zh-Hans/README.md) ·
[繁體中文](./docs/zh-Hant/README.md) ·
[日本語](./docs/ja/README.md) ·
[한국어](./docs/ko/README.md) ·
[Français](./docs/fr/README.md) ·
[Español](./docs/es/README.md) ·
[Русский](./docs/ru/README.md) ·
[العربية](./docs/ar/README.md)

## License

Licensed under [SySL 1.0](https://github.com/celestia-island/sysl); the full SySL NOTICE text
(including model disclosure requirements) lives in [`docs/en/README.md`](docs/en/README.md) and
[`LICENSE.txt`](LICENSE.txt).
