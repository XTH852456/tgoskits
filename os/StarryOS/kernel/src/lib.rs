//! The core functionality of a monolithic kernel, including loading user
//! programs and managing processes.

#![no_std]
#![feature(likely_unlikely)]
#![feature(bstr)]
#![feature(c_variadic)]
#![allow(missing_docs)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

extern crate alloc;
extern crate ax_runtime;

#[macro_use]
extern crate ax_log;

pub mod entry;

mod config;
mod file;
mod mm;
mod pseudofs;
mod syscall;
mod task;
mod time;

/// Override the weak `printf` from `lwext4_rust::ulibc` to prevent the
/// `while(1)` in lwext4's `ext4_assert` macro from hanging the kernel.
/// When an assertion fails, we panic instead of spinning forever.
mod lwext4_assert_fix {
    use core::ffi::{c_char, c_int};

    /// Strong `printf` symbol that overrides the weak one in `lwext4_rust::ulibc`.
    /// lwext4's `ext4_assert` calls `printf("assertion failed:...")` followed by
    /// `while(1);`. By panicking here, we prevent the infinite loop.
    #[unsafe(no_mangle)]
    unsafe extern "C" fn printf(fmt: *const c_char, _args: ...) -> c_int {
        use core::ffi::CStr;
        if fmt.is_null() {
            return 0;
        }
        let c_str = unsafe { CStr::from_ptr(fmt) };
        let s = c_str.to_bytes();

        // Check if this is an assertion failure message from ext4_assert
        if s.starts_with(b"assertion failed") {
            panic!("[lwext4] {:?}", c_str);
        }

        // Normal debug output - just log it
        info!("[lwext4] {:?}", c_str);
        0
    }
}
