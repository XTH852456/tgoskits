#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![doc = include_str!("../README.md")]

#[cfg(target_os = "none")]
extern crate alloc;

#[cfg(target_os = "none")]
use alloc::{borrow::ToOwned, vec::Vec};

#[cfg(target_os = "none")]
pub const CMDLINE: &[&str] = &["/bin/sh", "-c", include_str!("init.sh")];

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
fn main() {
    let args = CMDLINE
        .iter()
        .copied()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let envs = [];

    starry_kernel::entry::init(&args, &envs);
}

#[cfg(not(target_os = "none"))]
fn main() {}

#[cfg(all(target_os = "none", feature = "vf2"))]
extern crate axplat_riscv64_visionfive2;
