//! DNS repair commands: flush the resolver cache, set or reset DNS servers.

use std::net::Ipv4Addr;

use tauri::AppHandle;

use crate::models::LogLine;
use crate::win::{dns, reset};

use super::{Logger, find_adapter, wmi_connection};

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
