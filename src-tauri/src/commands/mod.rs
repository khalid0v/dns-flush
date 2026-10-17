//! Tauri commands: every toolkit action, ported from the former egui worker
//! threads. Blocking Windows work runs on `spawn_blocking` threads; progress
//! streams back as `log` events, and the collected lines are returned so the
//! frontend can clear its busy state when the promise resolves.
//!
//! Each command opens its own WMI connection inside the worker thread:
//! `WMIConnection` is not `Send`, so connections are never shared.

use std::net::Ipv4Addr;
use std::sync::{Mutex, mpsc::Sender};

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use wmi::WMIConnection;

use crate::msg::{MonitorCmd, ScanResult};
use crate::win::{adapters, dns, ip, public_ip, reset, scan};

/// One activity-log line, streamed as a `log` event and collected as the
/// command's return value.
#[derive(Clone, Debug, Serialize)]
pub struct LogLine {
    pub kind: String,
    pub text: String,
}

/// Sender that drives the read-only background monitor. Managed as Tauri state.
pub struct MonitorState {
    cmd_tx: Mutex<Sender<MonitorCmd>>,
}

impl MonitorState {
    pub fn new(cmd_tx: Sender<MonitorCmd>) -> Self {
        Self {
            cmd_tx: Mutex::new(cmd_tx),
        }
    }

    fn send(&self, cmd: MonitorCmd) -> Result<(), String> {
        self.cmd_tx
            .lock()
            .map_err(|_| "Monitor is unavailable.".to_string())?
            .send(cmd)
            .map_err(|_| "Monitor is unavailable.".to_string())
    }
}

/// Streams log lines as `log` events while collecting them for the return value.
struct Logger {
    app: AppHandle,
    out: Vec<LogLine>,
}

impl Logger {
    fn new(app: AppHandle, head: &str) -> Self {
        let mut logger = Self {
            app,
            out: Vec::new(),
        };
        logger.log("head", head.to_string());
        logger
    }

    fn log(&mut self, kind: &str, text: String) {
        let line = LogLine {
            kind: kind.to_string(),
            text,
        };
        let _ = self.app.emit("log", &line);
        self.out.push(line);
    }

    /// Log the outcome of a Win32 call that reports a status code (`0` means
    /// success), mirroring the old `report_status` helper.
    fn report(&mut self, operation: &str, success: &str, result: anyhow::Result<u32>) {
        match result {
            Ok(0) => self.log("ok", success.to_string()),
            Ok(code) => self.log("err", format!("{operation} returned code {code}")),
            Err(error) => self.log("err", format!("{operation} failed: {error}")),
        }
    }

    fn finish(self) -> Vec<LogLine> {
        self.out
    }
}

fn wmi_connection() -> Result<WMIConnection, String> {
    adapters::connect().map_err(|error| format!("WMI error: {error}"))
}

/// Fresh snapshot of the adapter with WMI `index`, or an error line's text.
fn find_adapter(index: u32) -> Result<adapters::AdapterInfo, String> {
    let connection = wmi_connection()?;
    adapters::list(&connection)
        .map_err(|error| format!("Adapter enumeration failed: {error}"))?
        .into_iter()
        .find(|adapter| adapter.index == index)
        .ok_or_else(|| "No adapter selected.".to_string())
}

#[tauri::command]
pub async fn list_adapters() -> Result<Vec<adapters::AdapterInfo>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let connection = wmi_connection()?;
        adapters::list(&connection).map_err(|error| format!("Adapter enumeration failed: {error}"))
    })
    .await
    .unwrap_or(Err("Adapter enumeration failed.".to_string()))
}

#[tauri::command]
pub async fn get_public_ip() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(|| {
        public_ip::lookup()
            .map(|address| address.to_string())
            .map_err(|error| error.to_string())
    })
    .await
    .unwrap_or(Err("Public IP lookup failed.".to_string()))
}

