use std::ffi::c_void;
use windows_sys::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

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
    let verb = wide("runas");
    let file = wide(&exe.to_string_lossy());

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
