#[cfg(target_os = "linux")]
mod linux_tray;
#[cfg(any(target_os = "windows", target_os = "macos"))]
mod generic_tray;

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
mod no_tray;

use serde::{Deserialize, Serialize};
#[cfg(target_os = "linux")]
pub use crate::linux_tray::LinuxTray as Tray;
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub use crate::generic_tray::GenericTray as Tray;
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub use crate::no_tray::NoTray as Tray;


#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayMenu {
    pub id: Option<String>,
    pub label: Option<String>,
    pub kind: String,
    pub checked: Option<bool>,
    pub enabled: Option<bool>,
}
pub enum MenuKind {
    Standard,
    Checkmark,
    Separator,
}

impl MenuKind {
    pub fn from_str(str: &str) -> Option<MenuKind> {
        match str {
            "standard" => Some(MenuKind::Standard),
            "checkmark" => Some(MenuKind::Checkmark),
            "separator" => Some(MenuKind::Separator),
            _ => None,
        }
    }
}