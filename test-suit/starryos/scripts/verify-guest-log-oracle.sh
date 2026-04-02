#!/usr/bin/sh
# From a QEMU/serial log (file or stdin), take the first line matching ^CASE  and
# compare it to probes/expected/<probe>.line (same as Linux oracle).
#
# Usage:
#   ./verify-guest-log-oracle.sh <probe_basename> [log_file|-]
#   ./verify-guest-log-oracle.sh write_stdout              # stdin（可粘贴串口输出，结束输入：Ctrl+D）
#   ./verify-guest-log-oracle.sh write_stdout serial.log   # 文件（需事先存在，例如 tee 保存）
#   ./verify-guest-log-oracle.sh write_stdout -            # 显式 stdin
#
# Exit: 0 match, 1 mismatch, 2 no CASE line found in input
set -eu
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

probe="${1:?usage: $0 <probe_basename> [log_file|-]}"
shift

if [ "$#" -ge 1 ]; then
  log_arg="$1"
else
  log_arg="-"
fi

if [ "$log_arg" = "-" ]; then
  line="$("$SCRIPT_DIR/extract-case-line.sh")"
else
  if [ ! -f "$log_arg" ]; then
    echo "verify-guest-log-oracle: 找不到日志文件: $log_arg" >&2
    echo "" >&2
    echo "可选做法：" >&2
    echo "  1) 在仓库根目录先准备镜像（按需），再跑 QEMU 并 tee 保存输出，例如 write_stdout：" >&2
    echo "       cargo xtask starry rootfs --arch riscv64" >&2
    echo "       ./test-suit/starryos/scripts/prepare-rootfs-with-write_stdout-probe.sh" >&2
    echo "       cargo xtask starry test qemu --target riscv64 \\" >&2
    echo "         --test-disk-image target/riscv64gc-unknown-none-elf/rootfs-riscv64-probe.img \\" >&2
    echo "         --shell-init-cmd test-suit/starryos/testcases/probe-write_stdout-0 \\" >&2
    echo "         --timeout 120 \\" >&2
    echo "         2>&1 | tee serial.log" >&2
    echo "       $0 $probe serial.log" >&2
    echo "  2) 不传文件，从标准输入读入（粘贴整段串口文本后按 Ctrl+D 结束）：" >&2
    echo "       $0 $probe" >&2
    echo "       $0 $probe -" >&2
    exit 2
  fi
  line="$("$SCRIPT_DIR/extract-case-line.sh" "$log_arg")"
fi

if [ -z "$line" ]; then
  echo "verify-guest-log-oracle: no line matching ^CASE  (probe=$probe)" >&2
  exit 2
fi

exec "$SCRIPT_DIR/diff-guest-line.sh" "$probe" "$line"
