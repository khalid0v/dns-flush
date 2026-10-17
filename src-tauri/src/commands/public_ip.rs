//! Read-only public-IP lookup.

use crate::win::public_ip;

#[tauri::command]
pub async fn get_public_ip() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(|| {
        public_ip::lookup()
            .map(|address| address.to_string())
            .map_err(|error| error.to_string())
    })
    .await
    .unwrap_or(Err("Public IP lookup failed.".to_string()))
}
