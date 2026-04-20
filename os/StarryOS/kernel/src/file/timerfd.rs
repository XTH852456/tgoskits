use alloc::{borrow::Cow, sync::Arc, vec::Vec};
use core::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    task::Context,
};

use ax_errno::{AxError, AxResult};
use ax_hal::time::{TimeValue, monotonic_time, wall_time};
use ax_kspin::SpinNoIrq;
use ax_task::register_timer_callback;
use axpoll::{IoEvents, PollSet, Pollable};
use linux_raw_sys::general::itimerspec;
use starry_vm::{VmMutPtr, VmPtr};

use super::{FileLike, IoDst, IoSrc, add_file_like, get_file_like};

/// Global list of active timer FDs, checked on each timer tick.
static TIMER_FDS: SpinNoIrq<Vec<Arc<TimerFd>>> = SpinNoIrq::new(Vec::new());
static CALLBACK_REGISTERED: AtomicBool = AtomicBool::new(false);

fn ensure_callback_registered() {
    if CALLBACK_REGISTERED.swap(true, Ordering::AcqRel) {
        return;
    }
    register_timer_callback(|_| {
        let mut fds = TIMER_FDS.lock();
        fds.retain(|fd| {
            fd.check_and_fire();
            fd.is_armed()
        });
    });
}

fn timespec_to_duration(ts: &linux_raw_sys::general::timespec) -> TimeValue {
    TimeValue::from_nanos(ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64)
}

fn duration_to_timespec(tv: TimeValue) -> linux_raw_sys::general::timespec {
    let total_nanos = tv.as_nanos();
    linux_raw_sys::general::timespec {
        tv_sec: (total_nanos / 1_000_000_000) as i64,
        tv_nsec: (total_nanos % 1_000_000_000) as i64,
    }
}

const TFD_TIMER_ABSTIME: u32 = 1;

pub struct TimerFd {
    clockid: i32, // 0 = CLOCK_REALTIME, 1 = CLOCK_MONOTONIC
    expiration_count: AtomicU64,
    next_expiry: SpinNoIrq<Option<TimeValue>>,
    interval: SpinNoIrq<TimeValue>,
    armed: AtomicBool,
    poll_rx: PollSet,
    non_blocking: AtomicBool,
}

impl TimerFd {
    pub fn new(clockid: i32) -> Arc<Self> {
        Arc::new(Self {
            clockid,
            expiration_count: AtomicU64::new(0),
            next_expiry: SpinNoIrq::new(None),
            interval: SpinNoIrq::new(TimeValue::from_nanos(0)),
            armed: AtomicBool::new(false),
            poll_rx: PollSet::new(),
            non_blocking: AtomicBool::new(false),
        })
    }

    fn now(&self) -> TimeValue {
        if self.clockid == 1 { monotonic_time() } else { wall_time() }
    }

    pub fn settime_owned(
        self: &Arc<Self>,
        flags: u32,
        new_value: &itimerspec,
    ) -> AxResult<itimerspec> {
        let old = self.gettime_inner();

        let initial_dur = timespec_to_duration(&new_value.it_value);
        let interval_dur = timespec_to_duration(&new_value.it_interval);

        if initial_dur.as_nanos() == 0 {
            self.armed.store(false, Ordering::Release);
            *self.next_expiry.lock() = None;
            *self.interval.lock() = interval_dur;
            TIMER_FDS.lock().retain(|fd| !Arc::ptr_eq(fd, self));
            return Ok(old);
        }

        let now = self.now();
        let next = if flags & TFD_TIMER_ABSTIME != 0 {
            initial_dur
        } else {
            now + initial_dur
        };

        self.expiration_count.store(0, Ordering::Release);
        *self.next_expiry.lock() = Some(next);
        *self.interval.lock() = interval_dur;
        self.armed.store(true, Ordering::Release);

        {
            let mut fds = TIMER_FDS.lock();
            if !fds.iter().any(|fd| Arc::ptr_eq(fd, self)) {
                fds.push(self.clone());
            }
        }

        ensure_callback_registered();
        Ok(old)
    }

    pub fn gettime_inner(&self) -> itimerspec {
        let zero_ts = linux_raw_sys::general::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };

        if !self.armed.load(Ordering::Acquire) {
            return itimerspec {
                it_interval: zero_ts,
                it_value: zero_ts,
            };
        }

        let interval_dur = *self.interval.lock();
        let next = *self.next_expiry.lock();
        let now = self.now();

