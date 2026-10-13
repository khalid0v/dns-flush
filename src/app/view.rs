//! Rendering for [`DnsApp`]: the title bar, activity log, sidebar, and pages.
//! Layout only; behaviour is dispatched to the action methods in `super`.

use std::time::Duration;

use eframe::egui;

use crate::monitor;
use crate::msg::{LogKind, ScanResult};
use crate::theme;
use crate::win::AdapterInfo;

use super::nav::{self, Tab};
use super::{DNS_PRESETS, DnsApp, TOAST_LIFETIME, current_dns_label, current_ip_label};

impl eframe::App for DnsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll();
        let ctx = ui.ctx().clone();
        if self.theme_dirty {
            theme::apply(&ctx, self.dark);
            self.theme_dirty = false;
        }
        if self.busy || self.public_ip_loading {
            ctx.request_repaint_after(Duration::from_millis(120));
        }

        egui::Panel::top("title")
            .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(12, 8)))
            .show(ui, |ui| {
                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Network support")
                            .color(theme::text(self.dark))
                            .size(22.0)
                            .strong(),
                    );
                    ui.add_space(10.0);
                    let label = if self.busy { "Working…" } else { "Ready" };
                    let color = if self.busy {
                        theme::warning(self.dark)
                    } else {
                        theme::success(self.dark)
                    };
                    ui.label(egui::RichText::new(label).color(color));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mode = if self.dark { "Light mode" } else { "Dark mode" };
                        if ui.button(mode).on_hover_text("Switch appearance").clicked() {
                            self.dark = !self.dark;
                            self.theme_dirty = true;
                            ui.ctx().request_repaint();
                        }

                        ui.add_space(6.0);
                        let activity_label = if self.activity_visible {
                            "Hide activity"
                        } else {
                            "Show activity"
                        };
                        if ui.button(activity_label).clicked() {
                            self.activity_visible = !self.activity_visible;
                        }
                    });
                });

                ui.add_space(2.0);

                // The credit gets its own row so it can never collide with the
                // title or the buttons when the window is near its minimum width.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.hyperlink_to(
                        egui::RichText::new("github.com/khalid0v").color(theme::accent(self.dark)),
                        "https://github.com/khalid0v",
                    );
                    ui.label(
                        egui::RichText::new("dev by ahmed khalid | ")
                            .color(theme::secondary_text(self.dark)),
                    );
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Network adapter");
                    let current = self
                        .adapters
                        .get(self.selected)
                        .map(AdapterInfo::title)
                        .unwrap_or_else(|| "No adapter found".to_string());
                    let mut chosen = self.selected;
                    let combo_width = (ui.available_width() - 160.0).max(150.0);
                    egui::ComboBox::from_id_salt("adapter")
                        .selected_text(current)
                        .width(combo_width)
                        .show_ui(ui, |ui| {
                            for (i, adapter) in self.adapters.iter().enumerate() {
                                ui.selectable_value(&mut chosen, i, adapter.title());
                            }
                        });
                    if chosen != self.selected {
                        self.selected = chosen;
                        self.sync_scan_range();
                    }
                    if ui
                        .add_enabled(!self.busy, egui::Button::new("Refresh status"))
                        .clicked()
                    {
                        self.refresh_status();
                    }
                });

                ui.add_space(2.0);
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
                        for group in nav::GROUPS {
                            self.nav_icon_group(ui, group);
                        }
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
                        Tab::SetDns => self.ui_set_dns(ui),
                        Tab::SetIp => self.ui_set_ip(ui),
                        Tab::Scan => self.ui_scan(ui),
                        Tab::Auto => self.ui_auto(ui),
                    }
                });
        });

        self.draw_toast(&ctx);
    }
}

