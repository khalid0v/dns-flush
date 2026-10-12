use std::net::{IpAddr, Ipv4Addr};
use std::sync::mpsc::{self, Receiver};

use eframe::egui;
use ipnet::Ipv4Net;

use crate::msg::{self, LogKind, Msg, Tx};
use crate::theme;
use crate::win::{AdapterInfo, WMIConnection, adapters, dns, ip, net, public_ip, reset, scan};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Flush,
    Register,
    Reset,
    Check,
    DnsLookup,
    SetDns,
    SetIp,
    Scan,
    Auto,
}

/// Draws the compact, line-icon navigation rail without relying on a platform
/// font or a raster asset. Every icon uses the same 1.8 point stroke.
fn draw_nav_icon(painter: &egui::Painter, rect: egui::Rect, tab: Tab, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.8, color);
    let center = rect.center();
    let left = rect.left();
    let right = rect.right();
    let top = rect.top();
    let bottom = rect.bottom();
    let line = |from, to| painter.line_segment([from, to], stroke);
    let dot = |at| painter.circle_filled(at, 1.8, color);

    match tab {
        Tab::Check => {
            line(egui::pos2(left, center.y), egui::pos2(center.x, top));
            line(egui::pos2(center.x, top), egui::pos2(right, center.y));
            line(
                egui::pos2(left + 2.0, center.y),
                egui::pos2(left + 2.0, bottom),
            );
            line(
                egui::pos2(right - 2.0, center.y),
                egui::pos2(right - 2.0, bottom),
            );
            line(
                egui::pos2(left + 2.0, bottom),
                egui::pos2(right - 2.0, bottom),
            );
            line(
                egui::pos2(center.x, bottom),
                egui::pos2(center.x, center.y + 2.0),
            );
        }
        Tab::DnsLookup => {
            painter.circle_stroke(center - egui::vec2(2.0, 2.0), 5.8, stroke);
            line(center + egui::vec2(2.0, 2.0), egui::pos2(right, bottom));
        }
        Tab::Scan => {
            painter.circle_stroke(center, 8.0, stroke);
            painter.circle_stroke(center, 3.0, stroke);
            dot(center);
            line(egui::pos2(center.x, top), egui::pos2(center.x, top + 3.0));
            line(
                egui::pos2(center.x, bottom - 3.0),
                egui::pos2(center.x, bottom),
            );
        }
        Tab::Flush => {
            for offset in [-5.0, 0.0, 5.0] {
                line(
                    egui::pos2(left + 2.0, center.y + offset),
                    egui::pos2(right - 3.0, center.y + offset),
                );
            }
            line(
                egui::pos2(right - 6.0, top + 1.0),
                egui::pos2(right - 1.0, top + 6.0),
            );
        }
        Tab::Register => {
            painter.circle_stroke(center, 8.0, stroke);
            line(
                egui::pos2(right - 1.0, center.y - 2.0),
                egui::pos2(right - 1.0, top + 1.0),
            );
            line(
                egui::pos2(right - 1.0, top + 1.0),
                egui::pos2(right - 5.0, top + 5.0),
            );
            line(
                egui::pos2(left + 1.0, center.y + 2.0),
                egui::pos2(left + 1.0, bottom - 1.0),
            );
            line(
                egui::pos2(left + 1.0, bottom - 1.0),
                egui::pos2(left + 5.0, bottom - 5.0),
            );
        }
        Tab::Reset => {
            painter.circle_stroke(center, 8.0, stroke);
            line(
                egui::pos2(center.x - 4.0, center.y - 4.0),
                egui::pos2(center.x + 4.0, center.y + 4.0),
            );
            line(
                egui::pos2(center.x + 4.0, center.y - 4.0),
                egui::pos2(center.x - 4.0, center.y + 4.0),
            );
        }
        Tab::SetDns => {
            for offset in [-6.0, 0.0, 6.0] {
                line(
                    egui::pos2(left + 2.0, center.y + offset),
                    egui::pos2(right - 2.0, center.y + offset),
                );
                dot(egui::pos2(left + 4.0, center.y + offset));
            }
        }
        Tab::SetIp => {
            for (x, y) in [
                (left + 3.0, top + 4.0),
                (right - 3.0, center.y),
                (left + 3.0, bottom - 4.0),
            ] {
                painter.circle_stroke(egui::pos2(x, y), 2.2, stroke);
            }
            line(
                egui::pos2(left + 5.0, top + 5.0),
                egui::pos2(right - 5.0, center.y - 1.0),
            );
            line(
                egui::pos2(right - 5.0, center.y + 1.0),
                egui::pos2(left + 5.0, bottom - 5.0),
            );
        }
        Tab::Auto => {
            painter.circle_stroke(center, 5.0, stroke);
            for (from, to) in [
                (egui::pos2(center.x, top), egui::pos2(center.x, top + 4.0)),
                (
                    egui::pos2(center.x, bottom - 4.0),
                    egui::pos2(center.x, bottom),
                ),
                (egui::pos2(left, center.y), egui::pos2(left + 4.0, center.y)),
                (
                    egui::pos2(right - 4.0, center.y),
                    egui::pos2(right, center.y),
                ),
            ] {
                line(from, to);
            }
            dot(center);
        }
    }
}

