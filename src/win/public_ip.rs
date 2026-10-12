//! Read the public address reported for this connection by ipify over HTTPS.

use std::ffi::c_void;
use std::net::IpAddr;
use std::ptr;

use anyhow::{Context, Result, anyhow};
use windows_sys::Win32::Networking::WinHttp::{
    WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE, WinHttpCloseHandle, WinHttpConnect,
    WinHttpOpen, WinHttpOpenRequest, WinHttpQueryDataAvailable, WinHttpReadData,
    WinHttpReceiveResponse, WinHttpSendRequest, WinHttpSetTimeouts,
};

const HOST: &str = "api64.ipify.org";
const MAX_RESPONSE_BYTES: usize = 64;

struct WinHttpHandle(*mut c_void);

impl WinHttpHandle {
    fn new(raw: *mut c_void, name: &str) -> Result<Self> {
        if raw.is_null() {
            return Err(anyhow!(
                "Could not create {name}: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(Self(raw))
    }
}

impl Drop for WinHttpHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                WinHttpCloseHandle(self.0);
            }
        }
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn lookup() -> Result<IpAddr> {
    let agent = wide("NetworkSupport/0.1");
    let host = wide(HOST);
    let method = wide("GET");
    let path = wide("/");

    let session = WinHttpHandle::new(
        unsafe {
            WinHttpOpen(
                agent.as_ptr(),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                ptr::null(),
                ptr::null(),
                0,
            )
        },
        "HTTP session",
    )?;

    if unsafe { WinHttpSetTimeouts(session.0, 5_000, 5_000, 5_000, 5_000) } == 0 {
        return Err(anyhow!(
            "Could not set public IP lookup timeout: {}",
            std::io::Error::last_os_error()
        ));
    }

    let connection = WinHttpHandle::new(
        unsafe { WinHttpConnect(session.0, host.as_ptr(), 443, 0) },
        "HTTP connection",
    )?;
    let request = WinHttpHandle::new(
        unsafe {
            WinHttpOpenRequest(
                connection.0,
                method.as_ptr(),
                path.as_ptr(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                WINHTTP_FLAG_SECURE,
            )
        },
        "HTTPS request",
    )?;

    if unsafe { WinHttpSendRequest(request.0, ptr::null(), 0, ptr::null(), 0, 0, 0) } == 0 {
        return Err(anyhow!(
            "Public IP request failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    if unsafe { WinHttpReceiveResponse(request.0, ptr::null_mut()) } == 0 {
        return Err(anyhow!(
            "Could not receive public IP response: {}",
            std::io::Error::last_os_error()
        ));
    }

    let mut response = Vec::new();
    loop {
        let mut available = 0;
        if unsafe { WinHttpQueryDataAvailable(request.0, &mut available) } == 0 {
            return Err(anyhow!(
                "Could not read public IP response: {}",
                std::io::Error::last_os_error()
            ));
        }
        if available == 0 {
            break;
        }
        if response.len() + available as usize > MAX_RESPONSE_BYTES {
            return Err(anyhow!(
                "Public IP service returned an unexpectedly large response"
            ));
        }

        let mut chunk = vec![0_u8; available as usize];
        let mut read = 0;
        if unsafe { WinHttpReadData(request.0, chunk.as_mut_ptr().cast(), available, &mut read) }
            == 0
        {
            return Err(anyhow!(
                "Could not read public IP response: {}",
                std::io::Error::last_os_error()
            ));
        }
        if read == 0 {
            break;
        }
        response.extend_from_slice(&chunk[..read as usize]);
    }

    let text = std::str::from_utf8(&response).context("Public IP response was not UTF-8")?;
    text.trim()
        .parse::<IpAddr>()
        .context("Public IP service returned an invalid address")
}
