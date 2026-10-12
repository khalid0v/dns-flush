mod app;
mod elevate;
mod msg;
mod theme;
mod win;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    if !elevate::is_admin() {
        eprintln!("Requesting administrator privileges...");
        if elevate::relaunch_elevated() {
            return Ok(());
        }
        eprintln!("Warning: could not elevate; network changes may fail.");
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 720.0])
            .with_min_inner_size([820.0, 560.0])
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
