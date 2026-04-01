#!/usr/bin/sh
set -eu
# Package root: test-suit/starryos
PKG="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${PROBE_OUT:-$PKG/probes/build-riscv64}"
CC="${CC:-riscv64-linux-musl-gcc}"

mkdir -p "$OUT"
if ! command -v "$CC" >/dev/null 2>&1; then
  echo "Missing cross compiler: $CC" >&2
  echo "Install riscv64-linux-musl-gcc (see probes/README.md)" >&2
  exit 1
fi

for src in "$PKG/probes/contract/"*.c; do
  [ -f "$src" ] || continue
  base="$(basename "$src" .c)"
  echo "CC $base"
  "$CC" -static -O2 -fno-stack-protector -o "$OUT/$base" "$src"
done
echo "Built probes -> $OUT"
