#!/usr/bin/env python3
"""Refresh or check the file catalogue in the root INDEX.md. No Git mutations."""

import argparse
from collections import defaultdict
from pathlib import Path
import subprocess
from urllib.parse import quote


ROOT = Path(__file__).resolve().parents[1]
START = "<!-- project-files:start -->"
END = "<!-- project-files:end -->"
EXCLUDED_DIRS = {
    ".git", ".worktrees", ".beads", ".pnpm-store", "node_modules", "target",
    ".next", "out", "test-results", "playwright-report", "graphify-out",
}


def catalogue():
    output = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=ROOT,
    )
    groups = defaultdict(list)
    for name in sorted(set(output.decode("utf-8").split("\0"))):
        path = Path(name)
        if not name or not (ROOT / path).is_file():
            continue
        if any(part in EXCLUDED_DIRS for part in path.parts):
            continue
        if name == ".beads.gate.lock" or name.startswith("artifacts/release-gate/"):
            continue
        if path.name == ".env" or path.name.startswith(".env."):
            continue
        groups[str(path.parent)].append(name)
    count = sum(map(len, groups.values()))
    lines = [START, "", f"Всего файлов: **{count}**. Каталог включает существующие tracked и",
             "неигнорируемые untracked-файлы; наличие в списке не означает коммит или готовность.",
             "Служебные каталоги и локальные runtime-артефакты описаны выше, без обхода содержимого.", ""]
    for directory, names in sorted(groups.items()):
        label = "Корень" if directory == "." else directory
        lines += ["<details>", f"<summary>{label} — {len(names)}</summary>", ""]
        for name in names:
            lines.append(f"- [{Path(name).name}]({quote(name, safe='/._-')})")
        lines += ["", "</details>", ""]
    lines.append(END)
    return "\n".join(lines), count


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Fail if the catalogue needs updating")
    args = parser.parse_args()
    index = ROOT / "INDEX.md"
    current = index.read_text(encoding="utf-8")
    if current.count(START) != 1 or current.count(END) != 1:
        raise SystemExit("INDEX.md must contain one catalogue marker pair")
    start = current.index(START)
    end = current.index(END) + len(END)
    if end <= start:
        raise SystemExit("INDEX.md catalogue markers are reversed")
    section, count = catalogue()
    updated = current[:start] + section + current[end:]
    if args.check:
        if current != updated:
            raise SystemExit("Catalogue is stale; run python3 scripts/project-index.py")
        print(f"INDEX.md: {count} files, current")
    else:
        index.write_text(updated, encoding="utf-8")
        print(f"INDEX.md: indexed {count} files")


if __name__ == "__main__":
    main()
