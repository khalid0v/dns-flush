//! Read-only adapter enumeration.

use crate::win::adapters;

use super::wmi_connection;

#[tauri::command]
pub async fn list_adapters() -> Result<Vec<adapters::AdapterInfo>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let connection = wmi_connection()?;
        adapters::list(&connection).map_err(|error| format!("Adapter enumeration failed: {error}"))
    })
    .await
    .unwrap_or(Err("Adapter enumeration failed.".to_string()))
}
