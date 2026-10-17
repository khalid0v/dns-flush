//! Non-OS application services: orchestration between the Tauri commands and
//! the `win` OS layer. Nothing here renders UI; everything here is read-only
//! with respect to system state.

pub mod monitor;