/// Recommended public DNS providers: display name + IPv4 servers.
const DNS_PRESETS: [(&str, &[&str]); 4] = [
    ("Google", &["8.8.8.8", "8.8.4.4"]),
    ("Cloudflare", &["1.1.1.1", "1.0.0.1"]),
    ("AdGuard", &["94.140.14.14", "94.140.15.15"]),
    ("Quad9", &["9.9.9.9", "149.112.112.112"]),
];

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

    lookup: String,
    scan_cidr: String,

    primary_net: String,
    alt_net: String,
    first_host: u32,
    alt_start_host: u32,
    auto_alt: bool,

    confirm_reset: bool,
    activity_visible: bool,
    dark: bool,
    theme_dirty: bool,
}

impl DnsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let dark = matches!(cc.egui_ctx.theme(), egui::Theme::Dark);
        theme::apply(&cc.egui_ctx, dark);

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
            lookup: "google.com".to_string(),
            scan_cidr: "192.168.1.0/24".to_string(),
            primary_net: "192.168.1".to_string(),
            alt_net: "172.16.0".to_string(),
            first_host: 11,
            alt_start_host: 200,
            auto_alt: true,
            confirm_reset: false,
            activity_visible: false,
            dark,
            theme_dirty: false,
        };
        app.refresh_adapters();
        app.refresh_public_ip();
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
                Msg::Adapters(list) => {
                    let previous = self.selected_index();
                    self.adapters = list;
                    self.selected = self
                        .adapters
                        .iter()
                        .position(|a| a.index == previous)
                        .unwrap_or(0);
                }
                Msg::Done => done = true,
            }
        }
        if done {
            self.busy = false;
            self.rx = None;
        }

        let public_ip_result = self.public_ip_rx.as_ref().and_then(|rx| rx.try_recv().ok());
        if let Some(result) = public_ip_result {
            self.public_ip_rx = None;
            self.public_ip_loading = false;
            match result {
                Ok(address) => {
                    self.public_ip = Some(address);
                    self.public_ip_error = None;
                }
                Err(error) => {
                    self.public_ip = None;
                    self.public_ip_error = Some(error);
                }
            }
        }
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
        self.refresh_adapters();
        self.refresh_public_ip();
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
        let Some(ad) = self.adapter() else {
            self.push_log(LogKind::Err, "No adapter selected.".into());
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

    fn act_check(&mut self) {
        let Some(adapter) = self.adapter() else {
            self.push_log(LogKind::Err, "No adapter selected.".into());
            return;
        };
        let host = self.lookup.trim().to_string();
        if host.is_empty() {
            self.push_log(LogKind::Err, "Enter a website name to check.".into());
            return;
        }
        self.spawn(move |tx| {
            msg::head(
                &tx,
                format!("DNS check for {host} on {}", adapter.description),
            );
            match dns::resolve(&host) {
                Ok(addresses) => {
                    msg::ok(&tx, format!("{host} resolved to:"));
                    for address in &addresses {
                        msg::info(&tx, format!("  {address}"));
                    }
                    for address in &addresses {
                        if let IpAddr::V4(address) = address {
                            match net::ping(*address, 1000) {
                                Some(milliseconds) => {
                                    msg::ok(
                                        &tx,
                                        format!("  {address} replied in {milliseconds} ms"),
                                    );
                                }
                                None => {
                                    msg::warn(&tx, format!("  {address} did not reply to ping"))
                                }
                            }
                        }
                    }
                }
                Err(error) => msg::err(&tx, format!("Could not resolve {host}: {error}")),
            }
        });
    }

    fn act_set_dns(&mut self) {
        let Some(ad) = self.adapter() else {
            self.push_log(LogKind::Err, "No adapter selected.".into());
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
        let Some(ad) = self.adapter() else {
            self.push_log(LogKind::Err, "No adapter selected.".into());
            return;
        };
        let servers: Vec<String> = servers.iter().map(|s| (*s).to_string()).collect();
        self.spawn(move |tx| {
            msg::head(&tx, format!("[5] Set DNS on {}", ad.description));
            set_dns_task(&tx, &ad, &servers);
        });
    }

    fn act_dns_auto(&mut self) {
        let Some(ad) = self.adapter() else {
            self.push_log(LogKind::Err, "No adapter selected.".into());
            return;
        };
        self.spawn(move |tx| {
            msg::head(
                &tx,
                format!("[5] Reset DNS to automatic on {}", ad.description),
            );
            with_wmi(&tx, |con| match dns::reset_servers(con, &ad) {
                Ok(0) => msg::ok(&tx, "DNS reset to automatic."),
                Ok(code) => msg::err(&tx, format!("Reset returned code {code}")),
                Err(e) => msg::err(&tx, format!("Reset failed: {e}")),
            });
        });
    }

    fn act_set_ip(&mut self) {
        let Some(ad) = self.adapter() else {
            self.push_log(LogKind::Err, "No adapter selected.".into());
            return;
        };
        let static_ip = self.static_ip;
        if !static_ip {
            self.spawn(move |tx| {
                msg::head(&tx, format!("[6] Switch {} to DHCP", ad.description));
                with_wmi(&tx, |con| match ip::enable_dhcp(con, &ad) {
                    Ok(0) => msg::ok(&tx, "Adapter now uses DHCP."),
                    Ok(code) => msg::err(&tx, format!("EnableDHCP returned code {code}")),
                    Err(e) => msg::err(&tx, format!("EnableDHCP failed: {e}")),
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
                match ip::set_static(con, &ad, &ips, gateway.as_deref(), dns.as_deref()) {
                    Ok(0) => msg::ok(&tx, "Static IP applied."),
                    Ok(code) => msg::err(&tx, format!("EnableStatic returned code {code}")),
                    Err(e) => msg::err(&tx, format!("Static IP failed: {e}")),
                }
            });
        });
    }

    fn act_scan(&mut self) {
        let Some(ad) = self.adapter() else {
            self.push_log(LogKind::Err, "No adapter selected.".into());
            return;
        };
        let Some(cidr) = scan::parse_cidr(&self.scan_cidr) else {
            self.push_log(LogKind::Err, format!("Invalid CIDR: {}", self.scan_cidr));
            return;
        };
        self.spawn(move |tx| {
            msg::head(&tx, format!("[7] Scan {}", cidr));
            report_scan(&tx, &ad, cidr);
        });
    }

    fn act_auto(&mut self) {
        let Some(ad) = self.adapter() else {
            self.push_log(LogKind::Err, "No adapter selected.".into());
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
                match ip::set_static(con, &ad, &ips, gateway.as_deref(), dns.as_deref()) {
                    Ok(0) => msg::ok(&tx, "AUTO setup applied."),
                    Ok(code) => msg::err(&tx, format!("EnableStatic returned code {code}")),
                    Err(e) => msg::err(&tx, format!("AUTO setup failed: {e}")),
                }
            });
        });
    }
}

