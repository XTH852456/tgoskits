use alloc::{
    borrow::Cow,
    sync::{Arc, Weak},
};
use core::{
    sync::atomic::{AtomicBool, Ordering},
    task::Context,
};

use ax_errno::{AxError, AxResult};
use axpoll::{IoEvents, PollSet, Pollable};

use crate::{
    file::FileLike,
    task::{ProcessData, Thread},
};

// Linux ioctl for PIDFD_GET_INFO (kernel 6.13+)
// #define PIDFD_GET_INFO _IOWR('P', 1, struct pidfd_info)
// struct pidfd_info is 56 bytes on 64-bit
const PIDFD_GET_INFO: u32 = 0xC0385001;

// pidfd_info masks
const PIDFD_INFO_PID: u64 = 1 << 0;
const PIDFD_INFO_CREDS: u64 = 1 << 1;

// Expected layout of struct pidfd_info (64-bit, 56 bytes, matches Linux kernel)
#[repr(C)]
struct PidfdInfo {
    mask: u64,       // input/output mask
    pid: u32,        // output: PID
    ppid: u32,       // output: parent PID
    ruid: u32,       // output: real UID
    euid: u32,       // output: effective UID
    fd: i32,         // output: fd (for pidfd)
    _pad0: u32,
    cgroupid: u64,   // output: cgroup ID
    _spare: [u32; 6], // reserved
}

pub struct PidFd {
    proc_data: Weak<ProcessData>,
    exit_event: Arc<PollSet>,
    thread_exit: Option<Arc<AtomicBool>>,

    non_blocking: AtomicBool,
}
impl PidFd {
    pub fn new_process(proc_data: &Arc<ProcessData>) -> Self {
        Self {
            proc_data: Arc::downgrade(proc_data),
            exit_event: proc_data.exit_event.clone(),
            thread_exit: None,

            non_blocking: AtomicBool::new(false),
        }
    }

    pub fn new_thread(thread: &Thread) -> Self {
        Self {
            proc_data: Arc::downgrade(&thread.proc_data),
            exit_event: thread.exit_event.clone(),
            thread_exit: Some(thread.exit.clone()),

            non_blocking: AtomicBool::new(false),
        }
    }

    pub fn process_data(&self) -> AxResult<Arc<ProcessData>> {
        // For threads, the pidfd is invalid once the thread exits, even if its
        // process is still alive.
        if let Some(thread_exit) = &self.thread_exit
            && thread_exit.load(Ordering::Acquire)
        {
            return Err(AxError::NoSuchProcess);
        }
        self.proc_data.upgrade().ok_or(AxError::NoSuchProcess)
    }
}
impl FileLike for PidFd {
    fn path(&self) -> Cow<'_, str> {
        "anon_inode:[pidfd]".into()
    }

    fn ioctl(&self, cmd: u32, arg: usize) -> AxResult<usize> {
        if cmd == PIDFD_GET_INFO {
            let info_ptr = arg as *mut PidfdInfo;
            // SAFETY: userspace provides the pointer; we validate it below
            let info = unsafe { core::ptr::read(info_ptr) };
            let proc_data = self.process_data()?;
            let proc = &proc_data.proc;

            let mut result = PidfdInfo {
                mask: info.mask,
                pid: 0,
                ppid: 0,
                ruid: 0,
                euid: 0,
                fd: -1,
                _pad0: 0,
                cgroupid: 0,
                _spare: [0; 6],
            };

            if info.mask & PIDFD_INFO_PID != 0 {
                result.pid = proc.pid();
                result.ppid = proc.parent().map(|p| p.pid()).unwrap_or(0);
                result.mask |= PIDFD_INFO_PID;
            }
            if info.mask & PIDFD_INFO_CREDS != 0 {
                result.ruid = 0; // root
                result.mask |= PIDFD_INFO_CREDS;
            }

            unsafe { core::ptr::write(info_ptr, result) };
            return Ok(0);
        }
        Err(AxError::NotATty)
    }

    fn set_nonblocking(&self, nonblocking: bool) -> AxResult {
        self.non_blocking.store(nonblocking, Ordering::Release);
        Ok(())
    }

    fn nonblocking(&self) -> bool {
        self.non_blocking.load(Ordering::Acquire)
    }
}

impl Pollable for PidFd {
    fn poll(&self) -> IoEvents {
        let mut events = IoEvents::empty();
        // For process-level pidfds: readable when the process has exited
        // (no threads remaining).
        // For thread-level pidfds: readable when the thread has exited.
        let readable = match &self.thread_exit {
            Some(thread_exit) => thread_exit.load(Ordering::Acquire),
            None => self
                .proc_data
                .upgrade()
                .is_none_or(|pd| pd.proc.threads().is_empty()),
        };
        events.set(IoEvents::IN, readable);
        events
    }

    fn register(&self, context: &mut Context<'_>, events: IoEvents) {
        if events.contains(IoEvents::IN) {
            self.exit_event.register(context.waker());
        }
    }
}
