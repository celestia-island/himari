#!/usr/bin/env python3
"""Cross-build the no_std surface for real embedded targets.

The claim "himari is no_std" is only worth anything if it compiles for a target
that has no std at all. This gate builds, offline:

  * ``himari`` alone for ``riscv32imc-unknown-none-elf``. That target has no
    pointer atomics at all, so a successful link is the proof that the core crate
    never reaches ``alloc::sync`` (this gate does not statically scan for it),
  * ``himari`` + both parser front-ends for ``thumbv7em-none-eabi`` and
    ``x86_64-unknown-none`` (the front-ends need atomics because ``pest`` keeps
    its parser state behind ``alloc::sync::Arc``).

Missing targets are an error, not a skip: a silent skip would turn this gate
into a no-op that always passes.
"""

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

BUILDS = [
    ("core-only", ["-p", "himari"], "riscv32imc-unknown-none-elf"),
    ("parsers", ["-p", "himari-latex", "-p", "himari-matlab"], "thumbv7em-none-eabi"),
    ("parsers", ["-p", "himari-latex", "-p", "himari-matlab"], "x86_64-unknown-none"),
]


class TargetProbeError(RuntimeError):
    """Raised when target availability cannot be determined."""


def installed_targets() -> set[str]:
    """Return installed target triples, or fail closed.

    Falling back to the full ``rustc --print target-list`` output would silently
    turn "rustup is unavailable" into "every target is installed", which is the
    exact fake-green this gate exists to prevent.
    """
    if shutil.which("rustup") is None:
        raise TargetProbeError("rustup not found; cannot determine installed targets")
    installed = subprocess.run(
        ["rustup", "target", "list", "--installed"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if installed.returncode != 0:
        raise TargetProbeError(
            "`rustup target list --installed` failed: "
            + installed.stderr.strip()[:200]
        )
    return set(installed.stdout.split())


def main() -> int:
    if shutil.which("cargo") is None:
        print("nostd-gate: cargo not found", file=sys.stderr)
        return 2
    try:
        available = installed_targets()
    except TargetProbeError as error:
        print(f"nostd-gate: {error}", file=sys.stderr)
        return 2
    rc = 0
    for label, packages, target in BUILDS:
        if target not in available:
            print(
                f"nostd-gate: target {target} is not installed "
                f"(rustup target add {target})",
                file=sys.stderr,
            )
            rc = 1
            continue
        cmd = ["cargo", "build", *packages, "--no-default-features", "--target", target, "--offline"]
        print(f"nostd-gate: {label} -> {target}")
        done = subprocess.run(cmd, cwd=ROOT)
        if done.returncode != 0:
            rc = 1
    print("nostd-gate: clean" if rc == 0 else "nostd-gate: failed")
    return rc


if __name__ == "__main__":
    sys.exit(main())