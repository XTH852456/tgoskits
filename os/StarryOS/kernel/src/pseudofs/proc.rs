use alloc::{
    borrow::Cow,
    boxed::Box,
    format,
    string::{String, ToString},
    sync::{Arc, Weak},
    vec,
    vec::Vec,
};
use core::{
    ffi::CStr,
    iter,
    sync::atomic::{AtomicUsize, Ordering},
};

use ax_task::{AxTaskRef, WeakAxTaskRef, current};
use axfs_ng_vfs::{Filesystem, NodeType, VfsError, VfsResult};
use indoc::indoc;
use starry_process::Process;

use crate::{
    file::{PidFd, FD_TABLE},
    pseudofs::{
        DirMaker, DirMapping, NodeOpsMux, RwFile, SimpleDir, SimpleDirOps, SimpleFile,
        SimpleFileOperation, SimpleFs,
    },
    task::{AsThread, TaskStat, get_process_data, get_task, processes, tasks},
};

const DUMMY_MEMINFO: &str = indoc! {"
    MemTotal:       32536204 kB
    MemFree:         5506524 kB
    MemAvailable:   18768344 kB
    Buffers:            3264 kB
    Cached:         14454588 kB
    SwapCached:            0 kB
    Active:         18229700 kB
    Inactive:        6540624 kB
    Active(anon):   11380224 kB
    Inactive(anon):        0 kB
    Active(file):    6849476 kB
    Inactive(file):  6540624 kB
    Unevictable:      930088 kB
    Mlocked:            1136 kB
    SwapTotal:       4194300 kB
    SwapFree:        4194300 kB
    Zswap:                 0 kB
    Zswapped:              0 kB
    Dirty:             47952 kB
    Writeback:             0 kB
    AnonPages:      10992512 kB
    Mapped:          1361184 kB
    Shmem:           1068056 kB
    KReclaimable:     341440 kB
    Slab:             628996 kB
    SReclaimable:     341440 kB
    SUnreclaim:       287556 kB
    KernelStack:       28704 kB
    PageTables:        85308 kB
    SecPageTables:      2084 kB
    NFS_Unstable:          0 kB
    Bounce:                0 kB
    WritebackTmp:          0 kB
    CommitLimit:    20462400 kB
    Committed_AS:   45105316 kB
    VmallocTotal:   34359738367 kB
    VmallocUsed:      205924 kB
    VmallocChunk:          0 kB
    Percpu:            23840 kB
    HardwareCorrupted:     0 kB
    AnonHugePages:   1417216 kB
    ShmemHugePages:        0 kB
    ShmemPmdMapped:        0 kB
    FileHugePages:    477184 kB
    FilePmdMapped:    288768 kB
    CmaTotal:              0 kB
    CmaFree:               0 kB
    Unaccepted:            0 kB
    HugePages_Total:       0
    HugePages_Free:        0
    HugePages_Rsvd:        0
    HugePages_Surp:        0
    Hugepagesize:       2048 kB
    Hugetlb:               0 kB
    DirectMap4k:     1739900 kB
    DirectMap2M:    31492096 kB
    DirectMap1G:     1048576 kB
"};

pub fn new_procfs() -> Filesystem {
    SimpleFs::new_with("proc".into(), 0x9fa0, builder)
}

struct ProcessTaskDir {
    fs: Arc<SimpleFs>,
    process: Weak<Process>,
}

impl SimpleDirOps for ProcessTaskDir {
    fn child_names<'a>(&'a self) -> Box<dyn Iterator<Item = Cow<'a, str>> + 'a> {
        let Some(process) = self.process.upgrade() else {
            return Box::new(iter::empty());
        };
        Box::new(
            process
                .threads()
                .into_iter()
                .map(|tid| tid.to_string().into()),
        )
    }

    fn lookup_child(&self, name: &str) -> VfsResult<NodeOpsMux> {
        let process = self.process.upgrade().ok_or(VfsError::NotFound)?;
        let tid = name.parse::<u32>().map_err(|_| VfsError::NotFound)?;
        let task = get_task(tid).map_err(|_| VfsError::NotFound)?;
        if task.as_thread().proc_data.proc.pid() != process.pid() {
            return Err(VfsError::NotFound);
        }

        Ok(NodeOpsMux::Dir(SimpleDir::new_maker(
            self.fs.clone(),
            Arc::new(ThreadDir {
                fs: self.fs.clone(),
                task: Arc::downgrade(&task),
            }),
        )))
    }

    fn is_cacheable(&self) -> bool {
        false
    }
}

