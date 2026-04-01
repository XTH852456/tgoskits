#!/usr/bin/sh
# Build probe rootfs + run starryos-test in QEMU (convenience wrapper).
# Prereq: cargo xtask starry rootfs --arch riscv64, riscv64-linux-musl-gcc, e2fsprogs.
# Usage: ./run-starry-probe-qemu.sh <probe_basename> [extra cargo xtask args...]
set -eu
WS="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$WS"

probe="${1:?usage: $0 <probe_basename e.g. write_stdout or read_stdin_zero>}"
shift

if [ "$probe" = write_stdout ]; then
  IMG="$WS/target/riscv64gc-unknown-none-elf/rootfs-riscv64-probe.img"
else
  IMG="$WS/target/riscv64gc-unknown-none-elf/rootfs-riscv64-probe-${probe}.img"
fi

"$WS/test-suit/starryos/scripts/prepare-rootfs-with-probe.sh" "$probe"

exec cargo xtask starry test qemu --target riscv64 \
  --test-disk-image "$IMG" \
  --shell-init-cmd "$WS/test-suit/starryos/testcases/probe-${probe}-0" \
  --timeout 120 \
  "$@"
