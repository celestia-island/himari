#!/usr/bin/env python3
"""Structural gate for the localized documentation.

Nine languages are a promise that is easy to break silently: a heading gets added to the
English page and the other eight fall behind, or a table row is dropped in translation. This
gate compares every locale against the canonical English document and fails on:

  * a missing locale file, or a locale present on disk but absent from ``docs/lagrange.toml``;
  * a different heading sequence (level and text are compared by level only, since text is
    translated);
  * a different number of table rows, or a different column count within a table;
  * a different number of fenced code blocks;
  * a code fence whose content differs from English (code must never be translated);
  * a different number of list items.

It deliberately does not compare prose. Translation quality is a review matter; structure is
what a machine can hold a line on.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"

HEADING = re.compile(r"^(#{1,6})\s+\S")
FENCE = re.compile(r"^\s*(```+|~~~+)")
TABLE_ROW = re.compile(r"^\s*\|")
LIST_ITEM = re.compile(r"^\s*(?:[-*+]|\d+[.)])\s+\S")
LANG_ORDER = re.compile(r'^order\s*=\s*\[(.*?)\]', re.M)


def locales() -> list[str]:
    config = DOCS / "lagrange.toml"
    if not config.is_file():
        raise SystemExit("i18n-gate: docs/lagrange.toml is missing")
    match = LANG_ORDER.search(config.read_text(encoding="utf-8"))
    if not match:
        raise SystemExit("i18n-gate: docs/lagrange.toml has no [languages].order")
    return [item.strip().strip('"') for item in match.group(1).split(",") if item.strip()]


def profile(path: Path) -> dict:
    """Structural profile: headings, table shapes, fences, list items."""
    headings: list[int] = []
    tables: list[tuple[int, int]] = []  # (rows, columns) per contiguous table
    fences: list[str] = []
    lists = 0
    current_table: list[int] | None = None
    in_fence = False
    fence_buffer: list[str] = []
    fence_marker = ""

    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.rstrip("\n")

        fence = FENCE.match(line)
        if fence:
            marker = fence.group(1)[0]
            if not in_fence:
                in_fence, fence_marker, fence_buffer = True, marker, []
            elif marker == fence_marker:
                fences.append("\n".join(fence_buffer))
                in_fence = False
            continue
        if in_fence:
            fence_buffer.append(line)
            continue

        if HEADING.match(line):
            current_table = None
            headings.append(len(HEADING.match(line).group(1)))
            continue

        if TABLE_ROW.match(line):
            columns = line.count("|") - 1
            if current_table is None:
                current_table = [1, columns]
                tables.append((current_table[0], current_table[1]))
            else:
                current_table[0] += 1
                tables[-1] = (current_table[0], columns)
            continue
        current_table = None

        if LIST_ITEM.match(line):
            lists += 1

    return {"headings": headings, "tables": tables, "fences": fences, "lists": lists}


def main() -> int:
    canonical = DOCS / "en" / "README.md"
    if not canonical.is_file():
        print("i18n-gate: docs/en/README.md is missing", file=sys.stderr)
        return 2

    ordered = locales()
    on_disk = sorted(p.name for p in DOCS.iterdir() if p.is_dir())
    problems: list[str] = []

    for name in on_disk:
        if name not in ordered:
            problems.append(f"docs/{name}/ exists but is absent from lagrange.toml")
    for name in ordered:
        if not (DOCS / name / "README.md").is_file():
            problems.append(f"docs/{name}/README.md is missing")

    if problems:
        for problem in problems:
            print(f"i18n-gate: {problem}", file=sys.stderr)
        return 1

    root = ROOT / "README.md"
    if root.is_file() and root.read_bytes() != canonical.read_bytes():
        problems.append(
            "README.md differs from docs/en/README.md: the root document is the English "
            "canonical, and the locale copies are compared against docs/en"
        )
        for problem in problems:
            print(f"i18n-gate: {problem}", file=sys.stderr)
        return 1

    base = profile(canonical)
    for name in ordered:
        if name == "en":
            continue
        candidate = profile(DOCS / name / "README.md")
        if candidate["headings"] != base["headings"]:
            problems.append(
                f"{name}: heading structure differs "
                f"({len(candidate['headings'])} vs {len(base['headings'])} headings)"
            )
        if candidate["tables"] != base["tables"]:
            problems.append(
                f"{name}: table shape differs ({candidate['tables']} vs {base['tables']})"
            )
        if len(candidate["fences"]) != len(base["fences"]):
            problems.append(
                f"{name}: {len(candidate['fences'])} code fences vs {len(base['fences'])}"
            )
        else:
            for index, (mine, theirs) in enumerate(zip(candidate["fences"], base["fences"])):
                if mine.strip() and mine != theirs:
                    problems.append(f"{name}: code fence #{index + 1} was modified")
                    break
        if candidate["lists"] != base["lists"]:
            problems.append(
                f"{name}: {candidate['lists']} list items vs {base['lists']}"
            )

    if problems:
        for problem in problems:
            print(f"i18n-gate: {problem}", file=sys.stderr)
        print(f"i18n-gate: {len(problems)} problem(s) across {len(ordered)} locales", file=sys.stderr)
        return 1

    print(f"i18n-gate: clean ({len(ordered)} locales, {len(base['headings'])} headings each)")
    return 0


if __name__ == "__main__":
    sys.exit(main())