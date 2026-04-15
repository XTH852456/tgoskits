use ax_errno::AxResult;

use crate::task::do_exit;

pub fn sys_exit(exit_code: i32) -> AxResult<isize> {
    debug!("sys_exit <= code: {exit_code}");
    do_exit(exit_code << 8, false);
    Ok(0)
}

pub fn sys_exit_group(exit_code: i32) -> AxResult<isize> {
    debug!("sys_exit_group <= code: {exit_code}");
    do_exit(exit_code << 8, true);
    Ok(0)
}