impl DnsApp {
    /// Small transient banner shown when the live monitor reports a change.
    fn draw_toast(&mut self, ctx: &egui::Context) {
        let Some((text, shown_at)) = self.toast.clone() else {
            return;
        };
        let elapsed = shown_at.elapsed();
        if elapsed >= TOAST_LIFETIME {
            self.toast = None;
            return;
        }
        ctx.request_repaint_after(TOAST_LIFETIME - elapsed);

        egui::Area::new(egui::Id::new("status_toast"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 56.0))
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(theme::sidebar_fill(self.dark))
                    .stroke(egui::Stroke::new(1.0, theme::accent(self.dark)))
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&text).color(theme::text(self.dark)));
                    });
            });
    }

    fn nav_icon_group(&mut self, ui: &mut egui::Ui, tabs: &[Tab]) {
        for tab in tabs {
            self.nav_icon_button(ui, *tab);
        }
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);
    }

    fn nav_icon_button(&mut self, ui: &mut egui::Ui, tab: Tab) {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::click());
        let response = response.on_hover_text(tab.label());
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
        nav::draw_icon(ui.painter(), rect.shrink(10.0), tab, color);

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

        ui.horizontal(|ui| {
            let mut live = self.monitor_on;
            let response = ui
                .checkbox(&mut live, "Live status")
                .on_hover_text("Refresh these cards automatically when the network changes.");
            if response.changed() {
                self.set_monitor(live);
            }
            let note = if self.monitor_on {
                "Monitoring for changes"
            } else {
                "Manual refresh only"
            };
            ui.label(egui::RichText::new(note).color(theme::secondary_text(self.dark)));
        });
        ui.add_space(4.0);

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
        let vpn_status = monitor::vpn_status_label(&self.adapters);

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

        // This PC's own address and how it was configured.
        if let Some(ad) = self.adapter() {
            let ip = ad
                .primary_ip()
                .cloned()
                .unwrap_or_else(|| "no IP".to_string());
            let subnet = ad
                .subnet_for(&ip)
                .map(|m| m.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let mode = if ad.dhcp_enabled { "DHCP" } else { "Static" };
            ui.label(
                egui::RichText::new(format!(
                    "This PC: {ip}  ·  subnet {subnet}  ·  {mode}  ·  {}",
                    ad.description
                ))
                .color(theme::secondary_text(self.dark)),
            );
            ui.add_space(4.0);
        }

        ui.horizontal(|ui| {
            ui.label("Network range");
            ui.text_edit_singleline(&mut self.scan_cidr);
            if ui
                .add_enabled(!self.busy, egui::Button::new("Find devices"))
                .clicked()
            {
                self.act_scan();
            }
        });

        let Some(result) = self.scan_result.as_ref() else {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Choose a range and press Find devices to see what is in use.")
                    .color(theme::secondary_text(self.dark)),
            );
            return;
        };

        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(format!(
                "{} of {} addresses in use in {}",
                result.used.len(),
                result.used.len() + result.free.len(),
                result.network
            ))
            .color(theme::text(self.dark))
            .strong(),
        );
        ui.add_space(6.0);
        self.scan_table(ui, result);
    }

    /// Scrollable table of every host: devices in use first, then free addresses.
    fn scan_table(&self, ui: &mut egui::Ui, result: &ScanResult) {
        let mut rows: Vec<(String, String, String, bool)> = Vec::new();
        for device in &result.used {
            rows.push((
                device.ip.to_string(),
                "In use".to_string(),
                format!("{} {}", device.mac, device.note).trim().to_string(),
                false,
            ));
        }
        for ip in &result.free {
            rows.push((ip.to_string(), "Free".to_string(), String::new(), true));
        }

        egui::ScrollArea::vertical()
            .max_height(240.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("scan_table")
                    .num_columns(3)
                    .spacing(egui::vec2(10.0, 4.0))
                    .show(ui, |ui| {
                        for title in ["IP address", "Status", "MAC / note"] {
                            ui.label(
                                egui::RichText::new(title)
                                    .color(theme::text(self.dark))
                                    .strong(),
                            );
                        }
                        ui.end_row();

                        for (ip, status, detail, is_free) in &rows {
                            ui.label(ip);
                            let color = if *is_free {
                                theme::success(self.dark)
                            } else {
                                theme::warning(self.dark)
                            };
                            ui.label(egui::RichText::new(status).color(color).strong());
                            ui.label(detail);
                            ui.end_row();
                        }
                    });
            });
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
