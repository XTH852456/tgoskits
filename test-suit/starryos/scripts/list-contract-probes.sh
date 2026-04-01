#!/usr/bin/sh
# List contract probe basenames (stem of probes/contract/*.c).
set -eu
PKG="$(cd "$(dirname "$0")/.." && pwd)"
for src in "$PKG/probes/contract/"*.c; do
  [ -f "$src" ] || continue
  basename "$src" .c
done | sort
