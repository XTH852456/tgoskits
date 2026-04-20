# Systemd 支持计划

## Context

StarryOS 当前使用 busybox init 启动用户态（通过 `/bin/sh -c init.sh`）。目标是支持最新版 systemd，以运行标准 Debian 用户态环境。

### 核心问题：systemd `chase()` crash

systemd 启动时调用链：
```
sd_device_new_from_devname()
  → device_new_from_mode_and_devnum()
    → device_set_syspath()
      → chase()   ← crash: path_is_absolute assertion
```

原因：`/sys` 是空的 tmpfs，systemd 找不到 `/sys/dev/block/{major}:{minor}/uevent` 和 `/sys/dev/char/{major}:{minor}/uevent`（相关 issue: systemd/systemd#36242）。

### 已解决的问题

| 问题 | 原因 | 解决 |
|------|------|------|
| PID 1 | alarm task 消耗了 TID 1-6 | `entry.rs` 中硬编码 PID=1，`spawn_alarm_task()` 移到 init 之后 |
| set_session panic | systemd 重设 TTY session | `job.rs` 中 assert 改为 warn |
| lwext4 ENOTSUP | trixie 默认启用 orphan_file / metadata_csum_seed | mkfs.ext4 加 `-O ^orphan_file,^metadata_csum_seed` |

## 方案：实现最小 sysfs

新建 `os/StarryOS/kernel/src/pseudofs/sys.rs`，用 `SimpleFs` + `DirMapping` 模式构建只读 sysfs。

### 目录结构

```
/sys/
├── dev/
│   ├── block/
│   │   └── {major}:{minor}/
│   │       └── uevent    — "MAJOR=X\nMINOR=Y\nDEVNAME=loop0\nDEVTYPE=disk\n"
│   └── char/
│       └── {major}:{minor}/
│           └── uevent    — "MAJOR=X\nMINOR=Y\nDEVNAME=null\n"
├── class/
│   └── graphics/
│       └── fb0/
│           └── device/
│               └── subsystem -> ../../../../../../../class/graphics
├── block/
│   └── loop0 -> ../dev/block/7:0
├── devices/system/cpu/
│   └── online            — "0\n"
├── fs/cgroup/            (空目录)
└── subsystem/            (空目录)
```

### 核心实现

1. **全局设备注册表**：`DEVICES: spin::Mutex<Vec<SysDevice>>`
   - devfs 注册设备时同步调用 `sys::register_device()`
   - sysfs builder 根据注册表动态创建 `/sys/dev/block/` 和 `/sys/dev/char/` 目录

2. **`SysDevTypeDir`**：动态 `SimpleDirOps` 实现，遍历注册表过滤 BlockDevice/CharacterDevice

3. **`SysDevEntryDir`**：每个 `{major}:{minor}/` 目录，包含 `uevent` 文件

4. **`SysBlockSymlinkDir`**：`/sys/block/` 下的符号链接（loop0 -> ../dev/block/7:0）

### 修改的文件

| 文件 | 操作 |
|------|------|
| `os/StarryOS/kernel/src/pseudofs/sys.rs` | **新建** — sysfs 实现 |
| `os/StarryOS/kernel/src/pseudofs/mod.rs` | **修改** — `pub mod sys` + 用 `sys::new_sysfs()` 替换 tmpfs |
| `os/StarryOS/kernel/src/pseudofs/dev/mod.rs` | **修改** — 每个设备注册时调用 `sys::register_device()` |
| `scripts/build-debian-rootfs.sh` | **修改** — 增加 `--init systemd` 选项 |
| `os/StarryOS/kernel/src/entry.rs` | **修改** — PID=1 硬编码 + alarm task 后移 |

### 复用的基础设施

- `SimpleFs`, `SimpleFsNode`（`pseudofs/fs.rs`）
- `DirMapping`, `SimpleDir`, `SimpleDirOps`（`pseudofs/dir.rs`）
- `SimpleFile`（`pseudofs/file.rs`）
- `DeviceId::major()` / `DeviceId::minor()`（`axfs_ng_vfs::DeviceId`）

## Rootfs 构建

```bash
# busybox init（默认）
bash scripts/build-debian-rootfs.sh --arch aarch64

# systemd init
bash scripts/build-debian-rootfs.sh --arch aarch64 --init systemd
```

systemd 模式的额外配置：
- 安装 systemd 包
- `/sbin/init` → `/lib/systemd/systemd`
- 创建 `/etc/machine-id`（空文件）
- 禁用不兼容的 units（networkd, resolved, journald, udevd 等）
- 镜像大小自动增大到 4G
- mkfs.ext4 禁用 `orphan_file` 和 `metadata_csum_seed`

## 迭代调试流程

启动 systemd 后预期卡死点：

| 卡死阶段 | 可能原因 | 排查文件 |
|---------|---------|---------|
| `/sbin/init` 加载 | ELF 加载失败 | `kernel/src/syscall/task/execve.rs` |
| systemd 早期初始化 | 缺失 syscall | `kernel/src/syscall/mod.rs` |
| 挂载文件系统 | mount 不支持 fs 类型 | `kernel/src/syscall/fs/mod.rs` |
| D-Bus / socket 通信 | unix socket 问题 | `kernel/src/syscall/ipc/` |
| 信号处理 | signal 机制不完整 | `kernel/src/syscall/signal/` |
| cgroup | 不支持 cgroupfs | mount 相关 |

### 测试命令

```bash
# 构建 systemd rootfs
bash scripts/build-debian-rootfs.sh --arch aarch64 --init systemd

# 修改 CMDLINE 为 ["/sbin/init"] 后启动
cargo starry qemu --arch aarch64

# 带详细日志
AX_LOG=debug cargo starry qemu --arch aarch64
```

### 成功标志

看到 systemd 启动日志（如 `systemd[1]: Started ...`），进入 shell。