#[rustfmt::skip]
fn task_status(task: &AxTaskRef) -> String {
    let proc = &task.as_thread().proc_data.proc;
    let ppid = proc.parent().map(|p| p.pid()).unwrap_or(0);
    format!(
        "Name:\t{}\n\
        Tgid:\t{}\n\
        Pid:\t{}\n\
        PPid:\t{ppid}\n\
        Uid:\t0 0 0 0\n\
        Gid:\t0 0 0 0\n\
        Cpus_allowed:\t1\n\
        Cpus_allowed_list:\t0\n\
        Mems_allowed:\t1\n\
        Mems_allowed_list:\t0",
        task.id_name(),
        proc.pid(),
        task.id().as_u64()
    )
}

/// The /proc/[pid]/fd directory
struct ThreadFdDir {
    fs: Arc<SimpleFs>,
    task: WeakAxTaskRef,
}

/// The /proc/[pid]/fdinfo directory
struct ThreadFdInfoDir {
    fs: Arc<SimpleFs>,
    task: WeakAxTaskRef,
}

impl SimpleDirOps for ThreadFdInfoDir {
    fn child_names<'a>(&'a self) -> Box<dyn Iterator<Item = Cow<'a, str>> + 'a> {
        let Some(task) = self.task.upgrade() else {
            return Box::new(iter::empty());
        };
        let ids = FD_TABLE
            .scope(&task.as_thread().proc_data.scope.read())
            .read()
            .ids()
            .map(|id| Cow::Owned(id.to_string()))
            .collect::<Vec<_>>();
        Box::new(ids.into_iter())
    }

    fn lookup_child(&self, name: &str) -> VfsResult<NodeOpsMux> {
        let fs = self.fs.clone();
        let task = self.task.upgrade().ok_or(VfsError::NotFound)?;
        let fd = name.parse::<u32>().map_err(|_| VfsError::NotFound)?;
        let file_obj = FD_TABLE
            .scope(&task.as_thread().proc_data.scope.read())
            .read()
            .get(fd as usize)
            .ok_or(VfsError::NotFound)?
            .inner
            .clone();

        // For pidfd, extract the target PID
        let pidfd_pid = file_obj.as_ref().downcast_ref::<PidFd>()
            .and_then(|pidfd| pidfd.process_data().ok())
            .map(|pd| pd.proc.pid());

        Ok(SimpleFile::new_regular(fs, move || {
            if let Some(target_pid) = pidfd_pid {
                Ok(format!("Pid:\t{target_pid}\nNSpid:\t{target_pid}\n").into_bytes())
            } else {
                Ok("".into())
            }
        })
        .into())
    }

    fn is_cacheable(&self) -> bool {
        false
    }
}

impl SimpleDirOps for ThreadFdDir {
    fn child_names<'a>(&'a self) -> Box<dyn Iterator<Item = Cow<'a, str>> + 'a> {
        let Some(task) = self.task.upgrade() else {
            return Box::new(iter::empty());
        };
        let ids = FD_TABLE
            .scope(&task.as_thread().proc_data.scope.read())
            .read()
            .ids()
            .map(|id| Cow::Owned(id.to_string()))
            .collect::<Vec<_>>();
        Box::new(ids.into_iter())
    }

    fn lookup_child(&self, name: &str) -> VfsResult<NodeOpsMux> {
        let fs = self.fs.clone();
        let task = self.task.upgrade().ok_or(VfsError::NotFound)?;
        let fd = name.parse::<u32>().map_err(|_| VfsError::NotFound)?;
        let path = FD_TABLE
            .scope(&task.as_thread().proc_data.scope.read())
            .read()
            .get(fd as _)
            .ok_or(VfsError::NotFound)?
            .inner
            .path()
            .into_owned();
        Ok(SimpleFile::new(fs, NodeType::Symlink, move || Ok(path.clone())).into())
    }

    fn is_cacheable(&self) -> bool {
        false
    }
}

/// The /proc/[pid] directory
struct ThreadDir {
    fs: Arc<SimpleFs>,
    task: WeakAxTaskRef,
}

