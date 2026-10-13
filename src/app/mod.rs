//! Application state and behaviour: the `DnsApp` struct, its lifecycle, the
//! background worker bridge, and the action methods that run repairs.
//!
//! Rendering lives in `view.rs`; the sidebar navigation model lives in `nav.rs`.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::monitor;
use crate::msg::{self, LogKind, MonitorCmd, Msg, ScanResult, StatusMsg, Tx};
use crate::win::{AdapterInfo, WMIConnection, adapters, dns, ip, public_ip, reset, scan, watch};

mod nav;
mod view;

use nav::Tab;

/// Recommended public DNS providers: display name + IPv4 servers.
const DNS_PRESETS: [(&str, &[&str]); 4] = [
    ("Google", &["8.8.8.8", "8.8.4.4"]),
    ("Cloudflare", &["1.1.1.1", "1.0.0.1"]),
    ("AdGuard", &["94.140.14.14", "94.140.15.15"]),
    ("Quad9", &["9.9.9.9", "149.112.112.112"]),
];

/// How long a "Network changed" toast stays on screen.
const TOAST_LIFETIME: Duration = Duration::from_secs(6);

pub struct DnsApp {
    adapters: Vec<AdapterInfo>,
    selected: usize,
    rx: Option<Receiver<Msg>>,
    public_ip_rx: Option<Receiver<Result<IpAddr, String>>>,
    public_ip: Option<IpAddr>,
    public_ip_loading: bool,
    public_ip_error: Option<String>,
    busy: bool,
    log: Vec<(LogKind, String)>,
    tab: Tab,

    dns_manual: bool,
    dns1: String,
    dns2: String,

    static_ip: bool,
    ip_addr: String,
    mask: String,
    gateway: String,
    ip_dns1: String,
    ip_dns2: String,
    alt_ip: String,

    scan_cidr: String,
    scan_result: Option<ScanResult>,

    primary_net: String,
    alt_net: String,
    first_host: u32,
    alt_start_host: u32,
    auto_alt: bool,

    confirm_reset: bool,
    activity_visible: bool,
    dark: bool,
    theme_dirty: bool,

    monitor_tx: Sender<MonitorCmd>,
    monitor_rx: Receiver<StatusMsg>,
    monitor_on: bool,
    previous_adapters: Option<Vec<AdapterInfo>>,
    previous_public_ip: Option<IpAddr>,
    toast: Option<(String, Instant)>,
}

