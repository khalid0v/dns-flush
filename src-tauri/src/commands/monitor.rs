//! Live status monitor commands: pause/resume updates and force a refresh.

use tauri::State;

use crate::msg::MonitorCmd;
use crate::services::monitor::MonitorState;

#[tauri::command]
pub fn set_monitor_enabled(state: State<'_, MonitorState>, on: bool) -> Result<(), String> {
    state.send(MonitorCmd::SetEnabled(on))
}

#[tauri::command]
pub fn refresh_monitor(state: State<'_, MonitorState>) -> Result<(), String> {
    state.send(MonitorCmd::RefreshNow)
}