impl SimpleDirOps for ThreadDir {
    fn child_names<'a>(&'a self) -> Box<dyn Iterator<Item = Cow<'a, str>> + 'a> {
        Box::new(
            [
                "stat",
                "status",
                "oom_score_adj",
                "task",
                "maps",
                "mounts",
                "mountinfo",
                "cmdline",
                "comm",
                "exe",
                "fd",
                "fdinfo",
                "cgroup",
            ]
            .into_iter()
            .map(Cow::Borrowed),
        )
    }

    fn lookup_child(&self, name: &str) -> VfsResult<NodeOpsMux> {
        let fs = self.fs.clone();
        let task = self.task.upgrade().ok_or(VfsError::NotFound)?;
        Ok(match name {
            "stat" => SimpleFile::new_regular(fs, move || {
                let result = TaskStat::from_thread(&task);
                if let Err(ref e) = result {
                    warn!("procfs: /proc/{}/stat read failed: {:?}", task.as_thread().proc_data.proc.pid(), e);
                }
                Ok(format!("{}", result?).into_bytes())
            })
            .into(),
            "status" => SimpleFile::new_regular(fs, move || Ok(task_status(&task))).into(),
            "oom_score_adj" => SimpleFile::new_regular(
                fs,
                RwFile::new(move |req| match req {
                    SimpleFileOperation::Read => Ok(Some(
                        task.as_thread().oom_score_adj().to_string().into_bytes(),
                    )),
                    SimpleFileOperation::Write(data) => {
                        if !data.is_empty() {
                            let value = str::from_utf8(data)
                                .ok()
                                .and_then(|it| it.parse::<i32>().ok())
                                .ok_or(VfsError::InvalidInput)?;
                            task.as_thread().set_oom_score_adj(value);
                        }
                        Ok(None)
                    }
                }),
            )
            .into(),
            "task" => SimpleDir::new_maker(
                fs.clone(),
                Arc::new(ProcessTaskDir {
                    fs,
                    process: Arc::downgrade(&task.as_thread().proc_data.proc),
                }),
            )
            .into(),
            "maps" => SimpleFile::new_regular(fs, move || {
                Ok(indoc! {"
                    7f000000-7f001000 r--p 00000000 00:00 0          [vdso]
                    7f001000-7f003000 r-xp 00001000 00:00 0          [vdso]
                    7f003000-7f005000 r--p 00003000 00:00 0          [vdso]
                    7f005000-7f007000 rw-p 00005000 00:00 0          [vdso]
                "})
            })
            .into(),
            "mounts" => SimpleFile::new_regular(fs, move || {
                Ok("proc /proc proc rw,nosuid,nodev,noexec,relatime 0 0\n")
            })
            .into(),
            "mountinfo" => SimpleFile::new_regular(fs, move || {
                // Linux mountinfo format:
                // mount_id parent_id major:minor root mount_point mount_options - fs_type mount_source super_options
                Ok(String::from(
                    "1 0 254:0 / / rw,relatime - ext4 /dev/vda rw\n\
                     2 1 0:3 / /proc rw,nosuid,nodev,noexec,relatime - proc proc rw\n\
                     3 1 0:6 / /sys rw,nosuid,nodev,noexec,relatime - sysfs sysfs rw\n\
                     4 1 0:5 / /dev rw,nosuid,relatime - devtmpfs devtmpfs rw\n",
                ))
            })
            .into(),
            "cmdline" => SimpleFile::new_regular(fs, move || {
                let cmdline = task.as_thread().proc_data.cmdline.read();
                let mut buf = Vec::new();
                for arg in cmdline.iter() {
                    buf.extend_from_slice(arg.as_bytes());
                    buf.push(0);
                }
                Ok(buf)
            })
            .into(),
            "comm" => SimpleFile::new_regular(
                fs,
                RwFile::new(move |req| match req {
                    SimpleFileOperation::Read => {
                        let mut bytes = vec![0; 16];
                        let name = task.name();
                        let copy_len = name.len().min(15);
                        bytes[..copy_len].copy_from_slice(&name.as_bytes()[..copy_len]);
                        bytes[copy_len] = b'\n';
                        Ok(Some(bytes))
                    }
                    SimpleFileOperation::Write(data) => {
                        if !data.is_empty() {
                            let mut input = [0; 16];
                            let copy_len = data.len().min(15);
                            input[..copy_len].copy_from_slice(&data[..copy_len]);
                            task.set_name(
                                CStr::from_bytes_until_nul(&input)
                                    .map_err(|_| VfsError::InvalidInput)?
                                    .to_str()
                                    .map_err(|_| VfsError::InvalidInput)?,
                            );
                        }
                        Ok(None)
                    }
                }),
            )
            .into(),
            "exe" => SimpleFile::new(fs, NodeType::Symlink, move || {
                Ok(task.as_thread().proc_data.exe_path.read().clone())
            })
            .into(),
            "fd" => SimpleDir::new_maker(
                fs.clone(),
                Arc::new(ThreadFdDir {
                    fs,
                    task: Arc::downgrade(&task),
                }),
            )
            .into(),
            "fdinfo" => SimpleDir::new_maker(
                fs.clone(),
                Arc::new(ThreadFdInfoDir {
                    fs,
                    task: Arc::downgrade(&task),
                }),
            )
            .into(),
            "cgroup" => SimpleFile::new_regular(fs, move || {
                Ok("0::/\n")
            })
            .into(),
            _ => return Err(VfsError::NotFound),
        })
    }

    fn is_cacheable(&self) -> bool {
        false
    }
}

/// Minimal directory for zombie/exited processes that no longer have a live task.
/// Only exposes basic entries (stat, status, cgroup) with limited info.
struct ZombieProcessDir {
    fs: Arc<SimpleFs>,
    proc: Arc<Process>,
}

impl SimpleDirOps for ZombieProcessDir {
    fn child_names<'a>(&'a self) -> Box<dyn Iterator<Item = Cow<'a, str>> + 'a> {
        Box::new(
            ["stat", "status", "cgroup", "cmdline", "exe", "fd"]
                .into_iter()
                .map(Cow::Borrowed),
        )
    }

    fn lookup_child(&self, name: &str) -> VfsResult<NodeOpsMux> {
        let fs = self.fs.clone();
        let proc = self.proc.clone();
        Ok(match name {
            "stat" => SimpleFile::new_regular(fs, move || {
                let pid = proc.pid();
                let state = if proc.is_zombie() { "Z" } else { "S" };
                let ppid = proc.parent().map(|p| p.pid()).unwrap_or(0);
                Ok(format!("{pid} ({state}) {ppid}\n").into_bytes())
            })
            .into(),
            "status" => SimpleFile::new_regular(fs, move || {
                let pid = proc.pid();
                let ppid = proc.parent().map(|p| p.pid()).unwrap_or(0);
                Ok(format!(
                    "Tgid:\t{pid}\n\
                    Pid:\t{pid}\n\
                    PPid:\t{ppid}\n\
                    Uid:\t0 0 0 0\n\
                    Gid:\t0 0 0 0\n"
                ))
            })
            .into(),
            "cgroup" => SimpleFile::new_regular(fs, move || Ok("0::/\n")).into(),
            "cmdline" => SimpleFile::new_regular(fs, move || Ok(Vec::<u8>::new())).into(),
            "exe" => SimpleFile::new(fs, NodeType::Symlink, move || Ok(String::new())).into(),
            "fd" => SimpleDir::new_maker(fs, Arc::new(DirMapping::new())).into(),
            _ => return Err(VfsError::NotFound),
        })
    }

    fn is_cacheable(&self) -> bool {
        false
    }
}

