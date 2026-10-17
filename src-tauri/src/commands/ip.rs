//! IP address commands: DHCP lease release/renew, switch to DHCP, apply a
//! static address.

use std::net::Ipv4Addr;

use tauri::AppHandle;

use crate::models::LogLine;
use crate::win::{ip, reset};

use super::{Logger, find_adapter, wmi_connection};

#[tauri::command]
pub async fn register_renew(app: AppHandle, index: u32) -> Vec<LogLine> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut log = Logger::new(app, "[2] Register DNS and renew IP");
        let adapter = match find_adapter(index) {
            Ok(adapter) => adapter,
            Err(error) => {
                log.log("err", error);
                return log.finish();
            }
        };
        match reset::register_dns() {
            Ok(lines) => {
                for line in lines {
                    log.log("info", line);
                }
            }
            Err(error) => log.log("err", format!("registerdns failed: {error}")),
        }
        match wmi_connection() {
            Ok(connection) => {
                match ip::release_lease(&connection, &adapter) {
                    Ok(code) => log.log("info", format!("ReleaseDHCPLease -> {code}")),
                    Err(error) => log.log("err", format!("Release failed: {error}")),
                }
                match ip::renew_lease(&connection, &adapter) {
                    Ok(code) => log.log("ok", format!("RenewDHCPLease -> {code}")),
                    Err(error) => log.log("err", format!("Renew failed: {error}")),
                }
            }
            Err(error) => log.log("err", error),
        }
        log.finish()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub async fn enable_dhcp(app: AppHandle, index: u32) -> Vec<LogLine> {
    tauri::async_runtime::spawn_blocking(move || {
        let adapter = match find_adapter(index) {
            Ok(adapter) => adapter,
            Err(error) => {
                let mut log = Logger::new(app, "[6] Switch to DHCP");
                log.log("err", error);
                return log.finish();
            }
        };
        let mut log = Logger::new(app, &format!("[6] Switch {} to DHCP", adapter.description));
        match wmi_connection() {
            Ok(connection) => log.report(
                "EnableDHCP",
                "Adapter now uses DHCP.",
                ip::enable_dhcp(&connection, &adapter),
            ),
            Err(error) => log.log("err", error),
        }
        log.finish()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn set_static_ip(
    app: AppHandle,
    index: u32,
    address: String,
    prefix: u8,
    alt_address: Option<String>,
    gateway: Option<String>,
    dns_servers: Vec<String>,
) -> Vec<LogLine> {
    tauri::async_runtime::spawn_blocking(move || {
        let adapter = match find_adapter(index) {
            Ok(adapter) => adapter,
            Err(error) => {
                let mut log = Logger::new(app, "[6] Apply static IP");
                log.log("err", error);
                return log.finish();
            }
        };
        let mut log = Logger::new(
            app,
            &format!("[6] Apply static IP to {}", adapter.description),
        );
        if prefix > 32 {
            log.log("err", format!("Invalid subnet prefix: {prefix}"));
            return log.finish();
        }
        if address.parse::<Ipv4Addr>().is_err() {
            log.log("err", format!("Invalid IP address: {address}"));
            return log.finish();
        }
        let mut ips = vec![(address, prefix)];
        if let Some(alt) = alt_address.filter(|value| !value.trim().is_empty()) {
            if alt.parse::<Ipv4Addr>().is_err() {
                log.log("err", format!("Invalid alternate IP: {alt}"));
                return log.finish();
            }
            ips.push((alt, prefix));
        }
        let gateway = gateway.filter(|value| !value.trim().is_empty());
        let dns = (!dns_servers.is_empty()).then_some(dns_servers);
        for (item, item_prefix) in &ips {
            log.log("info", format!("  address {item}/{item_prefix}"));
        }
        match wmi_connection() {
            Ok(connection) => log.report(
                "EnableStatic",
                "Static IP applied.",
                ip::set_static(
                    &connection,
                    &adapter,
                    &ips,
                    gateway.as_deref(),
                    dns.as_deref(),
                ),
            ),
            Err(error) => log.log("err", error),
        }
        log.finish()
    })
    .await
    .unwrap_or_default()
}
