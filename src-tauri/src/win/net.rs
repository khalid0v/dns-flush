use std::ffi::c_void;
use std::net::Ipv4Addr;

use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::NetworkManagement::IpHelper::{
    ICMP_ECHO_REPLY, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho,
};

fn ip_to_u32(ip: Ipv4Addr) -> u32 {
    u32::from_ne_bytes(ip.octets())
}

pub fn u32_to_ip(v: u32) -> Ipv4Addr {
    Ipv4Addr::from(v.to_ne_bytes())
}

/// Send a single ICMP echo. Returns the round-trip time in ms on success.
pub fn ping(ip: Ipv4Addr, timeout_ms: u32) -> Option<u32> {
    unsafe {
        let handle = IcmpCreateFile();
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return None;
        }

        let payload = [0u8; 16];
        let mut reply = [0u8; 128];

        let sent = IcmpSendEcho(
            handle,
            ip_to_u32(ip),
            payload.as_ptr() as *const c_void,
            payload.len() as u16,
            std::ptr::null(),
            reply.as_mut_ptr() as *mut c_void,
            reply.len() as u32,
            timeout_ms,
        );

        let result = if sent > 0 {
            let reply = &*(reply.as_ptr() as *const ICMP_ECHO_REPLY);
            if reply.Status == 0 {
                Some(reply.RoundTripTime)
            } else {
                None
            }
        } else {
            None
        };

        IcmpCloseHandle(handle);
        result
    }
}
