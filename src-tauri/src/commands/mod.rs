//! Tauri commands: every toolkit action, split by domain. Blocking Windows
//! work runs on `spawn_blocking` threads; progress streams back as `log`
//! events, and the collected lines are returned so the frontend can clear its
//! busy state when the promise resolves.
//!
//! Each command opens its own WMI connection inside the worker thread:
//! `WMIConnection` is not `Send`, so connections are never shared. Shared
//! plumbing (`Logger`, `wmi_connection`, `find_adapter`) lives here.

pub mod adapters;
pub mod auto;
pub mod dns;
pub mod ip;
pub mod monitor;
pub mod public_ip;
pub mod reset;
pub mod scan;

use tauri::{AppHandle, Emitter};
use wmi::WMIConnection;

use crate::models::LogLine;
use crate::win;

/// Streams log lines as `log` events while collecting them for the return value.
pub(crate) struct Logger {
    app: AppHandle,
    out: Vec<LogLine>,
}

impl Logger {
    pub(crate) fn new(app: AppHandle, head: &str) -> Self {
        let mut logger = Self {
            app,
            out: Vec::new(),
        };
        logger.log("head", head.to_string());
        logger
    }

    pub(crate) fn log(&mut self, kind: &str, text: String) {
        let line = LogLine {
            kind: kind.to_string(),
            text,
        };
        let _ = self.app.emit("log", &line);
        self.out.push(line);
    }

    /// Log the outcome of a Win32 call that reports a status code (`0` means
    /// success), mirroring the old `report_status` helper.
    pub(crate) fn report(&mut self, operation: &str, success: &str, result: anyhow::Result<u32>) {
        match result {
            Ok(0) => self.log("ok", success.to_string()),
            Ok(code) => self.log("err", format!("{operation} returned code {code}")),
            Err(error) => self.log("err", format!("{operation} failed: {error}")),
        }
    }

    pub(crate) fn finish(self) -> Vec<LogLine> {
        self.out
    }
}

pub(crate) fn wmi_connection() -> Result<WMIConnection, String> {
    win::adapters::connect().map_err(|error| format!("WMI error: {error}"))
}

/// Fresh snapshot of the adapter with WMI `index`, or an error line's text.
pub(crate) fn find_adapter(index: u32) -> Result<win::adapters::AdapterInfo, String> {
    let connection = wmi_connection()?;
    win::adapters::list(&connection)
        .map_err(|error| format!("Adapter enumeration failed: {error}"))?
        .into_iter()
        .find(|adapter| adapter.index == index)
        .ok_or_else(|| "No adapter selected.".to_string())
}
