use std::ffi::c_void;
use windows_sys::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};

use crate::win::to_wide;

pub fn is_admin() -> bool {
    unsafe { IsUserAnAdmin() != 0 }
}

/// Relaunch the current executable elevated via the UAC "runas" verb.
/// Returns true if a new elevated process was started (caller should exit).
pub fn relaunch_elevated() -> bool {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let verb = to_wide("runas");
    let file = to_wide(&exe.to_string_lossy());

    let h: *mut c_void = unsafe {
        ShellExecuteW(
            std::ptr::null_mut::<c_void>(),
            verb.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
    h as isize > 32
}
