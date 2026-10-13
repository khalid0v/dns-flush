use std::net::{IpAddr, Ipv4Addr};

use serde::Serialize;

use crate::win::adapters::AdapterInfo;

/// One device found on a scanned network.
#[derive(Clone, Debug, Serialize)]
pub struct ScanDevice {
    pub ip: Ipv4Addr,
    pub mac: String,
    /// Human note, e.g. "this PC" or "gateway".
    pub note: String,
}

/// Result of a network scan: the network, the devices in use, and free hosts.
#[derive(Clone, Debug, Default, Serialize)]
pub struct ScanResult {
    pub network: String,
    pub used: Vec<ScanDevice>,
    pub free: Vec<Ipv4Addr>,
}

/// Commands sent from the UI to the background status monitor.
pub enum MonitorCmd {
    /// Read the current state now, ignoring the fallback timer.
    RefreshNow,
    /// Enable or pause live updates.
    SetEnabled(bool),
}

/// Read-only observations produced by the background status monitor.
pub enum StatusMsg {
    Adapters(Vec<AdapterInfo>),
    PublicIp(Result<IpAddr, String>),
}
