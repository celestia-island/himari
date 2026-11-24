#!/usr/bin/env python3
"""漂移门：所有 .formula 文件必须处于 canonical 格式（himari-fmt --check）。

同时拦截 pest 文法里的行首 `%` 注释（pest 只认 //；历史上踩过 4 次的坑，
机械防线）。

用法（容器车道内）:
    python3 scripts/fmt_check.py           # 只检查（门禁用）
    python3 scripts/fmt_check.py --fix     # 就地修复 canonical 格式与 sidecar
"""

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

def run_fmt(*args: str) -> int:
    return subprocess.run(["cargo", "run", "-q", "-p", "himari-check", "--bin", "himari-fmt", "--offline", "--", *args], cwd = ROOT).returncode

def main(argv: list[str]) -> int:
    fix = "--fix" in argv
    rc = 0

    # 1) pest 文法行首 % 注释（非法注释语法）
    for g in ROOT.rglob("*.pest"):
        if "target" in g.parts:
            continue

        bad = [(i, line)for i, line in enumerate(g.read_text(encoding = "utf-8").splitlines(), 1)if line.startswith("%")]

        if bad:
            print(f"pest 注释非法（用 //）：{g}")

            for i, line in bad:
                print(f"  {i}: {line}")

            rc = 1

    # 2) .formula canonical 格式 + sidecar（latex+mathml）漂移 + 内容锁
    #
    # 这里有**跨文件依赖**，必须分阶段：
    #   · sidecar 是联邦展开后的产物 —— 入口公式的 sidecar 会内联它 `use` 的库内容，
    #     所以库的格式化必须在**所有** sidecar 生成之前完成；
    #   · 内容锁记录的是被引用文件的内容哈希 —— 同理。
    # 曾经把三件事塞进同一个 per-file 循环，按 `sorted(rglob)` 顺序处理：
    # `--fix` 会先给入口重 lock、再把库格式化 → 留下过期锁/漂移，打印 clean 但树上脏，
    # 得再跑一次才干净（复核实测过）。
    formulas = [f for f in sorted(ROOT.rglob("*.formula")) if "target" not in f.parts]

    # 阶段 A：canonical 格式
    for f in formulas:
        if run_fmt("--check", str(f)) != 0:
            if fix:
                run_fmt("--write", str(f))
                print(f"已格式化：{f.relative_to(ROOT)}")
            else:
                rc = 1

    # 阶段 B：内容锁 —— 必须在 sidecar **之前**：
    # `--sidecar` 会走 `himari_loader::load()`，而 load 会校验锁；锁还过期就直接失败。
    for f in formulas:
        if not (f.with_suffix(f.suffix + ".lock")).exists():
            continue

        if run_fmt("--lock-check", str(f)) != 0:
            if fix:
                run_fmt("--lock", str(f))
                print(f"已重新上锁：{f.relative_to(ROOT)}")
            else:
                rc = 1

    # 阶段 C：sidecar（latex + mathml）—— 此时公式内容与锁都是最终的
    for f in formulas:
        if run_fmt("--sidecar-check", str(f)) != 0:
            if fix:
                run_fmt("--sidecar", str(f))
                print(f"已重生成 sidecar：{f.relative_to(ROOT)}")
            else:
                rc = 1

    if rc == 0:
        print("fmt-gate: clean")
    else:
        print("fmt-gate: 存在漂移 —— python3 scripts/fmt_check.py --fix 修复")

    return rc

if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
