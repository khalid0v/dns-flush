#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! trubo: Windows IT support toolkit (Tauri backend).
//!
//! All OS access lives in `win/` (no UI types there). Actions are exposed as
//! async Tauri commands in `commands`; the read-only live monitor from
//! `win::watch` runs on a bridge thread and forwards observations as `log`,
//! `adapters`, `public-ip`, and `toast` events. Pure diff/classification of
//! adapter snapshots stays in `monitor` (unit-tested, no OS access).
//!
//! The app self-elevates to Administrator at startup because repairs change
//! system state; read-only commands work without elevation where Windows
//! allows it.

mod commands;
mod elevate;
mod monitor;
mod msg;
mod win;

use std::net::IpAddr;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::msg::StatusMsg;
use crate::win::adapters::AdapterInfo;

/// Public-IP observation, normalized so the frontend sees one shape.
#[derive(Clone, Debug, Serialize)]
struct PublicIpPayload {
    ok: bool,
    value: String,
}

/// Transient banner for live-monitor network changes.
#[derive(Clone, Debug, Serialize)]
struct ToastPayload {
    text: String,
}

fn emit_log(app: &AppHandle, kind: &str, text: String) {
    let _ = app.emit(
        "log",
        commands::LogLine {
            kind: kind.to_string(),
            text,
        },
    );
}

/// Forward `win::watch` observations to the frontend: adapter snapshots (with
/// change descriptions logged and toasted like the old UI did) and public-IP
/// updates. Read-only; never starts a repair.
fn monitor_bridge(app: AppHandle, rx: std::sync::mpsc::Receiver<StatusMsg>) {
    let mut previous_adapters: Option<Vec<AdapterInfo>> = None;
    let mut previous_ip: Option<IpAddr> = None;
    for msg in rx {
        match msg {
            StatusMsg::Adapters(list) => {
                if let Some(previous) = &previous_adapters {
                    let changes = monitor::describe_changes(previous, &list);
                    for change in &changes {
                        emit_log(&app, "info", change.clone());
                    }
                    if !changes.is_empty() {
                        let _ = app.emit(
                            "toast",
                            ToastPayload {
                                text: monitor::summarize(&changes),
                            },
                        );
                    }
                }
                previous_adapters = Some(list.clone());
                let _ = app.emit("adapters", &list);
            }
            StatusMsg::PublicIp(result) => {
                let payload = match result {
                    Ok(address) => {
                        if let Some(previous) = previous_ip
                            && previous != address
                        {
                            emit_log(
                                &app,
                                "info",
                                format!("Public IP changed: {previous} -> {address}"),
                            );
                        }
                        previous_ip = Some(address);
                        PublicIpPayload {
                            ok: true,
                            value: address.to_string(),
                        }
                    }
                    Err(error) => PublicIpPayload {
                        ok: false,
                        value: error,
                    },
                };
                let _ = app.emit("public-ip", &payload);
            }
        }
    }
}

fn main() {
    if !elevate::is_admin() && elevate::relaunch_elevated() {
        return;
    }
    if let Err(error) = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_adapters,
            commands::get_public_ip,
            commands::flush_dns,
            commands::register_renew,
            commands::reset_stack,
            commands::set_dns,
            commands::reset_dns,
            commands::enable_dhcp,
            commands::set_static_ip,
            commands::scan_network,
            commands::auto_setup,
            commands::set_monitor_enabled,
            commands::refresh_monitor,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let (cmd_tx, status_rx) = win::watch::start(|| {});
            app.manage(commands::MonitorState::new(cmd_tx));
            std::thread::spawn(move || monitor_bridge(handle, status_rx));
            Ok(())
        })
        .run(tauri::generate_context!())
    {
        eprintln!("error while running trubo: {error}");
        std::process::exit(1);
    }
}