fn report_scan(tx: &Tx, ad: &AdapterInfo, net: Ipv4Net) {
    let used = scan::scan_net(ad.interface_index, net, 700);
    let hosts: Vec<Ipv4Addr> = net.hosts().collect();
    let local = ad.primary_ip().and_then(|s| s.parse::<Ipv4Addr>().ok());

    let mut used_sorted: Vec<_> = used.iter().collect();
    used_sorted.sort_by_key(|(ip, _)| u32::from(**ip));
    msg::info(tx, format!("USED ({}):", used.len()));
    for (ip, mac) in used_sorted {
        let mut note = String::new();
        if Some(*ip) == local {
            note = "  <- this PC".into();
        } else if ad.gateways.iter().any(|g| g == &ip.to_string()) {
            note = "  <- gateway".into();
        }
        msg::info(tx, format!("  {ip:<15} {mac}{note}"));
    }

    let free: Vec<Ipv4Addr> = hosts
        .iter()
        .copied()
        .filter(|h| !used.contains_key(h))
        .collect();
    msg::ok(tx, format!("FREE: {} of {}", free.len(), hosts.len()));
    let candidates: Vec<_> = free
        .iter()
        .filter(|h| u32::from(**h) > u32::from(net.network()) + 10)
        .copied()
        .collect();
    let recommended = if candidates.is_empty() {
        &free
    } else {
        &candidates
    };
    for ip in recommended.iter().take(5) {
        msg::ok(tx, format!("  recommended {ip}"));
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
    with_wmi(tx, |con| match dns::set_servers(con, ad, servers) {
        Ok(0) => msg::ok(tx, format!("DNS set to {}", servers.join(", "))),
        Ok(code) => msg::err(tx, format!("SetDNS returned code {code}")),
        Err(e) => msg::err(tx, format!("SetDNS failed: {e}")),
    });
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
        .filter(|address| is_usable_ip(*address))
        .map(|address| address.to_string())
        .collect();

    if addresses.is_empty() {
        "No active IP address".to_string()
    } else {
        addresses.join(", ")
    }
}

fn is_usable_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            !address.is_unspecified()
                && !address.is_loopback()
                && !address.is_multicast()
                && !(address.octets()[0] == 169 && address.octets()[1] == 254)
        }
        IpAddr::V6(address) => {
            !address.is_unspecified()
                && !address.is_loopback()
                && !address.is_multicast()
                && !address.is_unicast_link_local()
        }
    }
}

