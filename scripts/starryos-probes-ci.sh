#!/bin/sh
# Static checks for StarryOS syscall probe tooling (no QEMU required).
# See docs/starryos-syscall-commit-strategy.md
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== extract + catalog check =="
python3 scripts/extract_starry_syscalls.py --check-catalog docs/starryos-syscall-catalog.yaml

echo "== probe path coverage =="
python3 scripts/check_probe_coverage.py

echo "== shell syntax =="
for f in "$ROOT/test-suit/starryos/scripts/"*.sh; do
  [ -f "$f" ] || continue
  sh -n "$f"
done
sh -n "$ROOT/scripts/starryos-probes-ci.sh"

echo "== OK: starryos-probes-ci static checks passed =="

if command -v riscv64-linux-musl-gcc >/dev/null 2>&1; then
  echo "== cross build probes =="
  CC=riscv64-linux-musl-gcc "$ROOT/test-suit/starryos/scripts/build-probes.sh"
  echo "== OK: probe build passed =="
else
  echo "SKIP: riscv64-linux-musl-gcc not found (cross build)"
fi
