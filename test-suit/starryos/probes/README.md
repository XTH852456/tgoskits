# StarryOS syscall probes

Small static `riscv64-linux-musl` ELF programs for Linux-oracle vs StarryOS diff testing.

## Build (host)

Requires `riscv64-linux-musl-gcc` on PATH (or set `CC`):

```sh
export CC=/path/to/riscv64-linux-musl-gcc   # optional
test-suit/starryos/scripts/build-probes.sh
```

Binaries go to `probes/build-riscv64/`.

## Linux oracle (user-mode QEMU)

Needs `qemu-riscv64` (Debian/Ubuntu: `qemu-user`):

```sh
export QEMU_RV64=qemu-riscv64
test-suit/starryos/scripts/run-diff-probes.sh oracle write_stdout
# 校验全部 expected/*.line（需已 build 且 qemu-riscv64 可用）
test-suit/starryos/scripts/run-diff-probes.sh verify-oracle-all
# CI：缺少 qemu-user 时失败退出码 2
VERIFY_STRICT=1 test-suit/starryos/scripts/run-diff-probes.sh verify-oracle-all
```

## Contract probes (hand-written)

| Basename | Syscall | `expected/*.line` |
|----------|---------|-------------------|
| `write_stdout` | write(2) 零长度写 stdout | `expected/write_stdout.line` |
| `close_badfd` | close(2) 非法 fd → EBADF | `expected/close_badfd.line` |
| `read_stdin_zero` | read(2) stdin count=0 → 0 | `expected/read_stdin_zero.line` |
| `dup_badfd` | dup(2) 非法 fd → EBADF | `expected/dup_badfd.line` |
| `fcntl_badfd` | fcntl(2) 非法 fd + F_GETFD → EBADF | `expected/fcntl_badfd.line` |

列出当前 contract 名称：`test-suit/starryos/scripts/list-contract-probes.sh`

## Catalog and extract

```sh
python3 scripts/extract_starry_syscalls.py --out-json docs/starryos-syscall-dispatch.json
python3 scripts/extract_starry_syscalls.py --check-catalog docs/starryos-syscall-catalog.yaml
python3 scripts/gen_syscall_probes.py --catalog docs/starryos-syscall-catalog.yaml
```

## Output format

One line per case, machine-parseable, e.g.:

`CASE write_stdout.zero_len ret=0 errno=0 note=handwritten`

## StarryOS side

Copy the static ELF into the guest rootfs (e.g. with `debugfs` on `rootfs-riscv64.img`) and run it from `shell_init_cmd` or an init script; compare stdout with the Linux oracle line above.

## StarryOS QEMU 回归（S0-5）

1. 准备带 probe 的磁盘镜像：

   ```sh
   ./test-suit/starryos/scripts/prepare-rootfs-with-write_stdout-probe.sh
   ```

2. 运行 `starryos-test`（使用 `--test-disk-image` 指向注入后的镜像）：

   ```sh
   cargo xtask starry test qemu --target riscv64 \
     --test-disk-image target/riscv64gc-unknown-none-elf/rootfs-riscv64-probe.img \
     --shell-init-cmd test-suit/starryos/testcases/probe-write_stdout-0 \
     --timeout 120
   ```

说明：`xtask` 仍会把该镜像再复制一份为临时测试盘，不会改写 `rootfs-riscv64-probe.img`。

### 其它探针（通用注入）

```sh
./test-suit/starryos/scripts/prepare-rootfs-with-probe.sh close_badfd
cargo xtask starry test qemu --target riscv64 \
  --test-disk-image target/riscv64gc-unknown-none-elf/rootfs-riscv64-probe-close_badfd.img \
  --shell-init-cmd test-suit/starryos/testcases/probe-close_badfd-0 \
  --timeout 120
```

## 一键 QEMU（starryos-test）

在仓库根目录（需已 `cargo xtask starry rootfs --arch riscv64`）：

```sh
./test-suit/starryos/scripts/run-starry-probe-qemu.sh read_stdin_zero
```

## 串口行与 oracle 比对

从日志中提取含 `CASE` 的一行后：

```sh
test-suit/starryos/scripts/diff-guest-line.sh read_stdin_zero 'CASE read_stdin_zero.zero_count ret=0 errno=0 note=handwritten'
```

## Catalog 与文件一致性

```sh
python3 scripts/check_probe_coverage.py
./scripts/starryos-probes-ci.sh
```

## 从日志抽取 `CASE` 行

```sh
test-suit/starryos/scripts/extract-case-line.sh guest-serial.log
# 或管道：... | test-suit/starryos/scripts/extract-case-line.sh | xargs -I{} test-suit/starryos/scripts/diff-guest-line.sh write_stdout {}
```

## 相关文档

- `docs/starryos-syscall-testing-method.md` — 分层与新增 contract 清单
- `docs/starryos-syscall-compat-matrix.yaml` — Linux 对齐矩阵骨架
- `docs/starryos-syscall-smp-notes.md` — SMP / 多核占位说明
- `docs/starryos-syscall-progress-rounds.md` — 多轮迭代纪要
- `docs/starryos-syscall-commit-strategy.md` — 分组提交建议
- `docs/starryos-probes-ci-example.md` — CI / oracle job 示例片段