fn vpn_status_label(adapters: &[AdapterInfo]) -> String {
    const VPN_MARKERS: [&str; 15] = [
        "vpn",
        "wireguard",
        "openvpn",
        "wintun",
        "tap-windows",
        "fortinet",
        "forticlient",
        "globalprotect",
        "anyconnect",
        "tailscale",
        "nordlynx",
        "protonvpn",
        "mullvad",
        "zerotier",
        "pulse secure",
    ];

    let active_vpn = adapters.iter().find(|adapter| {
        let description = adapter.description.to_ascii_lowercase();
        let looks_like_vpn = VPN_MARKERS
            .iter()
            .any(|marker| description.contains(marker));
        looks_like_vpn
            && adapter
                .ip_addresses
                .iter()
                .any(|address| address.parse::<IpAddr>().is_ok_and(is_usable_ip))
    });

    active_vpn
        .map(|adapter| format!("Detected ({})", adapter.description))
        .unwrap_or_else(|| "No known VPN adapter detected".to_string())
}

impl eframe::App for DnsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll();
        let ctx = ui.ctx().clone();
        if self.theme_dirty {
            theme::apply(&ctx, self.dark);
            self.theme_dirty = false;
        }
        if self.busy || self.public_ip_loading {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }

        egui::Panel::top("title").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new("Network support")
                        .color(theme::text(self.dark))
                        .size(22.0)
                        .strong(),
                );
                ui.add_space(8.0);
                let label = if self.busy { "Working…" } else { "Ready" };
                let color = if self.busy {
                    theme::warning(self.dark)
                } else {
                    theme::success(self.dark)
                };
                ui.label(egui::RichText::new(label).color(color));

                let activity_label = if self.activity_visible {
                    "Hide activity"
                } else {
                    "Show activity"
                };
                if ui.button(activity_label).clicked() {
                    self.activity_visible = !self.activity_visible;
                }

                let mode = if self.dark { "Light mode" } else { "Dark mode" };
                if ui.button(mode).on_hover_text("Switch appearance").clicked() {
                    self.dark = !self.dark;
                    self.theme_dirty = true;
                    ui.ctx().request_repaint();
                }
            });

            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label("Network adapter");
                let current = self
                    .adapters
                    .get(self.selected)
                    .map(AdapterInfo::title)
                    .unwrap_or_else(|| "No adapter found".to_string());
                let mut chosen = self.selected;
                let combo_width = (ui.available_width() - 112.0).max(120.0);
                egui::ComboBox::from_id_salt("adapter")
                    .selected_text(current)
                    .width(combo_width)
                    .show_ui(ui, |ui| {
                        for (i, adapter) in self.adapters.iter().enumerate() {
                            ui.selectable_value(&mut chosen, i, adapter.title());
                        }
                    });
                self.selected = chosen;
                if ui
                    .add_enabled(!self.busy, egui::Button::new("Refresh status"))
                    .clicked()
                {
                    self.refresh_status();
                }
            });
            ui.add_space(4.0);
        });

        if self.activity_visible {
            egui::Panel::bottom("log")
                .resizable(true)
                .default_size(140.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Activity")
                                .color(theme::secondary_text(self.dark))
                                .strong(),
                        );
                        if ui.button("Clear activity").clicked() {
                            self.log.clear();
                        }
                        if ui.button("Hide").clicked() {
                            self.activity_visible = false;
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (kind, text) in &self.log {
                                let color = match kind {
                                    LogKind::Info => theme::text(self.dark),
                                    LogKind::Ok => theme::success(self.dark),
                                    LogKind::Warn => theme::warning(self.dark),
                                    LogKind::Err => theme::destructive(self.dark),
                                    LogKind::Head => theme::accent(self.dark),
                                };
                                ui.label(egui::RichText::new(text).color(color).monospace());
                            }
                        });
                });
        }

        let sidebar = theme::sidebar_fill(self.dark);
        egui::Panel::left("actions")
            .resizable(false)
            .exact_size(68.0)
            .frame(
                egui::Frame::new()
                    .fill(sidebar)
                    .inner_margin(egui::Margin::symmetric(12, 8)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(2.0);
                        self.nav_icon_group(
                            ui,
                            &[
                                ("Network status", Tab::Check),
                                ("Check a website", Tab::DnsLookup),
                                ("Find devices", Tab::Scan),
                            ],
                        );
                        self.nav_icon_group(
                            ui,
                            &[
                                ("Flush DNS cache", Tab::Flush),
                                ("Renew connection", Tab::Register),
                                ("Reset network", Tab::Reset),
                            ],
                        );
                        self.nav_icon_group(
                            ui,
                            &[
                                ("Choose DNS servers", Tab::SetDns),
                                ("Set IP address", Tab::SetIp),
                                ("Automatic setup", Tab::Auto),
                            ],
                        );
                    });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(12.0);
                    match self.tab {
                        Tab::Flush => self.ui_flush(ui),
                        Tab::Register => self.ui_register(ui),
                        Tab::Reset => self.ui_reset(ui),
                        Tab::Check => self.ui_check(ui),
                        Tab::DnsLookup => self.ui_dns_lookup(ui),
                        Tab::SetDns => self.ui_set_dns(ui),
                        Tab::SetIp => self.ui_set_ip(ui),
                        Tab::Scan => self.ui_scan(ui),
                        Tab::Auto => self.ui_auto(ui),
                    }
                });
        });
    }
}

