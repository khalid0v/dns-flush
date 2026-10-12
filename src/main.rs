#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod elevate;
mod msg;
mod theme;
mod win;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    if !elevate::is_admin() {
        if elevate::relaunch_elevated() {
            return Ok(());
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 720.0])
            .with_min_inner_size([680.0, 480.0])
            .with_title("DNS FLUSH"),
        ..Default::default()
    };

    eframe::run_native(
        "DNS FLUSH",
        options,
        Box::new(|cc| Ok(Box::new(app::DnsApp::new(cc)))),
    )?;

    Ok(())
}
