use std::net::Ipv4Addr;

use anyhow::Result;
use wmi::{Variant, WMIConnection};

use super::{adapters::AdapterInfo, exec_method, return_code};

pub fn prefix_to_mask(prefix: u8) -> String {
    let bits: u32 = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix.min(32))
    };
    Ipv4Addr::from(bits).to_string()
}

/// Parse a dotted mask or a bare prefix length into a prefix (0-32).
pub fn parse_mask(text: &str) -> Option<u8> {
    let text = text.trim();
    if let Ok(n) = text.parse::<u8>() {
        return (n <= 32).then_some(n);
    }
    let ip: Ipv4Addr = text.parse().ok()?;
    let bits = u32::from(ip);
    let prefix = bits.leading_ones();
    let rest = if prefix == 32 { 0 } else { bits << prefix };
    (rest == 0).then_some(prefix as u8)
}

/// Apply one or more static IPv4 addresses to an adapter.
pub fn set_static(
    con: &WMIConnection,
    ad: &AdapterInfo,
    ips: &[(String, u8)],
    gateway: Option<&str>,
    dns: Option<&[String]>,
) -> Result<u32> {
    let path = ad.wmi_path();

    let addresses = Variant::Array(
        ips.iter()
            .map(|(ip, _)| Variant::String(ip.clone()))
            .collect(),
    );
    let masks = Variant::Array(
        ips.iter()
            .map(|(_, prefix)| Variant::String(prefix_to_mask(*prefix)))
            .collect(),
    );
    let code = return_code(exec_method(
        con,
        "Win32_NetworkAdapterConfiguration",
        &path,
        "EnableStatic",
        vec![("IPAddress", addresses), ("SubnetMask", masks)],
    )?);
    if code != 0 {
        return Ok(code);
    }

    if let Some(gw) = gateway {
        let gws = Variant::Array(vec![Variant::String(gw.to_string())]);
        let metrics = Variant::Array(vec![Variant::UI2(1)]);
        let code = return_code(exec_method(
            con,
            "Win32_NetworkAdapterConfiguration",
            &path,
            "SetGateways",
            vec![("DefaultIPGateway", gws), ("GatewayCostMetric", metrics)],
        )?);
        if code != 0 {
            return Ok(code);
        }
    }

    if let Some(servers) = dns
        && !servers.is_empty()
    {
        return super::dns::set_servers(con, ad, servers);
    }

    Ok(0)
}

/// Switch an adapter back to DHCP.
pub fn enable_dhcp(con: &WMIConnection, ad: &AdapterInfo) -> Result<u32> {
    let code = return_code(exec_method(
        con,
        "Win32_NetworkAdapterConfiguration",
        &ad.wmi_path(),
        "EnableDHCP",
        vec![],
    )?);
    if code != 0 {
        return Ok(code);
    }
    let _ = super::dns::reset_servers(con, ad);
    Ok(0)
}

pub fn release_lease(con: &WMIConnection, ad: &AdapterInfo) -> Result<u32> {
    Ok(return_code(exec_method(
        con,
        "Win32_NetworkAdapterConfiguration",
        &ad.wmi_path(),
        "ReleaseDHCPLease",
        vec![],
    )?))
}

pub fn renew_lease(con: &WMIConnection, ad: &AdapterInfo) -> Result<u32> {
    Ok(return_code(exec_method(
        con,
        "Win32_NetworkAdapterConfiguration",
        &ad.wmi_path(),
        "RenewDHCPLease",
        vec![],
    )?))
}