impl DnsApp {
    fn nav_icon_group(&mut self, ui: &mut egui::Ui, items: &[(&str, Tab)]) {
        for (label, tab) in items {
            self.nav_icon_button(ui, label, *tab);
        }
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);
    }

    fn nav_icon_button(&mut self, ui: &mut egui::Ui, label: &str, tab: Tab) {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::click());
        let response = response.on_hover_text(label);
        let selected = self.tab == tab;
        let fill = if selected {
            theme::accent(self.dark)
        } else if response.hovered() {
            ui.visuals().widgets.hovered.weak_bg_fill
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(9), fill);

        let color = if selected {
            egui::Color32::WHITE
        } else {
            theme::secondary_text(self.dark)
        };
        draw_nav_icon(ui.painter(), rect.shrink(10.0), tab, color);

        if response.clicked() {
            self.tab = tab;
            self.confirm_reset = false;
        }
    }

    fn heading(&self, ui: &mut egui::Ui, text: &str, sub: &str) {
        ui.label(
            egui::RichText::new(text)
                .color(theme::text(self.dark))
                .size(26.0)
                .strong(),
        );
        ui.label(egui::RichText::new(sub).color(theme::secondary_text(self.dark)));
        ui.add_space(8.0);
    }

    fn status_card(&self, ui: &mut egui::Ui, title: &str, value: &str, note: &str) {
        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.set_min_height(76.0);
                ui.label(
                    egui::RichText::new(title)
                        .color(theme::secondary_text(self.dark))
                        .strong(),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(value)
                        .color(theme::text(self.dark))
                        .size(18.0)
                        .strong(),
                );
                ui.label(egui::RichText::new(note).color(theme::secondary_text(self.dark)));
            });
    }

    fn ui_flush(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "Flush DNS cache",
            "Clear the Windows DNS resolver cache.",
        );
        if ui
            .add_enabled(!self.busy, egui::Button::new("Flush DNS cache"))
            .clicked()
        {
            self.act_flush();
        }
    }

    fn ui_register(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "Renew your connection",
            "Refresh the network address and register this PC with DNS.",
        );
        if ui
            .add_enabled(!self.busy, egui::Button::new("Renew connection"))
            .clicked()
        {
            self.act_register();
        }
    }

    fn ui_reset(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "Reset network components",
            "This may interrupt your connection. Restart the PC afterward.",
        );
        ui.label(
            egui::RichText::new("This changes system network state.")
                .color(theme::warning(self.dark)),
        );
        if !self.confirm_reset {
            if ui
                .add_enabled(!self.busy, egui::Button::new("Review reset"))
                .clicked()
            {
                self.confirm_reset = true;
            }
        } else {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !self.busy,
                        egui::Button::new(
                            egui::RichText::new("Confirm reset").color(egui::Color32::WHITE),
                        )
                        .fill(theme::destructive(self.dark)),
                    )
                    .clicked()
                {
                    self.act_reset();
                }
                if ui.button("Cancel").clicked() {
                    self.confirm_reset = false;
                }
            });
        }
    }

    fn ui_check(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "Network status",
            "Current details for the selected network adapter.",
        );

        let no_adapter = "No active adapter selected";
        let (dns, ip_address) = self
            .adapters
            .get(self.selected)
            .map(|adapter| (current_dns_label(adapter), current_ip_label(adapter)))
            .unwrap_or_else(|| (no_adapter.to_string(), no_adapter.to_string()));
        let public_ip = if self.public_ip_loading {
            "Checking…".to_string()
        } else {
            self.public_ip
                .map(|address| address.to_string())
                .unwrap_or_else(|| "Unavailable".to_string())
        };
        let public_ip_note = if self.public_ip_error.is_some() {
            "Could not reach the public IP service."
        } else {
            "Retrieved securely from ipify.org."
        };
        let vpn_status = vpn_status_label(&self.adapters);

        if ui.available_width() < 520.0 {
            self.status_card(ui, "Current DNS", &dns, "For the selected adapter");
            ui.add_space(12.0);
            self.status_card(
                ui,
                "Current IP address",
                &ip_address,
                "For the selected adapter",
            );
            ui.add_space(12.0);
            self.status_card(ui, "Public IP address", &public_ip, public_ip_note);
            ui.add_space(12.0);
            self.status_card(
                ui,
                "VPN status",
                &vpn_status,
                "Based on active adapter names; some VPNs may not be recognized.",
            );
        } else {
            ui.columns(2, |columns| {
                self.status_card(
                    &mut columns[0],
                    "Current DNS",
                    &dns,
                    "For the selected adapter",
                );
                self.status_card(
                    &mut columns[1],
                    "Current IP address",
                    &ip_address,
                    "For the selected adapter",
                );
                columns[0].add_space(12.0);
                columns[1].add_space(12.0);
                self.status_card(
                    &mut columns[0],
                    "Public IP address",
                    &public_ip,
                    public_ip_note,
                );
                self.status_card(
                    &mut columns[1],
                    "VPN status",
                    &vpn_status,
                    "Based on active adapter names; some VPNs may not be recognized.",
                );
            });
        }
    }

    fn ui_dns_lookup(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "Check a website",
            "See whether Windows can find the website and whether it replies to a ping.",
        );
        ui.horizontal(|ui| {
            ui.label("Website name");
            ui.text_edit_singleline(&mut self.lookup);
            if ui
                .add_enabled(!self.busy, egui::Button::new("Check website"))
                .clicked()
            {
                self.act_check();
            }
        });
    }

    fn ui_set_dns(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "Choose DNS servers",
            "Select a provider or enter the DNS addresses from your network administrator.",
        );
        ui.horizontal_wrapped(|ui| {
            for (name, servers) in DNS_PRESETS {
                let label = format!("{name}  {}", servers.join(" / "));
                if ui
                    .add_enabled(!self.busy, egui::Button::new(label))
                    .clicked()
                {
                    self.act_set_dns_preset(servers);
                }
            }
        });
        ui.add_space(6.0);
        ui.checkbox(&mut self.dns_manual, "Manual servers");
        ui.add_enabled_ui(self.dns_manual, |ui| {
            ui.horizontal(|ui| {
                ui.label("Preferred DNS");
                ui.text_edit_singleline(&mut self.dns1);
            });
            ui.horizontal(|ui| {
                ui.label("Alternate DNS");
                ui.text_edit_singleline(&mut self.dns2);
            });
            if ui
                .add_enabled(!self.busy, egui::Button::new("Apply DNS settings"))
                .clicked()
            {
                self.act_set_dns();
            }
        });
        ui.add_space(6.0);
        if ui
            .add_enabled(
                !self.busy,
                egui::Button::new("Reset to automatic (DHCP / AD)"),
            )
            .clicked()
        {
            self.act_dns_auto();
        }
    }

    fn ui_set_ip(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "Set an IP address",
            "Use automatic settings or enter the address provided by your network administrator.",
        );
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.static_ip, false, "Automatic (DHCP)");
            ui.radio_value(&mut self.static_ip, true, "Enter an address");
        });
        ui.add_space(6.0);
        ui.add_enabled_ui(self.static_ip, |ui| {
            egui::Grid::new("ipform").num_columns(2).show(ui, |ui| {
                ui.label("IP address");
                ui.text_edit_singleline(&mut self.ip_addr);
                ui.end_row();
                ui.label("Subnet mask");
                ui.text_edit_singleline(&mut self.mask);
                ui.end_row();
                ui.label("Gateway");
                ui.text_edit_singleline(&mut self.gateway);
                ui.end_row();
                ui.label("Preferred DNS");
                ui.text_edit_singleline(&mut self.ip_dns1);
                ui.end_row();
                ui.label("Alternate DNS");
                ui.text_edit_singleline(&mut self.ip_dns2);
                ui.end_row();
                ui.label("Alternate IP");
                ui.text_edit_singleline(&mut self.alt_ip);
                ui.end_row();
            });
        });
        ui.add_space(6.0);
        if ui
            .add_enabled(!self.busy, egui::Button::new("Apply IP settings"))
            .clicked()
        {
            self.act_set_ip();
        }
    }

    fn ui_scan(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "Find devices on your network",
            "Check a network you own or administer to see which addresses are in use.",
        );
        ui.horizontal(|ui| {
            ui.label("Network range");
            ui.text_edit_singleline(&mut self.scan_cidr);
        });
        if ui
            .add_enabled(!self.busy, egui::Button::new("Find devices"))
            .clicked()
        {
            self.act_scan();
        }
    }

    fn ui_auto(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "Automatic network setup",
            "Find an available address and configure this adapter for that network.",
        );
        egui::Grid::new("autoform").num_columns(2).show(ui, |ui| {
            ui.label("Primary network");
            ui.text_edit_singleline(&mut self.primary_net);
            ui.end_row();
            ui.label("Alternative network");
            ui.text_edit_singleline(&mut self.alt_net);
            ui.end_row();
            ui.label("Start checking at address");
            ui.add(egui::DragValue::new(&mut self.first_host).range(2..=254));
            ui.end_row();
            ui.label("Alternative start address");
            ui.add(egui::DragValue::new(&mut self.alt_start_host).range(2..=254));
            ui.end_row();
        });
        ui.checkbox(
            &mut self.auto_alt,
            "Also add an address for the alternative network",
        );
        ui.add_space(6.0);
        if ui
            .add_enabled(!self.busy, egui::Button::new("Start automatic setup"))
            .clicked()
        {
            self.act_auto();
        }
    }
}