/// Handles /proc/[pid] & /proc/self
struct ProcFsHandler(Arc<SimpleFs>);

impl SimpleDirOps for ProcFsHandler {
    fn child_names<'a>(&'a self) -> Box<dyn Iterator<Item = Cow<'a, str>> + 'a> {
        // Collect task-based names
        let mut names: Vec<Cow<'a, str>> = tasks()
            .into_iter()
            .map(|task| task.id().as_u64().to_string().into())
            .collect();

        // Add zombie processes not already listed (from process table)
        for proc_data in processes() {
            let pid = proc_data.proc.pid().to_string();
            if !names.iter().any(|n| *n == pid) {
                names.push(pid.into());
            }
        }

        // Also add children of current process (may include zombies)
        let curr = current();
        for child in curr.as_thread().proc_data.proc.children() {
            let pid = child.pid().to_string();
            if !names.iter().any(|n| *n == pid) {
                names.push(pid.into());
            }
        }

        names.push(Cow::Borrowed("self"));
        Box::new(names.into_iter())
    }

    fn lookup_child(&self, name: &str) -> VfsResult<NodeOpsMux> {
        if name == "self" {
            let task = current().clone();
            let node = NodeOpsMux::Dir(SimpleDir::new_maker(
                self.0.clone(),
                Arc::new(ThreadDir {
                    fs: self.0.clone(),
                    task: Arc::downgrade(&task),
                }),
            ));
            return Ok(node);
        }

        let pid = name.parse::<u32>().map_err(|_| VfsError::NotFound)?;

        // Try live task first
        if let Ok(task) = get_task(pid) {
            warn!("procfs: lookup /proc/{pid} found live task");
            let node = NodeOpsMux::Dir(SimpleDir::new_maker(
                self.0.clone(),
                Arc::new(ThreadDir {
                    fs: self.0.clone(),
                    task: Arc::downgrade(&task),
                }),
            ));
            return Ok(node);
        }

        // Fallback: check process table for zombie/exited processes
        if let Ok(proc_data) = get_process_data(pid) {
            warn!("procfs: lookup /proc/{pid} found in process table, zombie={}", proc_data.proc.is_zombie());
            let node = NodeOpsMux::Dir(SimpleDir::new_maker(
                self.0.clone(),
                Arc::new(ZombieProcessDir {
                    fs: self.0.clone(),
                    proc: proc_data.proc.clone(),
                }),
            ));
            return Ok(node);
        }

        // Fallback: search current process children for zombie matching this pid
        let curr = current();
        warn!("procfs: lookup /proc/{pid} NOT found in task/process tables, searching children");
        let curr_proc = &curr.as_thread().proc_data.proc;
        if let Some(child) = curr_proc.children().into_iter().find(|c| c.pid() == pid) {
            let node = NodeOpsMux::Dir(SimpleDir::new_maker(
                self.0.clone(),
                Arc::new(ZombieProcessDir {
                    fs: self.0.clone(),
                    proc: child,
                }),
            ));
            return Ok(node);
        }

        warn!("procfs: lookup /proc/{pid} NOT FOUND at all");
        Err(VfsError::NotFound)
    }

    fn is_cacheable(&self) -> bool {
        false
    }
}