#[tauri::command]
pub async fn flush_dns(app: AppHandle) -> Vec<LogLine> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut log = Logger::new(app, "[1] Flush DNS resolver cache");
        if dns::flush() {
            log.log("ok", "DNS cache flushed.".to_string());
        } else {
            log.log(
                "warn",
                "Native flush failed, falling back to ipconfig /flushdns".to_string(),
            );
            match reset::flush_dns_fallback() {
                Ok(lines) => {
                    for line in lines {
                        log.log("info", line);
                    }
                }
                Err(error) => log.log("err", format!("Flush failed: {error}")),
            }
        }
        log.finish()
    })
    .await
    .unwrap_or_default()
}

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
pub async fn reset_stack(app: AppHandle) -> Vec<LogLine> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut log = Logger::new(app, "[3] Reset Winsock and TCP/IP stack");
        log.log("warn", "Resetting Winsock...".to_string());
        match reset::winsock_reset() {
            Ok(lines) => {
                for line in lines {
                    log.log("info", line);
                }
            }
            Err(error) => log.log("err", format!("winsock reset failed: {error}")),
        }
        log.log("warn", "Resetting TCP/IP stack...".to_string());
        match reset::ip_reset() {
            Ok(lines) => {
                for line in lines {
                    log.log("info", line);
                }
            }
            Err(error) => log.log("err", format!("ip reset failed: {error}")),
        }
        log.log("ok", "Reset complete. Restart the computer to finish.".to_string());
        log.finish()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub async fn set_dns(app: AppHandle, index: u32, servers: Vec<String>) -> Vec<LogLine> {
    tauri::async_runtime::spawn_blocking(move || {
        let adapter = match find_adapter(index) {
            Ok(adapter) => adapter,
            Err(error) => {
                let mut log = Logger::new(app, "[5] Set DNS");
                log.log("err", error);
                return log.finish();
            }
        };
        let mut log = Logger::new(app, &format!("[5] Set DNS on {}", adapter.description));
        if servers.is_empty() {
            log.log("err", "No DNS servers given.".to_string());
            return log.finish();
        }
        for server in &servers {
            if server.parse::<Ipv4Addr>().is_err() {
                log.log("err", format!("Invalid DNS server: {server}"));
                return log.finish();
            }
        }
        match wmi_connection() {
            Ok(connection) => log.report(
                "SetDNS",
                &format!("DNS set to {}", servers.join(", ")),
                dns::set_servers(&connection, &adapter, &servers),
            ),
            Err(error) => log.log("err", error),
        }
        log.finish()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub async fn reset_dns(app: AppHandle, index: u32) -> Vec<LogLine> {
    tauri::async_runtime::spawn_blocking(move || {
        let adapter = match find_adapter(index) {
            Ok(adapter) => adapter,
            Err(error) => {
                let mut log = Logger::new(app, "[5] Reset DNS to automatic");
                log.log("err", error);
                return log.finish();
            }
        };
        let mut log = Logger::new(
            app,
            &format!("[5] Reset DNS to automatic on {}", adapter.description),
        );
        match wmi_connection() {
            Ok(connection) => log.report(
                "Reset",
                "DNS reset to automatic.",
                dns::reset_servers(&connection, &adapter),
            ),
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

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn auto_setup(
    app: AppHandle,
    index: u32,
    primary_net: String,
    alt_net: String,
    first_host: u32,
    alt_start_host: u32,
    auto_alt: bool,
) -> Vec<LogLine> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut log = Logger::new(app, "[8] AUTO setup");
        let adapter = match find_adapter(index) {
            Ok(adapter) => adapter,
            Err(error) => {
                log.log("err", error);
                return log.finish();
            }
        };
        let primary = match scan::parse_cidr(&format!("{}.0/24", primary_net.trim())) {
            Some(net) => net,
            None => {
                log.log("err", "Invalid primary network.".to_string());
                return log.finish();
            }
        };
        let alt = scan::parse_cidr(&format!("{}.0/24", alt_net.trim()));

        let local = adapter.primary_ip().cloned().unwrap_or_default();
        if local.parse::<Ipv4Addr>().is_ok_and(|value| !primary.contains(&value)) {
            log.log(
                "err",
                format!("Adapter has no address inside {primary}."),
            );
            return log.finish();
        }

        let used = scan::scan_net(adapter.interface_index, primary, 700);
        log.log(
            "info",
            format!("{} address(es) in use on {primary}.", used.len()),
        );

        let pick = (first_host..=254)
            .map(|host| Ipv4Addr::from(u32::from(primary.network()) + host))
            .find(|candidate| !used.contains_key(candidate));
        let Some(pick) = pick else {
            log.log("err", format!("No free address found in {primary}."));
            return log.finish();
        };
        log.log("ok", format!("First free IP: {pick}"));

        let mut ips = vec![(pick.to_string(), 24u8)];
        if let Some(alt) = alt
            && auto_alt
        {
            let alt_used = scan::scan_net(adapter.interface_index, alt, 700);
            if let Some(alt_ip) = (alt_start_host..=254u32)
                .map(|host| Ipv4Addr::from(u32::from(alt.network()) + host))
                .find(|candidate| !alt_used.contains_key(candidate))
            {
                log.log("ok", format!("Alternative IP: {alt_ip}"));
                ips.push((alt_ip.to_string(), 24u8));
            } else {
                log.log("warn", format!("No free address found in {alt}."));
            }
        }

        let gateway = adapter.gateways.first().cloned();
        let dns = (!adapter.dns_servers.is_empty()).then(|| adapter.dns_servers.clone());
        match wmi_connection() {
            Ok(connection) => log.report(
                "EnableStatic",
                "AUTO setup applied.",
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

#[tauri::command]
pub fn set_monitor_enabled(state: State<'_, MonitorState>, on: bool) -> Result<(), String> {
    state.send(MonitorCmd::SetEnabled(on))
}

#[tauri::command]
pub fn refresh_monitor(state: State<'_, MonitorState>) -> Result<(), String> {
    state.send(MonitorCmd::RefreshNow)
}
