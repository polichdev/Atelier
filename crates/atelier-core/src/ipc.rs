#![allow(dead_code)]

use atelier_common::config::{TaskbarConfig, TaskbarStyle};
use atelier_common::presets::ThemePreset;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AtelierIpcCommand {
    SetTaskbarStyle(TaskbarStyle),
    SetFloatingDock { enabled: bool, margin: i32 },
    SetAccentColor(u32),
    ApplyPreset(ThemePreset),
    ResetDefaults,
    GetStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtelierDaemonStatus {
    pub is_active: bool,
    pub taskbar: TaskbarConfig,
    pub current_preset: Option<String>,
    pub live_wallpaper_active: bool,
    pub audio_reactive_active: bool,
}

pub struct IpcDispatcher;

impl IpcDispatcher {
    pub fn serialize_command(cmd: &AtelierIpcCommand) -> Result<String, serde_json::Error> {
        serde_json::to_string(cmd)
    }

    pub fn deserialize_command(raw: &str) -> Result<AtelierIpcCommand, serde_json::Error> {
        serde_json::from_str(raw)
    }

    pub fn serialize_status(status: &AtelierDaemonStatus) -> Result<String, serde_json::Error> {
        serde_json::to_string(status)
    }

    pub fn deserialize_status(raw: &str) -> Result<AtelierDaemonStatus, serde_json::Error> {
        serde_json::from_str(raw)
    }
}
