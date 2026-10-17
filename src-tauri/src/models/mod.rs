//! Serializable data shapes shared by the commands, services, and OS layers.
//! Nothing here touches the UI or opens a handle: these are plain models.

pub mod log;
pub mod payload;
pub mod scan;

pub use log::LogLine;
pub use payload::{PublicIpPayload, ToastPayload};
pub use scan::{ScanDevice, ScanResult};
