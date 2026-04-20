use alloc::vec::Vec;
use core::{future::poll_fn, task::Poll};

use ax_errno::{AxError, AxResult, LinuxError};
use ax_task::{
    current,
    future::{block_on, interruptible},
};
use bitflags::bitflags;
use linux_raw_sys::general::{
    CLD_EXITED, P_ALL, P_PGID, P_PID, SIGCHLD, WCONTINUED, WEXITED, WNOHANG, WNOWAIT, WUNTRACED,
    __WALL, __WCLONE, __WNOTHREAD,
    __sifields__bindgen_ty_4, siginfo, siginfo__bindgen_ty_1, siginfo__bindgen_ty_1__bindgen_ty_1,
    __sifields,
};

/// Linux `P_PIDFD` idtype for `waitid()` (since Linux 5.4).
/// Not exported by `linux_raw_sys`, defined manually.
const P_PIDFD: u32 = 3;
use starry_process::{Pid, Process};
use starry_vm::{VmMutPtr, VmPtr};

use crate::file::{FileLike, PidFd};
use crate::task::AsThread;

bitflags! {
    #[derive(Debug)]
    struct WaitOptions: u32 {
        /// Do not block when there are no processes wishing to report status.
        const WNOHANG = WNOHANG;
        /// Report the status of selected processes which are stopped due to a
        /// `SIGTTIN`, `SIGTTOU`, `SIGTSTP`, or `SIGSTOP` signal.
        const WUNTRACED = WUNTRACED;
        /// Report the status of selected processes which have terminated.
        const WEXITED = WEXITED;
        /// Report the status of selected processes that have continued from a
        /// job control stop by receiving a `SIGCONT` signal.
        const WCONTINUED = WCONTINUED;
        /// Don't reap, just poll status.
        const WNOWAIT = WNOWAIT;

        /// Don't wait on children of other threads in this group
        const WNOTHREAD = __WNOTHREAD;
        /// Wait on all children, regardless of type
        const WALL = __WALL;
        /// Wait for "clone" children only.
        const WCLONE = __WCLONE;
    }
}

#[derive(Debug, Clone, Copy)]
enum WaitPid {
    /// Wait for any child process
    Any,
    /// Wait for the child whose process ID is equal to the value.
    Pid(Pid),
    /// Wait for any child process whose process group ID is equal to the value.
    Pgid(Pid),
}

impl WaitPid {
    fn apply(&self, child: &Process) -> bool {
        match self {
            WaitPid::Any => true,
            WaitPid::Pid(pid) => child.pid() == *pid,
            WaitPid::Pgid(pgid) => child.group().pgid() == *pgid,
        }
    }
}

pub fn sys_waitpid(pid: i32, exit_code: *mut i32, options: u32) -> AxResult<isize> {
    let options = WaitOptions::from_bits_truncate(options);
    info!("sys_waitpid <= pid: {pid:?}, options: {options:?}");

    let curr = current();
    let proc_data = &curr.as_thread().proc_data;
    let proc = &proc_data.proc;

    let pid = if pid == -1 {
        WaitPid::Any
    } else if pid == 0 {
        WaitPid::Pgid(proc.group().pgid())
    } else if pid > 0 {
        WaitPid::Pid(pid as _)
    } else {
        WaitPid::Pgid(-pid as _)
    };

    // FIXME: add back support for WALL & WCLONE, since ProcessData may drop before
    // Process now.
    let children = proc
        .children()
        .into_iter()
        .filter(|child| pid.apply(child))
        .collect::<Vec<_>>();
    if children.is_empty() {
        return Err(AxError::from(LinuxError::ECHILD));
    }

    let check_children = || {
        if let Some(child) = children.iter().find(|child| child.is_zombie()) {
            if !options.contains(WaitOptions::WNOWAIT) {
                child.free();
            }
            if let Some(exit_code) = exit_code.nullable() {
                exit_code.vm_write(child.exit_code())?;
            }
            Ok(Some(child.pid() as _))
        } else if options.contains(WaitOptions::WNOHANG) {
            Ok(Some(0))
        } else {
            Ok(None)
        }
    };

    let result = block_on(interruptible(poll_fn(|cx| {
        match check_children().transpose() {
            Some(res) => Poll::Ready(res),
            None => {
                proc_data.child_exit_event.register(cx.waker());
                Poll::Pending
            }
        }
    })))?;
    result
}

