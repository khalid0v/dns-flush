use std::net::Ipv4Addr;

use serde::Serialize;

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
