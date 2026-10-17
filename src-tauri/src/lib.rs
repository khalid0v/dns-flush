#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! trubo: Windows IT support toolkit (Tauri backend).
//!
//! All OS access lives in `win/` (no UI types there). Actions are exposed as
//! async Tauri commands under `commands`, split by domain; the read-only live
//! monitor from `win::watch` runs on the bridge in `services::monitor` and
//! forwards observations as `log`, `adapters`, `public-ip`, and `toast`
//! events. Pure diff/classification of adapter snapshots stays in `monitor`
//! (unit-tested, no OS access).
//!
//! The app self-elevates to Administrator at startup because repairs change
//! system state; read-only commands work without elevation where Windows
//! allows it.

pub mod commands;
pub mod models;
pub mod monitor;
pub mod msg;
pub mod services;
pub mod win;

use tauri::{Builder, Manager};

/// Wire up the Tauri app: register the commands, start the live monitor, and
/// run the event loop. Called by the `trubo` binary.
pub fn run() {
    if !win::elevate::is_admin() && win::elevate::relaunch_elevated() {
        return;
    }
    if let Err(error) = Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::adapters::list_adapters,
            commands::public_ip::get_public_ip,
            commands::dns::flush_dns,
            commands::ip::register_renew,
            commands::reset::reset_stack,
            commands::dns::set_dns,
            commands::dns::reset_dns,
            commands::ip::enable_dhcp,
            commands::ip::set_static_ip,
            commands::scan::scan_network,
            commands::auto::auto_setup,
            commands::monitor::set_monitor_enabled,
            commands::monitor::refresh_monitor,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let (cmd_tx, status_rx) = win::watch::start(|| {});
            app.manage(services::monitor::MonitorState::new(cmd_tx));
            std::thread::spawn(move || services::monitor::bridge(handle, status_rx));
            Ok(())
        })
        .run(tauri::generate_context!())
    {
        eprintln!("error while running trubo: {error}");
        std::process::exit(1);
    }
}