fn builder(fs: Arc<SimpleFs>) -> DirMaker {
    let mut root = DirMapping::new();
    root.add(
        "mounts",
        SimpleFile::new_regular(fs.clone(), || {
            Ok("proc /proc proc rw,nosuid,nodev,noexec,relatime 0 0\n")
        }),
    );
    root.add(
        "mountinfo",
        SimpleFile::new_regular(fs.clone(), || {
            Ok(String::from(
                "1 0 254:0 / / rw,relatime - ext4 /dev/vda rw\n\
                 2 1 0:3 / /proc rw,nosuid,nodev,noexec,relatime - proc proc rw\n\
                 3 1 0:6 / /sys rw,nosuid,nodev,noexec,relatime - sysfs sysfs rw\n\
                 4 1 0:5 / /dev rw,nosuid,relatime - devtmpfs devtmpfs rw\n",
            ))
        }),
    );
    root.add(
        "cmdline",
        SimpleFile::new_regular(fs.clone(), || Ok("console=ttyAMA0 systemd.mask=serial-getty@ttyS0.service")),
    );
    root.add(
        "meminfo",
        SimpleFile::new_regular(fs.clone(), || Ok(DUMMY_MEMINFO)),
    );
    root.add(
        "meminfo2",
        SimpleFile::new_regular(fs.clone(), || {
            let allocator = ax_alloc::global_allocator();
            Ok(format!("{:?}\n", allocator.usages()))
        }),
    );
    root.add(
        "instret",
        SimpleFile::new_regular(fs.clone(), || {
            #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
            {
                Ok(format!("{}\n", riscv::register::instret::read64()))
            }
            #[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
            {
                Ok("0\n".to_string())
            }
        }),
    );
    {
        static IRQ_CNT: AtomicUsize = AtomicUsize::new(0);

        ax_task::register_timer_callback(|_| {
            IRQ_CNT.fetch_add(1, Ordering::Relaxed);
        });

        root.add(
            "interrupts",
            SimpleFile::new_regular(fs.clone(), || {
                Ok(format!("0: {}", IRQ_CNT.load(Ordering::Relaxed)))
            }),
        );
    }

    root.add("sys", {
        let mut sys = DirMapping::new();

        sys.add("kernel", {
            let mut kernel = DirMapping::new();

            kernel.add(
                "pid_max",
                SimpleFile::new_regular(fs.clone(), || Ok("32768\n")),
            );

            SimpleDir::new_maker(fs.clone(), Arc::new(kernel))
        });

        SimpleDir::new_maker(fs.clone(), Arc::new(sys))
    });

    let proc_dir = ProcFsHandler(fs.clone());
    SimpleDir::new_maker(fs, Arc::new(proc_dir.chain(root)))
}
