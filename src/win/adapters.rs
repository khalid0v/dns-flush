use anyhow::Result;
use serde::Deserialize;
use wmi::WMIConnection;

#[derive(Clone, Debug, Default)]
pub struct AdapterInfo {
    pub index: u32,
    pub interface_index: u32,
    pub description: String,
    pub ip_addresses: Vec<String>,
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
            gateways: a.DefaultIPGateway.unwrap_or_default(),
            dns_servers: a.DNSServerSearchOrder.unwrap_or_default(),
            dhcp_enabled: a.DHCPEnabled.unwrap_or(false),
        })
        .collect();
    out.sort_by_key(|a| a.index);
    Ok(out)
}
