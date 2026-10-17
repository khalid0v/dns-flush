//! Read-only network scan (ping sweep + ARP table).

use crate::models::ScanResult;
use crate::win::scan;

use super::find_adapter;

#[tauri::command]
pub async fn scan_network(index: u32, cidr: String) -> Result<ScanResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let adapter = find_adapter(index)?;
        let net = scan::parse_cidr(&cidr).ok_or_else(|| format!("Invalid CIDR: {cidr}"))?;
        Ok(scan::scan_result(adapter.interface_index, net, &adapter))
    })
    .await
    .unwrap_or(Err("Scan failed.".to_string()))
}