impl DnsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let dark = matches!(cc.egui_ctx.theme(), egui::Theme::Dark);
        crate::theme::apply(&cc.egui_ctx, dark);

        let repaint = cc.egui_ctx.clone();
        let (monitor_tx, monitor_rx) = watch::start(move || repaint.request_repaint());

        let mut app = Self {
            adapters: Vec::new(),
            selected: 0,
            rx: None,
            public_ip_rx: None,
            public_ip: None,
            public_ip_loading: true,
            public_ip_error: None,
            busy: false,
            log: Vec::new(),
            tab: Tab::Check,
            dns_manual: false,
            dns1: String::new(),
            dns2: String::new(),
            static_ip: true,
            ip_addr: String::new(),
            mask: "255.255.255.0".to_string(),
            gateway: String::new(),
            ip_dns1: String::new(),
            ip_dns2: String::new(),
            alt_ip: String::new(),
            scan_cidr: "192.168.1.0/24".to_string(),
            scan_result: None,
            primary_net: "192.168.1".to_string(),
            alt_net: "172.16.0".to_string(),
            first_host: 11,
            alt_start_host: 200,
            auto_alt: true,
            confirm_reset: false,
            activity_visible: false,
            dark,
            theme_dirty: false,
            monitor_tx,
            monitor_rx,
            monitor_on: false,
            previous_adapters: None,
            previous_public_ip: None,
            toast: None,
        };
        app.set_monitor(true);
        app
    }

    fn push_log(&mut self, kind: LogKind, text: String) {
        self.log.push((kind, text));
        if self.log.len() > 4000 {
            self.log.drain(0..1000);
        }
    }

    fn spawn<F>(&mut self, f: F)
    where
        F: FnOnce(Tx) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.busy = true;
        self.activity_visible = true;
        std::thread::spawn(move || {
            f(tx.clone());
            let _ = tx.send(Msg::Done);
        });
    }

    fn poll(&mut self) {
        let mut messages = Vec::new();
        if let Some(rx) = &self.rx {
            while let Ok(message) = rx.try_recv() {
                messages.push(message);
            }
        }
        let mut done = false;
        for message in messages {
            match message {
                Msg::Log(kind, text) => self.push_log(kind, text),
                Msg::Adapters(list) => self.apply_adapters(list),
                Msg::ScanResult(result) => self.scan_result = Some(result),
                Msg::Done => done = true,
            }
        }
        if done {
            self.busy = false;
            self.rx = None;
            if self.monitor_on {
                let _ = self.monitor_tx.send(MonitorCmd::RefreshNow);
            }
        }

        let public_ip_result = self.public_ip_rx.as_ref().and_then(|rx| rx.try_recv().ok());
        if let Some(result) = public_ip_result {
            self.public_ip_rx = None;
            self.apply_public_ip(result);
        }

        let mut status = Vec::new();
        while let Ok(message) = self.monitor_rx.try_recv() {
            status.push(message);
        }
        for message in status {
            if self.busy {
                continue;
            }
            match message {
                StatusMsg::Adapters(list) => self.apply_snapshot(list),
                StatusMsg::PublicIp(result) => self.apply_public_ip(result),
            }
        }
    }

    /// Replace the adapter list while keeping the current selection when the
    /// adapter is still present.
    fn apply_adapters(&mut self, list: Vec<AdapterInfo>) {
        let previous = self.selected_index();
        self.adapters = list;
        let keep = self.adapters.iter().position(|a| a.index == previous);
        self.selected = keep.unwrap_or_else(|| preferred_adapter(&self.adapters));
    }

    /// Apply a live monitor snapshot, logging and surfacing any changes. The
    /// first snapshot after enabling establishes a silent baseline.
    fn apply_snapshot(&mut self, list: Vec<AdapterInfo>) {
        if let Some(previous) = &self.previous_adapters {
            let changes = monitor::describe_changes(previous, &list);
            if !changes.is_empty() {
                for change in &changes {
                    self.push_log(LogKind::Info, change.clone());
                }
                self.toast = Some((monitor::summarize(&changes), Instant::now()));
            }
        }
        self.previous_adapters = Some(list.clone());
        self.apply_adapters(list);
    }

    /// Record a public-IP lookup result, logging when the address changed.
    fn apply_public_ip(&mut self, result: Result<IpAddr, String>) {
        self.public_ip_loading = false;
        match result {
            Ok(address) => {
                if let Some(previous) = self.previous_public_ip
                    && previous != address
                {
                    self.push_log(
                        LogKind::Info,
                        format!("Public IP changed: {previous} -> {address}"),
                    );
                }
                self.previous_public_ip = Some(address);
                self.public_ip = Some(address);
                self.public_ip_error = None;
            }
            Err(error) => {
                self.public_ip = None;
                self.public_ip_error = Some(error);
            }
        }
    }

    fn set_monitor(&mut self, on: bool) {
        if self.monitor_on == on {
            return;
        }
        self.monitor_on = on;
        self.previous_adapters = None;
        let _ = self.monitor_tx.send(MonitorCmd::SetEnabled(on));
    }

    fn selected_index(&self) -> u32 {
        self.adapters
            .get(self.selected)
            .map(|a| a.index)
            .unwrap_or(0)
    }

    fn adapter(&self) -> Option<AdapterInfo> {
        self.adapters.get(self.selected).cloned()
    }

    /// The selected adapter, logging an error when there is none so action
    /// methods can bail with a single `let ... else`.
    fn adapter_or_report_missing(&mut self) -> Option<AdapterInfo> {
        match self.adapter() {
            Some(adapter) => Some(adapter),
            None => {
                self.push_log(LogKind::Err, "No adapter selected.".into());
                None
            }
        }
    }

    fn refresh_adapters(&mut self) {
        self.spawn(|tx| {
            msg::info(&tx, "Enumerating network adapters...");
            match adapters::connect().and_then(|con| adapters::list(&con)) {
                Ok(list) => {
                    msg::ok(&tx, format!("{} adapter(s) found.", list.len()));
                    let _ = tx.send(Msg::Adapters(list));
                }
                Err(e) => msg::err(&tx, format!("Adapter enumeration failed: {e}")),
            }
        });
    }

    fn refresh_public_ip(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.public_ip_rx = Some(rx);
        self.public_ip = None;
        self.public_ip_error = None;
        self.public_ip_loading = true;
        std::thread::spawn(move || {
            let result = public_ip::lookup().map_err(|error| error.to_string());
            let _ = tx.send(result);
        });
    }

    fn refresh_status(&mut self) {
        if self.monitor_on {
            let _ = self.monitor_tx.send(MonitorCmd::RefreshNow);
        } else {
            self.refresh_adapters();
            self.refresh_public_ip();
        }
    }

    fn act_flush(&mut self) {
        self.spawn(|tx| {
            msg::head(&tx, "[1] Flush DNS resolver cache");
            if dns::flush() {
                msg::ok(&tx, "DNS cache flushed.");
            } else {
                msg::warn(
                    &tx,
                    "Native flush failed, falling back to ipconfig /flushdns",
                );
                match reset::flush_dns_fallback() {
                    Ok(lines) => lines.iter().for_each(|l| msg::info(&tx, l)),
                    Err(e) => msg::err(&tx, format!("Flush failed: {e}")),
                }
            }
        });
    }

    fn act_register(&mut self) {
        let Some(ad) = self.adapter_or_report_missing() else {
            return;
        };
        self.spawn(move |tx| {
            msg::head(&tx, "[2] Register DNS and renew IP");
            match reset::register_dns() {
                Ok(lines) => lines.iter().for_each(|l| msg::info(&tx, l)),
                Err(e) => msg::err(&tx, format!("registerdns failed: {e}")),
            }
            with_wmi(&tx, |con| {
                match ip::release_lease(con, &ad) {
                    Ok(code) => msg::info(&tx, format!("ReleaseDHCPLease -> {code}")),
                    Err(e) => msg::err(&tx, format!("Release failed: {e}")),
                }
                match ip::renew_lease(con, &ad) {
                    Ok(code) => msg::ok(&tx, format!("RenewDHCPLease -> {code}")),
                    Err(e) => msg::err(&tx, format!("Renew failed: {e}")),
                }
            });
        });
    }

    fn act_reset(&mut self) {
        self.confirm_reset = false;
        self.spawn(|tx| {
            msg::head(&tx, "[3] Reset Winsock and TCP/IP stack");
            msg::warn(&tx, "Resetting Winsock...");
            match reset::winsock_reset() {
                Ok(lines) => lines.iter().for_each(|l| msg::info(&tx, l)),
                Err(e) => msg::err(&tx, format!("winsock reset failed: {e}")),
            }
            msg::warn(&tx, "Resetting TCP/IP stack...");
            match reset::ip_reset() {
                Ok(lines) => lines.iter().for_each(|l| msg::info(&tx, l)),
                Err(e) => msg::err(&tx, format!("ip reset failed: {e}")),
            }
            msg::ok(&tx, "Reset complete. Restart the computer to finish.");
        });
    }

    fn act_set_dns(&mut self) {
        let Some(ad) = self.adapter_or_report_missing() else {
            return;
        };
        if !self.dns_manual {
            self.push_log(
                LogKind::Info,
                "Use preset radio options below, or tick 'Manual' to enter servers.".into(),
            );
            return;
        }
        let dns1 = self.dns1.trim().to_string();
        let dns2 = self.dns2.trim().to_string();
        self.spawn(move |tx| {
            msg::head(&tx, format!("[5] Set DNS on {}", ad.description));
            if dns1.parse::<Ipv4Addr>().is_err() {
                msg::err(&tx, format!("Invalid preferred DNS: {dns1}"));
                return;
            }
            let mut servers = vec![dns1];
            if !dns2.is_empty() {
                servers.push(dns2);
            }
            set_dns_task(&tx, &ad, &servers);
        });
    }

    fn act_set_dns_preset(&mut self, servers: &[&str]) {
        let Some(ad) = self.adapter_or_report_missing() else {
            return;
        };
        let servers: Vec<String> = servers.iter().map(|s| (*s).to_string()).collect();
        self.spawn(move |tx| {
            msg::head(&tx, format!("[5] Set DNS on {}", ad.description));
            set_dns_task(&tx, &ad, &servers);
        });
    }

    fn act_dns_auto(&mut self) {
        let Some(ad) = self.adapter_or_report_missing() else {
            return;
        };
        self.spawn(move |tx| {
            msg::head(
                &tx,
                format!("[5] Reset DNS to automatic on {}", ad.description),
            );
            with_wmi(&tx, |con| {
                report_status(
                    &tx,
                    "Reset",
                    "DNS reset to automatic.",
                    dns::reset_servers(con, &ad),
                );
            });
        });
    }

    fn act_set_ip(&mut self) {
        let Some(ad) = self.adapter_or_report_missing() else {
            return;
        };
        let static_ip = self.static_ip;
        if !static_ip {
            self.spawn(move |tx| {
                msg::head(&tx, format!("[6] Switch {} to DHCP", ad.description));
                with_wmi(&tx, |con| {
                    report_status(
                        &tx,
                        "EnableDHCP",
                        "Adapter now uses DHCP.",
                        ip::enable_dhcp(con, &ad),
                    );
                });
            });
            return;
        }

        let ip_addr = self.ip_addr.trim().to_string();
        let mask = self.mask.trim().to_string();
        let gateway = self.gateway.trim().to_string();
        let dns1 = self.ip_dns1.trim().to_string();
        let dns2 = self.ip_dns2.trim().to_string();
        let alt = self.alt_ip.trim().to_string();

        let Some(prefix) = ip::parse_mask(&mask) else {
            self.push_log(LogKind::Err, format!("Invalid mask: {mask}"));
            return;
        };
        if ip_addr.parse::<Ipv4Addr>().is_err() {
            self.push_log(LogKind::Err, format!("Invalid IP address: {ip_addr}"));
            return;
        }
        let mut ips = vec![(ip_addr, prefix)];
        if !alt.is_empty() {
            if alt.parse::<Ipv4Addr>().is_err() {
                self.push_log(LogKind::Err, format!("Invalid alternate IP: {alt}"));
                return;
            }
            ips.push((alt, prefix));
        }
        let gateway = (!gateway.is_empty()).then_some(gateway);
        let mut dns_servers = Vec::new();
        if !dns1.is_empty() {
            dns_servers.push(dns1);
        }
        if !dns2.is_empty() {
            dns_servers.push(dns2);
        }
        let dns = (!dns_servers.is_empty()).then_some(dns_servers);

        self.spawn(move |tx| {
            msg::head(&tx, format!("[6] Apply static IP to {}", ad.description));
            for (a, p) in &ips {
                msg::info(&tx, format!("  address {a}/{p}"));
            }
            with_wmi(&tx, |con| {
                report_status(
                    &tx,
                    "EnableStatic",
                    "Static IP applied.",
                    ip::set_static(con, &ad, &ips, gateway.as_deref(), dns.as_deref()),
                );
            });
        });
    }

    /// Point the scan range at the selected adapter's own subnet and drop any
    /// stale result from a previous network.
    fn sync_scan_range(&mut self) {
        if let Some(ad) = self.adapter()
            && let Some(cidr) = ad.primary_cidr()
        {
            self.scan_cidr = cidr.to_string();
        }
        self.scan_result = None;
    }

    fn act_scan(&mut self) {
        let Some(ad) = self.adapter_or_report_missing() else {
            return;
        };
        let Some(cidr) = scan::parse_cidr(&self.scan_cidr) else {
            self.push_log(LogKind::Err, format!("Invalid CIDR: {}", self.scan_cidr));
            return;
        };
        self.scan_result = None;
        self.spawn(move |tx| {
            msg::head(&tx, format!("[7] Scan {}", cidr));
            let result = scan::scan_result(ad.interface_index, cidr, &ad);
            let _ = tx.send(Msg::ScanResult(result));
        });
    }

    fn act_auto(&mut self) {
        let Some(ad) = self.adapter_or_report_missing() else {
            return;
        };
        let Some(primary) = scan::parse_cidr(&format!("{}.0/24", self.primary_net.trim())) else {
            self.push_log(LogKind::Err, "Invalid primary network.".into());
            return;
        };
        let alt = scan::parse_cidr(&format!("{}.0/24", self.alt_net.trim()));
        let first_host = self.first_host;
        let alt_start = self.alt_start_host;
        let auto_alt = self.auto_alt;

        self.spawn(move |tx| {
            msg::head(&tx, "[8] AUTO setup");
            let local = ad.primary_ip().cloned().unwrap_or_default();
            if local
                .parse::<Ipv4Addr>()
                .is_ok_and(|v| !primary.contains(&v))
            {
                msg::err(&tx, format!("Adapter has no address inside {primary}."));
                return;
            }

            let used = scan::scan_net(ad.interface_index, primary, 700);
            msg::info(
                &tx,
                format!("{} address(es) in use on {primary}.", used.len()),
            );

            let pick = (first_host..=254)
                .map(|h| Ipv4Addr::from(u32::from(primary.network()) + h))
                .find(|c| !used.contains_key(c));
            let Some(pick) = pick else {
                msg::err(&tx, format!("No free address found in {primary}."));
                return;
            };
            msg::ok(&tx, format!("First free IP: {pick}"));

            let mut ips = vec![(pick.to_string(), 24u8)];
            if let Some(alt) = alt
                && auto_alt
            {
                let alt_used = scan::scan_net(ad.interface_index, alt, 700);
                if let Some(alt_ip) = (alt_start..=254u32)
                    .map(|h| Ipv4Addr::from(u32::from(alt.network()) + h))
                    .find(|c| !alt_used.contains_key(c))
                {
                    msg::ok(&tx, format!("Alternative IP: {alt_ip}"));
                    ips.push((alt_ip.to_string(), 24u8));
                } else {
                    msg::warn(&tx, format!("No free address found in {alt}."));
                }
            }

            let gateway = ad.gateways.first().cloned();
            let dns = (!ad.dns_servers.is_empty()).then(|| ad.dns_servers.clone());

            with_wmi(&tx, |con| {
                report_status(
                    &tx,
                    "EnableStatic",
                    "AUTO setup applied.",
                    ip::set_static(con, &ad, &ips, gateway.as_deref(), dns.as_deref()),
                );
            });
        });
    }
}

