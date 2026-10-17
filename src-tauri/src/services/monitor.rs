//! Live status monitor service: the managed state that drives the background
//! watcher (`win::watch`) and the bridge thread that forwards its read-only
//! observations to the frontend as `log`, `adapters`, `public-ip`, and `toast`
//! events. Never starts a repair.

use std::net::IpAddr;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;

use tauri::{AppHandle, Emitter};

use crate::models::{LogLine, PublicIpPayload, ToastPayload};
use crate::msg::{MonitorCmd, StatusMsg};
use crate::win::adapters::AdapterInfo;

/// Sender that drives the read-only background monitor. Managed as Tauri state.
pub struct MonitorState {
    cmd_tx: Mutex<Sender<MonitorCmd>>,
}

impl MonitorState {
    pub fn new(cmd_tx: Sender<MonitorCmd>) -> Self {
        Self {
            cmd_tx: Mutex::new(cmd_tx),
        }
    }

    pub fn send(&self, cmd: MonitorCmd) -> Result<(), String> {
        self.cmd_tx
            .lock()
            .map_err(|_| "Monitor is unavailable.".to_string())?
            .send(cmd)
            .map_err(|_| "Monitor is unavailable.".to_string())
    }
}

fn emit_log(app: &AppHandle, kind: &str, text: String) {
    let _ = app.emit(
        "log",
        LogLine {
            kind: kind.to_string(),
            text,
        },
    );
}

/// Forward `win::watch` observations to the frontend: adapter snapshots (with
/// change descriptions logged and toasted like the old UI did) and public-IP
/// updates. Read-only; never starts a repair.
pub fn bridge(app: AppHandle, rx: Receiver<StatusMsg>) {
    let mut previous_adapters: Option<Vec<AdapterInfo>> = None;
    let mut previous_ip: Option<IpAddr> = None;
    for msg in rx {
        match msg {
            StatusMsg::Adapters(list) => {
                if let Some(previous) = &previous_adapters {
                    let changes = crate::monitor::describe_changes(previous, &list);
                    for change in &changes {
                        emit_log(&app, "info", change.clone());
                    }
                    if !changes.is_empty() {
                        let _ = app.emit(
                            "toast",
                            ToastPayload {
                                text: crate::monitor::summarize(&changes),
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
