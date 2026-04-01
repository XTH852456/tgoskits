#!/usr/bin/env python3
"""Extract Sysno arms from StarryOS handle_syscall match in mod.rs."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

try:
    import yaml  # type: ignore
except ImportError:
    yaml = None  # type: ignore


def _split_match_block(text: str) -> str | None:
    needle = "let result = match sysno {"
    start = text.find(needle)
    if start < 0:
        return None
    start += len(needle)
    end = text.find("\n        _ => {", start)
    if end < 0:
        end = text.find("\n        _ =>", start)
    if end < 0:
        return None
    return text[start:end]


def _parse_cfg(line: str) -> str | None:
    s = line.strip()
    if s.startswith("#[cfg(") or s.startswith("#[cfg_attr("):
        return s
    return None


def extract_dispatch(mod_rs: str) -> list[dict[str, Any]]:
    block = _split_match_block(mod_rs)
    if block is None:
        raise ValueError("could not find handle_syscall match block in mod.rs")

    lines = block.splitlines()
    last_section: str | None = None
    pending_cfgs: list[str] = []
    buffer = ""
    out: list[dict[str, Any]] = []

    def flush_buffer() -> None:
        nonlocal buffer
        if "=>" not in buffer:
            return
        arm, _rest = buffer.split("=>", 1)
        names = re.findall(r"Sysno::(\w+)", arm)
        cfgs = pending_cfgs.copy()
        for name in names:
            out.append(
                {
                    "syscall": name,
                    "section_comment": last_section,
                    "cfgs": cfgs,
                }
            )
        buffer = ""

    for line in lines:
        stripped = line.strip()
        if stripped.startswith("//") and not stripped.startswith("///"):
            flush_buffer()
            last_section = stripped[2:].strip()
            continue
        cfg = _parse_cfg(line)
        if cfg is not None:
            flush_buffer()
            pending_cfgs.append(cfg)
            continue

        buffer += " " + line
        if "=>" in buffer:
            flush_buffer()
            buffer = ""
            pending_cfgs = []

    seen: dict[str, dict[str, Any]] = {}
    order: list[str] = []
    for row in out:
        name = row["syscall"]
        if name not in seen:
            order.append(name)
        seen[name] = row
    return [seen[k] for k in order]


def load_catalog_syscalls(catalog_path: Path) -> list[str]:
    if yaml is None:
        print("PyYAML not installed; skip or: pip install pyyaml", file=sys.stderr)
        sys.exit(2)
    data = yaml.safe_load(catalog_path.read_text(encoding="utf-8"))
    entries = data.get("syscalls") or []
    names: list[str] = []
    for e in entries:
        if isinstance(e, dict) and "syscall" in e:
            names.append(str(e["syscall"]))
    return names


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--mod-rs", type=Path, default=Path("os/StarryOS/kernel/src/syscall/mod.rs"))
    ap.add_argument("--out-json", type=Path, default=Path("docs/starryos-syscall-dispatch.json"))
    ap.add_argument("--check-catalog", type=Path, default=None)
    args = ap.parse_args()

    mod_text = args.mod_rs.read_text(encoding="utf-8")
    rows = extract_dispatch(mod_text)
    payload = {
        "source": str(args.mod_rs).replace("\\", "/"),
        "count": len(rows),
        "syscalls": rows,
    }
    args.out_json.parent.mkdir(parents=True, exist_ok=True)
    args.out_json.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {len(rows)} syscalls to {args.out_json}")

    if args.check_catalog:
        catalog_names = load_catalog_syscalls(args.check_catalog)
        extracted = {r["syscall"] for r in rows}
        missing = [n for n in catalog_names if n not in extracted]
        if missing:
            print("Catalog entries not in extract:", file=sys.stderr)
            for n in missing:
                print(f"  - {n}", file=sys.stderr)
            sys.exit(1)
        print(f"Catalog check OK ({len(catalog_names)} entries).")


if __name__ == "__main__":
    main()