/// Run `action` with a fresh WMI connection. `WMIConnection` is not `Send`, so
/// each worker thread opens its own connection here.
fn with_wmi(tx: &Tx, action: impl FnOnce(&WMIConnection)) {
    match adapters::connect() {
        Ok(con) => action(&con),
        Err(e) => msg::err(tx, format!("WMI error: {e}")),
    }
}

/// Apply a DNS server list to `ad`, reporting the Win32 return code.
fn set_dns_task(tx: &Tx, ad: &AdapterInfo, servers: &[String]) {
    let success = format!("DNS set to {}", servers.join(", "));
    with_wmi(tx, |con| {
        report_status(tx, "SetDNS", &success, dns::set_servers(con, ad, servers));
    });
}

/// Log the outcome of a Win32 call that reports a status code (`0` is success).
fn report_status(tx: &Tx, operation: &str, success: &str, result: anyhow::Result<u32>) {
    match result {
        Ok(0) => msg::ok(tx, success),
        Ok(code) => msg::err(tx, format!("{operation} returned code {code}")),
        Err(error) => msg::err(tx, format!("{operation} failed: {error}")),
    }
}

/// Index of the adapter most likely to be the live connection: the first with
/// a real (non-APIPA) address and a default gateway, else the first with a real
/// address, else the first in the list.
fn preferred_adapter(adapters: &[AdapterInfo]) -> usize {
    adapters
        .iter()
        .position(|ad| ad.primary_ip().is_some() && !ad.gateways.is_empty())
        .or_else(|| adapters.iter().position(|ad| ad.primary_ip().is_some()))
        .unwrap_or(0)
}