/// `waitid(idtype, id, infop, options)` — wait for a child process to change state.
pub fn sys_waitid(
    idtype: u32,
    id: i32,
    infop: *mut siginfo,
    options: u32,
) -> AxResult<isize> {
    warn!("sys_waitid <= idtype: {idtype}, id: {id}, options: {options}");
    let wait_opts = WaitOptions::from_bits_truncate(options);

    let curr = current();
    let proc_data = &curr.as_thread().proc_data;
    let proc = &proc_data.proc;

    // Determine which child to wait for based on idtype
    let wait_pid = match idtype {
        P_ALL => WaitPid::Any,
        P_PID => WaitPid::Pid(id as Pid),
        P_PGID => {
            if id == 0 {
                WaitPid::Pgid(proc.group().pgid())
            } else {
                WaitPid::Pgid(id as Pid)
            }
        }
        P_PIDFD => {
            // `id` is a file descriptor referring to a pidfd.
            let pidfd = PidFd::from_fd(id)?;
            let pid = pidfd.process_data()?.proc.pid();
            WaitPid::Pid(pid)
        }
        _ => return Err(AxError::InvalidInput),
    };

    let children: Vec<_> = proc
        .children()
        .into_iter()
        .filter(|child| wait_pid.apply(child))
        .collect();

    if children.is_empty() {
        return Err(AxError::from(LinuxError::ECHILD));
    }

    let check_children = || -> AxResult<Option<isize>> {
        if let Some(child) = children.iter().find(|child| child.is_zombie()) {
            if !wait_opts.contains(WaitOptions::WNOWAIT) {
                child.free();
            }
            // Write siginfo if infop is non-null
            if let Some(infop) = infop.nullable() {
                let si = siginfo {
                    __bindgen_anon_1: siginfo__bindgen_ty_1 {
                        __bindgen_anon_1: siginfo__bindgen_ty_1__bindgen_ty_1 {
                            si_signo: SIGCHLD as _,
                            si_errno: 0,
                            si_code: CLD_EXITED as _,
                            _sifields: __sifields {
                                _sigchld: __sifields__bindgen_ty_4 {
                                    _pid: child.pid() as _,
                                    _uid: 0,
                                    _status: child.exit_code() >> 8,
                                    _utime: 0,
                                    _stime: 0,
                                },
                            },
                        },
                    },
                };
                infop.vm_write(si)?;
            }
            Ok(Some(child.pid() as _))
        } else if wait_opts.contains(WaitOptions::WNOHANG) {
            // WNOHANG: write zeroed siginfo and return 0
            if let Some(infop) = infop.nullable() {
                let si = siginfo {
                    __bindgen_anon_1: siginfo__bindgen_ty_1 {
                        __bindgen_anon_1: siginfo__bindgen_ty_1__bindgen_ty_1 {
                            si_signo: 0,
                            si_errno: 0,
                            si_code: 0,
                            _sifields: unsafe { core::mem::zeroed() },
                        },
                    },
                };
                infop.vm_write(si)?;
            }
            Ok(Some(0))
        } else {
            Ok(None)
        }
    };

    let result = block_on(interruptible(poll_fn(|cx| {
        match check_children().transpose() {
            Some(res) => Poll::Ready(res),
            None => {
                proc_data.child_exit_event.register(cx.waker());
                Poll::Pending
            }
        }
    })))?;
    warn!("sys_waitid => result={:?}", result);
    result
}
