//! AUTO setup: scan the primary network for a free address, then apply it as a
//! static address (plus an optional free address on the alternate network).

use std::net::Ipv4Addr;

use tauri::AppHandle;

use crate::models::LogLine;
use crate::win::{ip, scan};

use super::{Logger, find_adapter, wmi_connection};

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
