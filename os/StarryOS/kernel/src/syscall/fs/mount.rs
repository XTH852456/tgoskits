use core::ffi::{c_char, c_void};

use ax_errno::{AxError, AxResult};
use ax_fs::FS_CONTEXT;

use crate::{mm::vm_load_string, pseudofs::{MemoryFs, cgroup}};

pub fn sys_mount(
    source: *const c_char,
    target: *const c_char,
    fs_type: *const c_char,
    _flags: i32,
    _data: *const c_void,
) -> AxResult<isize> {
    let source = vm_load_string(source).unwrap_or_default();
    let target = vm_load_string(target)?;
    let fs_type = vm_load_string(fs_type).unwrap_or_default();
    warn!("sys_mount <= source: {source:?}, target: {target:?}, fs_type: {fs_type:?}, flags: {_flags:#x}");

    match fs_type.as_str() {
        // Flag-changing mount (e.g., MS_REC|MS_SLAVE for mount propagation)
        // Return success since we don't support mount propagation
        "" => {
            warn!("sys_mount: flag-only mount, returning success");
            Ok(0)
        }
        "cgroup2" => {
            // Use our custom cgroup2 filesystem with virtual cgroup files
            let fs = cgroup::new_cgroup2_fs();
            let target_loc = FS_CONTEXT.lock().resolve(&target)?;
            warn!("sys_mount: resolved target '{target}' -> device={}", target_loc.mountpoint().device());
            target_loc.mount(&fs).map_err(|e| {
                warn!("sys_mount: mount failed for '{target}': {e:?}");
                e
            })?;
            warn!("sys_mount: cgroup2 mounted at {target}");
            Ok(0)
        }
        "tmpfs" | "cgroup" | "devpts" | "mqueue" => {
            let fs = MemoryFs::new();
            let target = FS_CONTEXT.lock().resolve(&target)?;
            target.mount(&fs)?;
            Ok(0)
        }
        _ => Err(AxError::NoSuchDevice),
    }
}

pub fn sys_umount2(target: *const c_char, _flags: i32) -> AxResult<isize> {
    let target = vm_load_string(target)?;
    debug!("sys_umount2 <= target: {target:?}");
    let target = FS_CONTEXT.lock().resolve(target)?;
    target.unmount()?;
    Ok(0)
}
