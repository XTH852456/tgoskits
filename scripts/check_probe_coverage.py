#!/usr/bin/env python3
"""Verify catalog `tests:` paths exist (contract probes registered in YAML)."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import yaml


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--catalog", type=Path, default=Path("docs/starryos-syscall-catalog.yaml"))
    ap.add_argument(
        "--root",
        type=Path,
        default=None,
        help="workspace root (default: parent of catalog's parent's parent)",
    )
    args = ap.parse_args()
    root = args.root
    if root is None:
        root = args.catalog.resolve().parent.parent

    data = yaml.safe_load(args.catalog.read_text(encoding="utf-8"))
    entries = data.get("syscalls") or []
    missing: list[str] = []
    for e in entries:
        if not isinstance(e, dict):
            continue
        name = e.get("syscall")
        tests = e.get("tests") or []
        for rel in tests:
            p = root / str(rel)
            if not p.is_file():
                missing.append(f"{name}: {rel}")

    if missing:
        print("Missing contract files:", file=sys.stderr)
        for m in missing:
            print(f"  {m}", file=sys.stderr)
        return 1
    print("Probe coverage OK: all catalog test paths exist.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
