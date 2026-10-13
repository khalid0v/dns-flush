use std::net::Ipv4Addr;

use anyhow::Result;
use ipnet::Ipv4Net;
use serde::Deserialize;
use wmi::WMIConnection;

/// WMI class that exposes per-adapter IP and DNS configuration.
pub const CLASS: &str = "Win32_NetworkAdapterConfiguration";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AdapterInfo {
    pub index: u32,
    pub interface_index: u32,
    pub description: String,
    pub ip_addresses: Vec<String>,
    pub ip_subnets: Vec<String>,
    pub gateways: Vec<String>,
    pub dns_servers: Vec<String>,
    pub dhcp_enabled: bool,
}

impl AdapterInfo {
    pub fn title(&self) -> String {
        let ip = self
            .ip_addresses
            .first()
            .cloned()
            .unwrap_or_else(|| "no ip".to_string());
        format!("#{}  {}  [{}]", self.index, self.description, ip)
    }

    pub fn primary_ip(&self) -> Option<&String> {
        self.ip_addresses
            .iter()
            .find(|a| !a.starts_with("169.254."))
    }

    /// Subnet mask configured for `ip`, if this adapter has one.
    pub fn subnet_for(&self, ip: &str) -> Option<&str> {
        self.ip_addresses
            .iter()
            .position(|a| a == ip)
            .and_then(|i| self.ip_subnets.get(i))
            .map(|s| s.as_str())
    }

    /// CIDR of the primary (non-APIPA) address, e.g. `172.16.0.0/24`.
    pub fn primary_cidr(&self) -> Option<Ipv4Net> {
        let ip = self.primary_ip()?;
        let prefix = crate::win::ip::parse_mask(self.subnet_for(ip)?)?;
        let ip: Ipv4Addr = ip.parse().ok()?;
        let bits = u32::from(ip);
        let network = if prefix == 0 {
            0
        } else {
            bits & (u32::MAX << (32 - prefix))
        };
        Ipv4Net::new(Ipv4Addr::from(network), prefix).ok()
    }

    /// WMI object path used to invoke instance methods on this adapter.
    pub fn wmi_path(&self) -> String {
        format!("Win32_NetworkAdapterConfiguration.Index={}", self.index)
    }
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_NetworkAdapterConfiguration")]
struct RawAdapter {
    Index: u32,
    InterfaceIndex: Option<u32>,
    Description: Option<String>,
    IPEnabled: Option<bool>,
    IPAddress: Option<Vec<String>>,
    IPSubnet: Option<Vec<String>>,
    DefaultIPGateway: Option<Vec<String>>,
    DNSServerSearchOrder: Option<Vec<String>>,
    DHCPEnabled: Option<bool>,
}

pub fn connect() -> Result<WMIConnection> {
    Ok(WMIConnection::new()?)
}

pub fn list(con: &WMIConnection) -> Result<Vec<AdapterInfo>> {
    let raw: Vec<RawAdapter> = con.query()?;
    let mut out: Vec<AdapterInfo> = raw
        .into_iter()
        .filter(|a| a.IPEnabled.unwrap_or(false))
        .map(|a| AdapterInfo {
            index: a.Index,
            interface_index: a.InterfaceIndex.unwrap_or(a.Index),
            description: a.Description.unwrap_or_else(|| "Unknown adapter".into()),
            ip_addresses: a.IPAddress.unwrap_or_default(),
            ip_subnets: a.IPSubnet.unwrap_or_default(),
            gateways: a.DefaultIPGateway.unwrap_or_default(),
            dns_servers: a.DNSServerSearchOrder.unwrap_or_default(),
            dhcp_enabled: a.DHCPEnabled.unwrap_or(false),
        })
        .collect();
    out.sort_by_key(|a| a.index);
    Ok(out)
}
