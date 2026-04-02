# StarryOS 系统调用分发表（机器生成）

数据源：[docs/starryos-syscall-dispatch.json](docs/starryos-syscall-dispatch.json)（`python3 scripts/extract_starry_syscalls.py --out-json ...`）。 表示 `handle_syscall` 中**已挂接**的 `Sysno`；`cfgs` 非空时仅在对应 **target/feature** 下参与编译。

**条目数**: 210

| # | syscall | section | cfgs |
|---|---------|---------|------|
| 1 | `ioctl` | fs ctl | — |
| 2 | `chdir` | fs ctl | — |
| 3 | `fchdir` | fs ctl | — |
| 4 | `chroot` | fs ctl | — |
| 5 | `mkdir` | fs ctl | #[cfg(target_arch = "x86_64")] |
| 6 | `mkdirat` | fs ctl | — |
| 7 | `getdents64` | fs ctl | — |
| 8 | `link` | fs ctl | #[cfg(target_arch = "x86_64")] |
| 9 | `linkat` | fs ctl | — |
| 10 | `rmdir` | fs ctl | #[cfg(target_arch = "x86_64")] |
| 11 | `unlink` | fs ctl | #[cfg(target_arch = "x86_64")] |
| 12 | `unlinkat` | fs ctl | — |
| 13 | `getcwd` | fs ctl | — |
| 14 | `symlink` | fs ctl | #[cfg(target_arch = "x86_64")] |
| 15 | `symlinkat` | fs ctl | — |
| 16 | `rename` | fs ctl | #[cfg(target_arch = "x86_64")] |
| 17 | `renameat` | fs ctl | #[cfg(not(target_arch = "riscv64"))] |
| 18 | `renameat2` | fs ctl | — |
| 19 | `sync` | fs ctl | — |
| 20 | `syncfs` | fs ctl | — |
| 21 | `chown` | file ops | #[cfg(target_arch = "x86_64")] |
| 22 | `lchown` | file ops | #[cfg(target_arch = "x86_64")] |
| 23 | `fchown` | file ops | — |
| 24 | `fchownat` | file ops | — |
| 25 | `chmod` | file ops | #[cfg(target_arch = "x86_64")] |
| 26 | `fchmod` | file ops | — |
| 27 | `fchmodat` | file ops | — |
| 28 | `fchmodat2` | file ops | — |
| 29 | `readlink` | file ops | #[cfg(target_arch = "x86_64")] |
| 30 | `readlinkat` | file ops | — |
| 31 | `utime` | file ops | #[cfg(target_arch = "x86_64")] |
| 32 | `utimes` | file ops | #[cfg(target_arch = "x86_64")] |
| 33 | `utimensat` | file ops | — |
| 34 | `open` | fd ops | #[cfg(target_arch = "x86_64")] |
| 35 | `openat` | fd ops | — |
| 36 | `close` | fd ops | — |
| 37 | `close_range` | fd ops | — |
| 38 | `dup` | fd ops | — |
| 39 | `dup2` | fd ops | #[cfg(target_arch = "x86_64")] |
| 40 | `dup3` | fd ops | — |
| 41 | `fcntl` | fd ops | — |
| 42 | `flock` | fd ops | — |
| 43 | `read` | io | — |
| 44 | `readv` | io | — |
| 45 | `write` | io | — |
| 46 | `writev` | io | — |
| 47 | `lseek` | io | — |
| 48 | `truncate` | io | — |
| 49 | `ftruncate` | io | — |
| 50 | `fallocate` | io | — |
| 51 | `fsync` | io | — |
| 52 | `fdatasync` | io | — |
| 53 | `fadvise64` | io | — |
| 54 | `pread64` | io | — |
| 55 | `pwrite64` | io | — |
| 56 | `preadv` | io | — |
| 57 | `pwritev` | io | — |
| 58 | `preadv2` | io | — |
| 59 | `pwritev2` | io | — |
| 60 | `sendfile` | io | — |
| 61 | `copy_file_range` | io | — |
| 62 | `splice` | io | — |
| 63 | `poll` | io mpx | #[cfg(target_arch = "x86_64")] |
| 64 | `ppoll` | io mpx | — |
| 65 | `select` | io mpx | #[cfg(target_arch = "x86_64")] |
| 66 | `pselect6` | io mpx | — |
| 67 | `epoll_create1` | io mpx | — |
| 68 | `epoll_ctl` | io mpx | — |
| 69 | `epoll_pwait` | io mpx | — |
| 70 | `epoll_pwait2` | io mpx | — |
| 71 | `mount` | fs mount | — |
| 72 | `umount2` | fs mount | — |
| 73 | `pipe2` | pipe | — |
| 74 | `pipe` | pipe | #[cfg(target_arch = "x86_64")] |
| 75 | `eventfd2` | event | — |
| 76 | `pidfd_open` | pidfd | — |
| 77 | `pidfd_getfd` | pidfd | — |
| 78 | `pidfd_send_signal` | pidfd | — |
| 79 | `memfd_create` | memfd | — |
| 80 | `stat` | fs stat | #[cfg(target_arch = "x86_64")] |
| 81 | `fstat` | fs stat | — |
| 82 | `lstat` | fs stat | #[cfg(target_arch = "x86_64")] |
| 83 | `newfstatat` | fs stat | #[cfg(any(target_arch = "x86_64", target_arch = "riscv64"))] |
| 84 | `fstatat` | fs stat | #[cfg(not(any(target_arch = "x86_64", target_arch = "riscv64")))] |
| 85 | `statx` | fs stat | — |
| 86 | `access` | fs stat | #[cfg(target_arch = "x86_64")] |
| 87 | `faccessat` | fs stat | — |
| 88 | `faccessat2` | fs stat | — |
| 89 | `statfs` | fs stat | — |
| 90 | `fstatfs` | fs stat | — |
| 91 | `brk` | mm | — |
| 92 | `mmap` | mm | — |
| 93 | `munmap` | mm | — |
| 94 | `mprotect` | mm | — |
| 95 | `mincore` | mm | — |
| 96 | `mremap` | mm | — |
| 97 | `madvise` | mm | — |
| 98 | `msync` | mm | — |
| 99 | `mlock` | mm | — |
| 100 | `mlock2` | mm | — |
| 101 | `getpid` | task info | — |
| 102 | `getppid` | task info | — |
| 103 | `gettid` | task info | — |
| 104 | `getrusage` | task info | — |
| 105 | `sched_yield` | task sched | — |
| 106 | `nanosleep` | task sched | — |
| 107 | `clock_nanosleep` | task sched | — |
| 108 | `sched_getaffinity` | task sched | — |
| 109 | `sched_setaffinity` | task sched | — |
| 110 | `sched_getscheduler` | task sched | — |
| 111 | `sched_setscheduler` | task sched | — |
| 112 | `sched_getparam` | task sched | — |
| 113 | `getpriority` | task sched | — |
| 114 | `execve` | task ops | — |
| 115 | `set_tid_address` | task ops | — |
| 116 | `arch_prctl` | task ops | #[cfg(target_arch = "x86_64")] |
| 117 | `prctl` | task ops | — |
| 118 | `prlimit64` | task ops | — |
| 119 | `capget` | task ops | — |
| 120 | `capset` | task ops | — |
| 121 | `umask` | task ops | — |
| 122 | `setreuid` | task ops | — |
| 123 | `setresuid` | task ops | — |
| 124 | `setresgid` | task ops | — |
| 125 | `get_mempolicy` | task ops | — |
| 126 | `clone` | task management | — |
| 127 | `clone3` | task management | — |
| 128 | `fork` | task management | #[cfg(target_arch = "x86_64")] |
| 129 | `exit` | task management | — |
| 130 | `exit_group` | task management | — |
| 131 | `wait4` | task management | — |
| 132 | `getsid` | task management | — |
| 133 | `setsid` | task management | — |
| 134 | `getpgid` | task management | — |
| 135 | `setpgid` | task management | — |
| 136 | `rt_sigprocmask` | signal | — |
| 137 | `rt_sigaction` | signal | — |
| 138 | `rt_sigpending` | signal | — |
| 139 | `rt_sigreturn` | signal | — |
| 140 | `rt_sigtimedwait` | signal | — |
| 141 | `rt_sigsuspend` | signal | — |
| 142 | `kill` | signal | — |
| 143 | `tkill` | signal | — |
| 144 | `tgkill` | signal | — |
| 145 | `rt_sigqueueinfo` | signal | — |
| 146 | `rt_tgsigqueueinfo` | signal | — |
| 147 | `sigaltstack` | signal | — |
| 148 | `futex` | signal | — |
| 149 | `get_robust_list` | signal | — |
| 150 | `set_robust_list` | signal | — |
| 151 | `getuid` | sys | — |
| 152 | `geteuid` | sys | — |
| 153 | `getgid` | sys | — |
| 154 | `getegid` | sys | — |
| 155 | `setuid` | sys | — |
| 156 | `setgid` | sys | — |
| 157 | `getgroups` | sys | — |
| 158 | `setgroups` | sys | — |
| 159 | `uname` | sys | — |
| 160 | `sysinfo` | sys | — |
| 161 | `syslog` | sys | — |
| 162 | `getrandom` | sys | — |
| 163 | `seccomp` | sys | — |
| 164 | `riscv_flush_icache` | sys | #[cfg(target_arch = "riscv64")] |
| 165 | `membarrier` | sync | — |
| 166 | `gettimeofday` | time | — |
| 167 | `times` | time | — |
| 168 | `clock_gettime` | time | — |
| 169 | `clock_getres` | time | — |
| 170 | `getitimer` | time | — |
| 171 | `setitimer` | time | — |
| 172 | `msgget` | msg | — |
| 173 | `msgsnd` | msg | — |
| 174 | `msgrcv` | msg | — |
| 175 | `msgctl` | msg | — |
| 176 | `shmget` | shm | — |
| 177 | `shmat` | shm | — |
| 178 | `shmctl` | shm | — |
| 179 | `shmdt` | shm | — |
| 180 | `socket` | net | — |
| 181 | `socketpair` | net | — |
| 182 | `bind` | net | — |
| 183 | `connect` | net | — |
| 184 | `getsockname` | net | — |
| 185 | `getpeername` | net | — |
| 186 | `listen` | net | — |
| 187 | `accept` | net | — |
| 188 | `accept4` | net | — |
| 189 | `shutdown` | net | — |
| 190 | `sendto` | net | — |
| 191 | `recvfrom` | net | — |
| 192 | `sendmsg` | net | — |
| 193 | `recvmsg` | net | — |
| 194 | `getsockopt` | net | — |
| 195 | `setsockopt` | net | — |
| 196 | `signalfd4` | signal file descriptors | — |
| 197 | `timerfd_create` | dummy fds | — |
| 198 | `fanotify_init` | dummy fds | — |
| 199 | `inotify_init1` | dummy fds | — |
| 200 | `userfaultfd` | dummy fds | — |
| 201 | `perf_event_open` | dummy fds | — |
| 202 | `io_uring_setup` | dummy fds | — |
| 203 | `bpf` | dummy fds | — |
| 204 | `fsopen` | dummy fds | — |
| 205 | `fspick` | dummy fds | — |
| 206 | `open_tree` | dummy fds | — |
| 207 | `memfd_secret` | dummy fds | — |
| 208 | `timer_create` | dummy fds | — |
| 209 | `timer_gettime` | dummy fds | — |
| 210 | `timer_settime` | dummy fds | — |
