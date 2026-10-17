//! Winsock and TCP/IP stack reset.

use tauri::AppHandle;

use crate::models::LogLine;
use crate::win::reset;

use super::Logger;

#[tauri::command]
pub async fn reset_stack(app: AppHandle) -> Vec<LogLine> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut log = Logger::new(app, "[3] Reset Winsock and TCP/IP stack");
        log.log("warn", "Resetting Winsock...".to_string());
        match reset::winsock_reset() {
            Ok(lines) => {
                for line in lines {
                    log.log("info", line);
                }
            }
            Err(error) => log.log("err", format!("winsock reset failed: {error}")),
        }
        log.log("warn", "Resetting TCP/IP stack...".to_string());
        match reset::ip_reset() {
            Ok(lines) => {
                for line in lines {
                    log.log("info", line);
                }
            }
            Err(error) => log.log("err", format!("ip reset failed: {error}")),
        }
        log.log(
            "ok",
            "Reset complete. Restart the computer to finish.".to_string(),
        );
        log.finish()
    })
    .await
    .unwrap_or_default()
}
