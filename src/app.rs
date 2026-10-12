use std::net::{IpAddr, Ipv4Addr};
use std::sync::mpsc::{self, Receiver};

use eframe::egui;
use ipnet::Ipv4Net;

use crate::msg::{self, LogKind, Msg, Tx};
use crate::theme;
use crate::win::{AdapterInfo, WMIConnection, adapters, dns, ip, net, reset, scan};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Flush,
    Register,
    Reset,
    Check,
    SetDns,
    SetIp,
    Scan,
    Auto,
}

const TABS: [(&str, Tab); 8] = [
    ("[1] Flush DNS", Tab::Flush),
    ("[2] Register / renew", Tab::Register),
    ("[3] Reset Winsock/TCP", Tab::Reset),
    ("[4] Check DNS", Tab::Check),
    ("[5] Set DNS servers", Tab::SetDns),
    ("[6] Static IP / DHCP", Tab::SetIp),
    ("[7] Scan subnet", Tab::Scan),
    ("[8] AUTO setup", Tab::Auto),
];

/// Recommended public DNS providers: display name + IPv4 servers.
const DNS_PRESETS: [(&str, &[&str]); 4] = [
    ("Google", &["8.8.8.8", "8.8.4.4"]),
    ("Cloudflare", &["1.1.1.1", "1.0.0.1"]),
    ("AdGuard", &["94.140.14.14", "94.140.15.15"]),
    ("Quad9", &["9.9.9.9", "149.112.112.112"]),
];

/// A DNS change the user chose but has not applied yet.
#[derive(Clone)]
enum DnsPlan {
    /// Let DHCP / Active Directory provide the servers.
    Automatic,
    Servers(Vec<String>),
}

pub struct DnsApp {
    adapters: Vec<AdapterInfo>,
    selected: usize,
    rx: Option<Receiver<Msg>>,
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
    pending_dns: Option<DnsPlan>,

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
            busy: false,
            log: Vec::new(),
            tab: Tab::Flush,
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
            pending_dns: None,
            dark,
            theme_dirty: false,
        };
        app.refresh_adapters();
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
        let Some(ad) = self.adapter() else {
            self.push_log(LogKind::Err, "No adapter selected.".into());
            return;
        };
        let host = self.lookup.clone();
        self.spawn(move |tx| {
            msg::head(&tx, format!("[4] DNS check for {}", ad.description));
            if ad.dns_servers.is_empty() {
                msg::info(&tx, "DNS is automatic (DHCP / AD) - no static servers set.");
            } else {
                msg::info(
                    &tx,
                    format!("Configured DNS: {}", ad.dns_servers.join(", ")),
                );
            }
            match dns::resolve(&host) {
                Ok(ips) => {
                    msg::ok(&tx, format!("Resolved {host}:"));
                    for ipaddr in &ips {
                        msg::info(&tx, format!("  {ipaddr}"));
                    }
                    for ipaddr in &ips {
                        if let IpAddr::V4(v4) = ipaddr {
                            match net::ping(*v4, 1000) {
                                Some(ms) => msg::ok(&tx, format!("  ping {v4} -> {ms} ms")),
                                None => msg::warn(&tx, format!("  ping {v4} -> no reply")),
                            }
                        }
                    }
                }
                Err(e) => msg::err(&tx, format!("Failed to resolve {host}: {e}")),
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

    fn act_apply_dns_plan(&mut self) {
        match self.pending_dns.take() {
            Some(DnsPlan::Automatic) => self.act_dns_auto(),
            Some(DnsPlan::Servers(servers)) => {
                let refs: Vec<&str> = servers.iter().map(String::as_str).collect();
                self.act_set_dns_preset(&refs);
            }
            None => {}
        }
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

impl eframe::App for DnsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll();
        let ctx = ui.ctx().clone();
        if self.theme_dirty {
            theme::apply(&ctx, self.dark);
            self.theme_dirty = false;
        }
        if self.busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }

        egui::Panel::top("title").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("DNS FLUSH")
                        .color(theme::text(self.dark))
                        .size(24.0)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new("// network toolkit")
                        .color(theme::secondary_text(self.dark)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = if self.busy { "WORKING..." } else { "IDLE" };
                    let color = if self.busy {
                        theme::warning(self.dark)
                    } else {
                        theme::success(self.dark)
                    };
                    ui.label(egui::RichText::new(label).color(color));
                    let mode = if self.dark { "Light mode" } else { "Dark mode" };
                    if ui.button(mode).on_hover_text("Switch appearance").clicked() {
                        self.dark = !self.dark;
                        self.theme_dirty = true;
                        ui.ctx().request_repaint();
                    }
                });
            });
            ui.add_space(4.0);
        });

        egui::Panel::bottom("log")
            .resizable(true)
            .default_size(240.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("CONSOLE")
                            .color(theme::secondary_text(self.dark))
                            .strong(),
                    );
                    if ui.button("clear").clicked() {
                        self.log.clear();
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

        let sidebar = theme::sidebar_fill(self.dark);
        egui::Panel::left("actions")
            .resizable(false)
            .exact_size(230.0)
            .frame(egui::Frame::new().fill(sidebar))
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("TARGET ADAPTER").color(theme::secondary_text(self.dark)),
                );
                let current = self
                    .adapters
                    .get(self.selected)
                    .map(AdapterInfo::title)
                    .unwrap_or_else(|| "(none found)".to_string());
                let mut chosen = self.selected;
                egui::ComboBox::from_id_salt("adapter")
                    .selected_text(current)
                    .width(210.0)
                    .show_ui(ui, |ui| {
                        for (i, a) in self.adapters.iter().enumerate() {
                            ui.selectable_value(&mut chosen, i, a.title());
                        }
                    });
                if chosen != self.selected {
                    self.pending_dns = None;
                }
                self.selected = chosen;

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!self.busy, egui::Button::new("refresh"))
                        .clicked()
                    {
                        self.refresh_adapters();
                    }
                    if let Some(ad) = self.adapters.get(self.selected) {
                        ui.label(
                            egui::RichText::new(if ad.dhcp_enabled { "DHCP" } else { "STATIC" })
                                .color(theme::secondary_text(self.dark))
                                .small(),
                        );
                    }
                });

                ui.separator();
                ui.label(egui::RichText::new("ACTIONS").color(theme::secondary_text(self.dark)));
                for (label, tab) in TABS {
                    let selected = self.tab == tab;
                    if ui.selectable_label(selected, label).clicked() {
                        self.tab = tab;
                        self.confirm_reset = false;
                        self.pending_dns = None;
                    }
                }
            });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(6.0);
            match self.tab {
                Tab::Flush => self.ui_flush(ui),
                Tab::Register => self.ui_register(ui),
                Tab::Reset => self.ui_reset(ui),
                Tab::Check => self.ui_check(ui),
                Tab::SetDns => self.ui_set_dns(ui),
                Tab::SetIp => self.ui_set_ip(ui),
                Tab::Scan => self.ui_scan(ui),
                Tab::Auto => self.ui_auto(ui),
            }
        });
    }
}