        let it_interval = duration_to_timespec(interval_dur);
        let it_value = match next {
            Some(expiry) if expiry > now => duration_to_timespec(expiry - now),
            Some(_) => linux_raw_sys::general::timespec { tv_sec: 0, tv_nsec: 1 },
            None => zero_ts,
        };

        itimerspec { it_interval, it_value }
    }

    fn check_and_fire(&self) {
        if !self.armed.load(Ordering::Acquire) {
            return;
        }
        let next_expiry = *self.next_expiry.lock();
        let Some(expiry) = next_expiry else { return };
        let now = self.now();

        if now >= expiry {
            let count = self.expiration_count.load(Ordering::Acquire) + 1;
            let interval = *self.interval.lock();
            warn!("timerfd fired! count={count}, expiry={:?}, now={:?}, interval={:?}", expiry, now, interval);
            self.expiration_count.fetch_add(1, Ordering::AcqRel);
            let interval = *self.interval.lock();
            if interval.as_nanos() > 0 {
                *self.next_expiry.lock() = Some(expiry + interval);
            } else {
                self.armed.store(false, Ordering::Release);
            }
            self.poll_rx.wake();
        }
    }

    fn is_armed(&self) -> bool {
        self.armed.load(Ordering::Acquire)
    }
}

// --- Syscall wrappers ---

pub fn sys_timerfd_create(clockid: i32, flags: u32) -> AxResult<isize> {
    if clockid != 0 && clockid != 1 {
        return Err(AxError::InvalidInput);
    }
    let close_on_exec = (flags & 0x80000) != 0;
    let tfd = TimerFd::new(clockid);
    let fd = add_file_like(tfd, close_on_exec)?;
    warn!("timerfd_create: fd={fd}");
    Ok(fd as isize)
}

pub fn sys_timerfd_settime(
    fd: i32,
    flags: u32,
    new_value: *const itimerspec,
    old_value: *mut itimerspec,
) -> AxResult<isize> {
    let tfd: Arc<TimerFd> = get_file_like(fd)?
        .downcast_arc()
        .map_err(|_| AxError::BadFileDescriptor)?;

    let new_val = unsafe { new_value.vm_read_uninit()?.assume_init() };
    warn!("timerfd_settime: fd={fd}, flags={flags}, initial={}.{}s, interval={}.{}s", new_val.it_value.tv_sec, new_val.it_value.tv_nsec, new_val.it_interval.tv_sec, new_val.it_interval.tv_nsec);
    let old = tfd.settime_owned(flags, &new_val)?;
    if let Some(old_value) = old_value.nullable() {
        old_value.vm_write(old)?;
    }
    Ok(0)
}

pub fn sys_timerfd_gettime(fd: i32, curr_value: *mut itimerspec) -> AxResult<isize> {
    let tfd: Arc<TimerFd> = get_file_like(fd)?
        .downcast_arc()
        .map_err(|_| AxError::BadFileDescriptor)?;
    curr_value.vm_write(tfd.gettime_inner())?;
    Ok(0)
}

// --- FileLike implementation ---

impl FileLike for TimerFd {
    fn read(&self, dst: &mut IoDst) -> AxResult<usize> {
        if dst.remaining_mut() < 8 {
            return Err(AxError::InvalidInput);
        }
        let count = self.expiration_count.swap(0, Ordering::AcqRel);
        if count == 0 {
            return Err(AxError::WouldBlock);
        }
        dst.write(&count.to_ne_bytes())?;
        Ok(8)
    }

    fn write(&self, _src: &mut IoSrc) -> AxResult<usize> {
        Err(AxError::BadFileDescriptor)
    }

    fn nonblocking(&self) -> bool {
        self.non_blocking.load(Ordering::Acquire)
    }

    fn set_nonblocking(&self, non_blocking: bool) -> AxResult {
        self.non_blocking.store(non_blocking, Ordering::Release);
        Ok(())
    }

    fn path(&self) -> Cow<'_, str> {
        "anon_inode:[timerfd]".into()
    }
}

impl Pollable for TimerFd {
    fn poll(&self) -> IoEvents {
        let mut events = IoEvents::empty();
        events.set(IoEvents::IN, self.expiration_count.load(Ordering::Acquire) > 0);
        events
    }

    fn register(&self, context: &mut Context<'_>, events: IoEvents) {
        if events.contains(IoEvents::IN) {
            self.poll_rx.register(context.waker());
        }
    }
}
