#!/usr/bin/sh
set -eu
PKG="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${PROBE_OUT:-$PKG/probes/build-riscv64}"
QEMU_RV64="${QEMU_RV64:-qemu-riscv64}"

usage() {
  echo "Usage: $0 {build|oracle|verify-oracle|verify-oracle-all|help}" >&2
  echo "  build                 run build-probes.sh" >&2
  echo "  oracle [name]         run \$OUT/<name> under \$QEMU_RV64 (default: write_stdout)" >&2
  echo "  verify-oracle [name]  diff stdout vs probes/expected/<name>.line" >&2
  echo "  verify-oracle-all     verify every probes/expected/*.line with matching binary" >&2
  echo "Env: VERIFY_STRICT=1  treat missing \$QEMU_RV64 as failure (exit 2)" >&2
  exit 1
}

verify_one() {
  p="$1"
  exp="$PKG/probes/expected/${p}.line"
  test -f "$exp" || { echo "Missing expected file: $exp" >&2; return 1; }
  test -x "$OUT/$p" || { echo "Missing $OUT/$p — run: $0 build" >&2; return 1; }
  if ! command -v "$QEMU_RV64" >/dev/null 2>&1; then
    if [ "${VERIFY_STRICT:-0}" = 1 ]; then
      echo "STRICT: missing $QEMU_RV64 (set VERIFY_STRICT=0 to allow SKIP)" >&2
      return 2
    fi
    echo "SKIP: $QEMU_RV64 not installed" >&2
    return 0
  fi
  got="$("$QEMU_RV64" "$OUT/$p" 2>/dev/null | tr -d '\r')"
  want="$(cat "$exp")"
  if [ "$got" != "$want" ]; then
    echo "DIFF oracle $p:" >&2
    echo "  want: $want" >&2
    echo "  got:  $got" >&2
    return 1
  fi
  echo "verify-oracle OK: $p -> $want"
  return 0
}

cmd="${1:-help}"
case "$cmd" in
  build)
    exec "$PKG/scripts/build-probes.sh"
    ;;
  oracle)
    p="${2:-write_stdout}"
    test -x "$OUT/$p" || { echo "Missing $OUT/$p — run: $0 build" >&2; exit 1; }
    if ! command -v "$QEMU_RV64" >/dev/null 2>&1; then
      echo "Missing $QEMU_RV64 (install qemu-user / qemu-system user package)" >&2
      exit 1
    fi
    "$QEMU_RV64" "$OUT/$p"
    ;;
  verify-oracle)
    p="${2:-write_stdout}"
    set +e
    verify_one "$p"
    rc=$?
    set -e
    exit "$rc"
    ;;
  verify-oracle-all)
    failed=0
    strict_fail=0
    any=0
    for exp in "$PKG/probes/expected/"*.line; do
      [ -f "$exp" ] || continue
      any=1
      base="$(basename "$exp" .line)"
      set +e
      verify_one "$base"
      rc=$?
      set -e
      if [ "$rc" -eq 2 ]; then
        strict_fail=1
        failed=1
      elif [ "$rc" -ne 0 ]; then
        failed=1
      fi
    done
    if [ "$any" -eq 0 ]; then
      echo "No probes/expected/*.line files" >&2
      exit 1
    fi
    if [ "$strict_fail" -eq 1 ]; then
      exit 2
    fi
    exit "$failed"
    ;;
  help|*)
    usage
    ;;
esac
