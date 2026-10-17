//! Internal channel messages for the background status monitor: how the UI
//! asks it to refresh, and what it reports back. Domain shapes live in
//! `crate::models`.

use std::net::IpAddr;

use crate::win::adapters::AdapterInfo;

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
