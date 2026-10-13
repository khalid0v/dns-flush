//! Navigation model for the sidebar: the available pages, their labels, and
//! their line icons. Pure data and drawing; no OS access and no app state.

use eframe::egui;

/// One page of the application, shown as an icon button in the sidebar.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Tab {
    Check,
    DnsLookup,
    Scan,
    Flush,
    Register,
    Reset,
    SetDns,
    SetIp,
    Auto,
}

impl Tab {
    /// Sidebar label for this page.
    pub(super) fn label(self) -> &'static str {
        match self {
            Tab::Check => "Network status",
            Tab::DnsLookup => "Check a website",
            Tab::Scan => "Find devices",
            Tab::Flush => "Flush DNS cache",
            Tab::Register => "Renew connection",
            Tab::Reset => "Reset network",
            Tab::SetDns => "Choose DNS servers",
            Tab::SetIp => "Set IP address",
            Tab::Auto => "Automatic setup",
        }
    }
}

/// Sidebar sections, top to bottom. Each slice is separated by a rule.
pub(super) const GROUPS: [&[Tab]; 3] = [
    &[Tab::Check, Tab::DnsLookup, Tab::Scan],
    &[Tab::Flush, Tab::Register, Tab::Reset],
    &[Tab::SetDns, Tab::SetIp, Tab::Auto],
];

/// Draws the compact, line-icon navigation glyph for `tab`. Every icon uses the
/// same 1.8 point stroke so the rail looks consistent.
pub(super) fn draw_icon(painter: &egui::Painter, rect: egui::Rect, tab: Tab, color: egui::Color32) {
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
