pub mod adapters;
pub mod dns;
pub mod ip;
pub mod net;
pub mod public_ip;
pub mod reset;
pub mod scan;
pub mod watch;

pub use adapters::AdapterInfo;

use anyhow::Result;
use wmi::Variant;

pub use wmi::WMIConnection;

/// NUL-terminated UTF-16 buffer for Win32 wide-string (`*W`) APIs.
pub fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Invoke a WMI instance method with arbitrary input parameters.
///
/// The high-level `WMIConnection::exec_instance_method` cannot pass array
/// parameters, so callers that need `Variant::Array` (DNS servers, static
/// addresses, gateways) must go through this mid-level path.
pub fn exec_method(
    con: &WMIConnection,
    class: &str,
    path: &str,
    method: &str,
    params: Vec<(&str, Variant)>,
) -> Result<Option<Variant>> {
    let class_obj = con.get_object(class)?;
    let in_sig = match class_obj.get_method(method)? {
        Some(sig) => sig,
        None => {
            con.exec_method(path, method, None)?;
            return Ok(None);
        }
    };
    let inst = in_sig.spawn_instance()?;
    for (name, value) in params {
        inst.put_property(name, value)?;
    }
    let out = con.exec_method(path, method, Some(&inst))?;
    match out {
        Some(obj) => Ok(Some(obj.get_property("ReturnValue")?)),
        None => Ok(None),
    }
}

pub fn return_code(v: Option<Variant>) -> u32 {
    match v {
        Some(Variant::UI4(n)) => n,
        Some(Variant::I4(n)) => n as u32,
        _ => 0,
    }
}