impl DnsApp {
    fn heading(&self, ui: &mut egui::Ui, text: &str, sub: &str) {
        ui.label(
            egui::RichText::new(text)
                .color(theme::text(self.dark))
                .size(18.0)
                .strong(),
        );
        ui.label(egui::RichText::new(sub).color(theme::secondary_text(self.dark)));
        ui.add_space(8.0);
    }

    fn ui_flush(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "[1] Flush DNS cache",
            "Clear the Windows DNS resolver cache.",
        );
        if ui
            .add_enabled(!self.busy, egui::Button::new("FLUSH"))
            .clicked()
        {
            self.act_flush();
        }
    }

    fn ui_register(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "[2] Register DNS and renew IP",
            "ipconfig /registerdns, then release + renew DHCP.",
        );
        if ui
            .add_enabled(!self.busy, egui::Button::new("RUN"))
            .clicked()
        {
            self.act_register();
        }
    }

    fn ui_reset(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "[3] Reset Winsock and TCP/IP",
            "netsh winsock reset + netsh int ip reset. Restart required.",
        );
        ui.label(
            egui::RichText::new("This changes system network state.")
                .color(theme::warning(self.dark)),
        );
        if !self.confirm_reset {
            if ui
                .add_enabled(!self.busy, egui::Button::new("RESET..."))
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
                            egui::RichText::new("CONFIRM RESET").color(egui::Color32::WHITE),
                        )
                        .fill(theme::destructive(self.dark)),
                    )
                    .clicked()
                {
                    self.act_reset();
                }
                if ui.button("cancel").clicked() {
                    self.confirm_reset = false;
                }
            });
        }
    }

    fn ui_check(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "[4] Check DNS servers & resolution",
            "Show current servers, test resolution, and apply a recommended DNS.",
        );

        if let Some(ad) = self.adapters.get(self.selected) {
            let text = current_dns_label(ad);
            let color = if ad.dns_servers.is_empty() {
                theme::secondary_text(self.dark)
            } else {
                theme::text(self.dark)
            };
            let dhcp = ad.dhcp_enabled;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Current DNS:").strong());
                ui.label(egui::RichText::new(text).color(color));
                if dhcp {
                    ui.label(egui::RichText::new("(DHCP)").color(theme::secondary_text(self.dark)));
                }
            });
        }

        ui.horizontal(|ui| {
            ui.label("Host:");
            ui.text_edit_singleline(&mut self.lookup);
            if ui
                .add_enabled(!self.busy, egui::Button::new("CHECK"))
                .clicked()
            {
                self.act_check();
            }
        });

        ui.add_space(10.0);
        ui.label(egui::RichText::new("RECOMMENDED DNS").color(theme::secondary_text(self.dark)));
        ui.horizontal_wrapped(|ui| {
            for (name, servers) in DNS_PRESETS {
                if ui
                    .add_enabled(!self.busy, egui::Button::new(name))
                    .on_hover_text(servers.join(" / "))
                    .clicked()
                {
                    self.pending_dns = Some(DnsPlan::Servers(
                        servers.iter().map(|s| (*s).to_string()).collect(),
                    ));
                }
            }
            if ui
                .add_enabled(!self.busy, egui::Button::new("AD / automatic"))
                .on_hover_text("Use the DHCP / Active Directory-provided DNS servers")
                .clicked()
            {
                self.pending_dns = Some(DnsPlan::Automatic);
            }
        });

        if let Some(plan) = self.pending_dns.clone() {
            ui.add_space(8.0);
            let current = self
                .adapters
                .get(self.selected)
                .map(current_dns_label)
                .unwrap_or_else(|| "?".to_string());
            let planned = match &plan {
                DnsPlan::Automatic => "Automatic (DHCP / AD)".to_string(),
                DnsPlan::Servers(servers) => servers.join(", "),
            };
            ui.label(
                egui::RichText::new(format!(
                    "Change DNS on this adapter:  {current}  ->  {planned}"
                ))
                .color(theme::warning(self.dark)),
            );
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.busy, egui::Button::new("APPLY"))
                    .clicked()
                {
                    self.act_apply_dns_plan();
                }
                if ui.button("cancel").clicked() {
                    self.pending_dns = None;
                }
            });
        }
    }

    fn ui_set_dns(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "[5] Set DNS servers",
            "Apply a DNS provider to the selected adapter.",
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
                ui.label("Preferred:");
                ui.text_edit_singleline(&mut self.dns1);
            });
            ui.horizontal(|ui| {
                ui.label("Alternate:");
                ui.text_edit_singleline(&mut self.dns2);
            });
            if ui
                .add_enabled(!self.busy, egui::Button::new("APPLY"))
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
            "[6] Static IP / DHCP",
            "Configure a static IPv4 address or switch back to DHCP.",
        );
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.static_ip, false, "DHCP (automatic)");
            ui.radio_value(&mut self.static_ip, true, "Static");
        });
        ui.add_space(6.0);
        ui.add_enabled_ui(self.static_ip, |ui| {
            egui::Grid::new("ipform").num_columns(2).show(ui, |ui| {
                ui.label("IP address");
                ui.text_edit_singleline(&mut self.ip_addr);
                ui.end_row();
                ui.label("Mask / prefix");
                ui.text_edit_singleline(&mut self.mask);
                ui.end_row();
                ui.label("Gateway");
                ui.text_edit_singleline(&mut self.gateway);
                ui.end_row();
                ui.label("DNS 1");
                ui.text_edit_singleline(&mut self.ip_dns1);
                ui.end_row();
                ui.label("DNS 2");
                ui.text_edit_singleline(&mut self.ip_dns2);
                ui.end_row();
                ui.label("Alternate IP");
                ui.text_edit_singleline(&mut self.alt_ip);
                ui.end_row();
            });
        });
        ui.add_space(6.0);
        if ui
            .add_enabled(!self.busy, egui::Button::new("APPLY"))
            .clicked()
        {
            self.act_set_ip();
        }
    }

    fn ui_scan(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "[7] Scan subnet",
            "Ping sweep + ARP: list used IPs and recommend free ones.",
        );
        ui.horizontal(|ui| {
            ui.label("CIDR:");
            ui.text_edit_singleline(&mut self.scan_cidr);
        });
        if ui
            .add_enabled(!self.busy, egui::Button::new("SCAN"))
            .clicked()
        {
            self.act_scan();
        }
    }

    fn ui_auto(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "[8] AUTO setup",
            "Find a free IP, set it static, optionally add an alternative network.",
        );
        egui::Grid::new("autoform").num_columns(2).show(ui, |ui| {
            ui.label("Primary net");
            ui.text_edit_singleline(&mut self.primary_net);
            ui.end_row();
            ui.label("Alternative net");
            ui.text_edit_singleline(&mut self.alt_net);
            ui.end_row();
            ui.label("First host");
            ui.add(egui::DragValue::new(&mut self.first_host).range(2..=254));
            ui.end_row();
            ui.label("Alt start host");
            ui.add(egui::DragValue::new(&mut self.alt_start_host).range(2..=254));
            ui.end_row();
        });
        ui.checkbox(&mut self.auto_alt, "Also add an alternative-network IP");
        ui.add_space(6.0);
        if ui
            .add_enabled(!self.busy, egui::Button::new("RUN AUTO SETUP"))
            .clicked()
        {
            self.act_auto();
        }
    }
}
