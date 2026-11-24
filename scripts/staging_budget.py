#!/usr/bin/env python3
"""Thin launcher for the AST-based staging-budget gate (`tools/staging-budget`).

The counting lives in Rust (syn AST walk) so the verdict never depends on line layout,
whitespace, or comments. This shim keeps the `python3 scripts/staging_budget.py` entry point
that `just verify`, CI and the README reference. Passes `--against-git` / `--root` through.
"""

import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent


def main() -> int:
    env = dict(os.environ)
    cargo_home = env.get("CARGO_HOME")
    if not cargo_home:
        # Reuse the workspace convention so the gate never touches a shared registry.
        env["CARGO_HOME"] = "/tmp/himari-cargohome"
    cmd = [
        "cargo", "run", "--quiet", "-p", "himari-staging-budget",
        "--offline", "--", *sys.argv[1:],
    ]
    return subprocess.run(cmd, cwd=ROOT, env=env).returncode


if __name__ == "__main__":
    sys.exit(main())
