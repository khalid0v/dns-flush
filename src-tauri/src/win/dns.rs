use std::ffi::c_void;
use std::sync::OnceLock;

use anyhow::Result;
use wmi::{Variant, WMIConnection};

use super::adapters::{AdapterInfo, CLASS};
use super::{exec_method, return_code};

unsafe extern "system" {
    fn LoadLibraryA(name: *const u8) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
}

type FlushFn = unsafe extern "system" fn() -> i32;

fn flush_fn() -> &'static Option<FlushFn> {
    static CACHE: OnceLock<Option<FlushFn>> = OnceLock::new();
    CACHE.get_or_init(|| unsafe {
        let lib = LoadLibraryA(c"dnsapi.dll".as_ptr().cast());
        if lib.is_null() {
            return None;
        }
        let proc = GetProcAddress(lib, c"DnsFlushResolverCache".as_ptr().cast());
        if proc.is_null() {
            return None;
        }
        Some(std::mem::transmute::<*mut c_void, FlushFn>(proc))
    })
}

/// Flush the Windows DNS resolver cache (`dnsapi!DnsFlushResolverCache`).
///
/// The symbol is missing from MinGW's import library, so it is resolved at
/// runtime through `LoadLibraryA`/`GetProcAddress`.
pub fn flush() -> bool {
    unsafe {
        match flush_fn() {
            Some(f) => f() != 0,
            None => false,
        }
    }
}

/// Set the static DNS server list on an adapter.
pub fn set_servers(con: &WMIConnection, ad: &AdapterInfo, servers: &[String]) -> Result<u32> {
    let list = Variant::Array(servers.iter().cloned().map(Variant::String).collect());
    let out = exec_method(
        con,
        CLASS,
        &ad.wmi_path(),
        "SetDNSServerSearchOrder",
        vec![("DNSServerSearchOrder", list)],
    )?;
    Ok(return_code(out))
}

/// Reset DNS servers back to automatic (DHCP-provided).
pub fn reset_servers(con: &WMIConnection, ad: &AdapterInfo) -> Result<u32> {
    let out = exec_method(
        con,
        CLASS,
        &ad.wmi_path(),
        "SetDNSServerSearchOrder",
        vec![("DNSServerSearchOrder", Variant::Array(vec![]))],
    )?;
    Ok(return_code(out))
}