/// Human-readable summary of an adapter's current DNS servers.
fn current_dns_label(ad: &AdapterInfo) -> String {
    if ad.dns_servers.is_empty() {
        if ad.dhcp_enabled {
            "Automatic (DHCP / AD)".to_string()
        } else {
            "Not set".to_string()
        }
    } else {
        ad.dns_servers.join(", ")
    }
}

fn current_ip_label(ad: &AdapterInfo) -> String {
    let addresses: Vec<String> = ad
        .ip_addresses
        .iter()
        .filter_map(|address| address.parse::<IpAddr>().ok())
        .filter(|address| monitor::is_usable_ip(*address))
        .map(|address| address.to_string())
        .collect();

    if addresses.is_empty() {
        "No active IP address".to_string()
    } else {
        addresses.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter(description: &str, dns: &[&str], dhcp_enabled: bool, ips: &[&str]) -> AdapterInfo {
        AdapterInfo {
            index: 1,
            interface_index: 1,
            description: description.to_string(),
            ip_addresses: ips.iter().map(|s| (*s).to_string()).collect(),
            ip_subnets: Vec::new(),
            gateways: Vec::new(),
            dns_servers: dns.iter().map(|s| (*s).to_string()).collect(),
            dhcp_enabled,
        }
    }

    fn adapter_with_gateways(description: &str, ips: &[&str], gateways: &[&str]) -> AdapterInfo {
        AdapterInfo {
            index: 1,
            interface_index: 1,
            description: description.to_string(),
            ip_addresses: ips.iter().map(|s| (*s).to_string()).collect(),
            ip_subnets: Vec::new(),
            gateways: gateways.iter().map(|s| (*s).to_string()).collect(),
            dns_servers: Vec::new(),
            dhcp_enabled: true,
        }
    }

    fn adapter_with_subnet(
        description: &str,
        ips: &[&str],
        subnets: &[&str],
        gateways: &[&str],
    ) -> AdapterInfo {
        AdapterInfo {
            index: 1,
            interface_index: 1,
            description: description.to_string(),
            ip_addresses: ips.iter().map(|s| (*s).to_string()).collect(),
            ip_subnets: subnets.iter().map(|s| (*s).to_string()).collect(),
            gateways: gateways.iter().map(|s| (*s).to_string()).collect(),
            dns_servers: Vec::new(),
            dhcp_enabled: true,
        }
    }

    #[test]
    fn dns_label_prefers_configured_servers() {
        let ad = adapter("Ethernet", &["1.1.1.1", "1.0.0.1"], true, &[]);
        assert_eq!(current_dns_label(&ad), "1.1.1.1, 1.0.0.1");
    }

    #[test]
    fn dns_label_reports_automatic_dhcp_without_servers() {
        let ad = adapter("Ethernet", &[], true, &[]);
        assert_eq!(current_dns_label(&ad), "Automatic (DHCP / AD)");
    }

    #[test]
    fn dns_label_reports_not_set_for_static_without_servers() {
        let ad = adapter("Ethernet", &[], false, &[]);
        assert_eq!(current_dns_label(&ad), "Not set");
    }

    #[test]
    fn ip_label_keeps_only_usable_addresses() {
        let ad = adapter(
            "Ethernet",
            &[],
            true,
            &["169.254.10.1", "127.0.0.1", "192.168.1.20"],
        );
        assert_eq!(current_ip_label(&ad), "192.168.1.20");
    }

    #[test]
    fn ip_label_reports_when_no_usable_address() {
        let ad = adapter("Ethernet", &[], true, &["169.254.10.1"]);
        assert_eq!(current_ip_label(&ad), "No active IP address");
    }

    #[test]
    fn report_status_maps_return_codes_to_log_lines() {
        let (tx, rx) = mpsc::channel();
        report_status(&tx, "SetDNS", "applied", Ok(0));
        report_status(&tx, "SetDNS", "applied", Ok(5));
        report_status(&tx, "SetDNS", "applied", Err(anyhow::anyhow!("boom")));

        let logs: Vec<(LogKind, String)> = std::iter::from_fn(|| rx.try_recv().ok())
            .map(|message| match message {
                Msg::Log(kind, text) => (kind, text),
                _ => unreachable!("report_status only sends log messages"),
            })
            .collect();

        assert_eq!(
            logs,
            vec![
                (LogKind::Ok, "applied".to_string()),
                (LogKind::Err, "SetDNS returned code 5".to_string()),
                (LogKind::Err, "SetDNS failed: boom".to_string()),
            ]
        );
    }

    #[test]
    fn preferred_adapter_picks_one_with_ip_and_gateway() {
        let adapters = vec![
            adapter_with_gateways("VMnet1", &["192.168.126.1"], &[]),
            adapter_with_gateways("Intel", &["172.16.0.200"], &["172.16.0.1"]),
        ];
        assert_eq!(preferred_adapter(&adapters), 1);
    }

    #[test]
    fn preferred_adapter_falls_back_to_ip_only() {
        let adapters = vec![
            adapter_with_gateways("VMnet1", &["192.168.126.1"], &[]),
            adapter_with_gateways("VPN", &["26.3.195.207"], &[]),
        ];
        assert_eq!(preferred_adapter(&adapters), 0);
    }

    #[test]
    fn preferred_adapter_skips_apipa() {
        let adapters = vec![
            adapter_with_gateways("Broken", &["169.254.10.1"], &["169.254.10.1"]),
            adapter_with_gateways("Intel", &["172.16.0.200"], &["172.16.0.1"]),
        ];
        assert_eq!(preferred_adapter(&adapters), 1);
    }

    #[test]
    fn preferred_adapter_defaults_to_first_when_nothing_usable() {
        let adapters = vec![
            adapter_with_gateways("Broken", &["169.254.10.1"], &[]),
            adapter_with_gateways("Empty", &[], &[]),
        ];
        assert_eq!(preferred_adapter(&adapters), 0);
    }

    #[test]
    fn primary_cidr_combines_address_and_subnet() {
        let ad = adapter_with_subnet(
            "Ethernet",
            &["172.16.0.200"],
            &["255.255.255.0"],
            &["172.16.0.1"],
        );
        assert_eq!(ad.primary_cidr().unwrap().to_string(), "172.16.0.0/24");
    }

    #[test]
    fn primary_cidr_ignores_apipa() {
        let ad = adapter_with_subnet("Broken", &["169.254.10.1"], &["255.255.0.0"], &[]);
        assert!(ad.primary_cidr().is_none());
    }

    #[test]
    fn subnet_for_matches_the_address() {
        let ad = adapter_with_subnet(
            "Ethernet",
            &["10.0.0.5", "192.168.1.9"],
            &["255.0.0.0", "255.255.255.0"],
            &[],
        );
        assert_eq!(ad.subnet_for("10.0.0.5"), Some("255.0.0.0"));
        assert_eq!(ad.subnet_for("192.168.1.9"), Some("255.255.255.0"));
        assert_eq!(ad.subnet_for("8.8.8.8"), None);
    }
}
