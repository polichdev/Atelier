use crate::config::{TaskbarConfig, TaskbarStyle};
use crate::error::{AtelierError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemePreset {
    pub name: String,
    pub description: String,
    pub taskbar: TaskbarConfig,
    pub accent_color: u32,
    pub audio_reactive: bool,
    pub clock_widget_enabled: bool,
    pub hardware_widget_enabled: bool,
    pub media_widget_enabled: bool,
}

impl ThemePreset {
    pub fn minimal_clean() -> Self {
        Self {
            name: "Minimal Clean".to_string(),
            description: "100% transparent taskbar with minimal desktop clock".to_string(),
            taskbar: TaskbarConfig {
                style: TaskbarStyle::Clear,
                floating_dock: false,
                floating_margin: 0,
            },
            accent_color: 0x00FFFFFF,
            audio_reactive: false,
            clock_widget_enabled: true,
            hardware_widget_enabled: false,
            media_widget_enabled: false,
        }
    }

    pub fn cyberpunk_neon() -> Self {
        Self {
            name: "Cyberpunk Neon".to_string(),
            description: "High-contrast acrylic blur with audio-reactive neon pulsing".to_string(),
            taskbar: TaskbarConfig {
                style: TaskbarStyle::Acrylic { tint: 0xCC1A0033 },
                floating_dock: true,
                floating_margin: 6,
            },
            accent_color: 0x00FF007F,
            audio_reactive: true,
            clock_widget_enabled: true,
            hardware_widget_enabled: true,
            media_widget_enabled: true,
        }
    }

    pub fn frosted_glass() -> Self {
        Self {
            name: "Frosted Glass".to_string(),
            description: "Floating island dock with native Mica reflections".to_string(),
            taskbar: TaskbarConfig {
                style: TaskbarStyle::Mica,
                floating_dock: true,
                floating_margin: 10,
            },
            accent_color: 0x00E0E0E0,
            audio_reactive: false,
            clock_widget_enabled: false,
            hardware_widget_enabled: false,
            media_widget_enabled: true,
        }
    }

    pub fn nordic_dark() -> Self {
        Self {
            name: "Nordic Dark".to_string(),
            description: "Subtle dark blur with muted hardware telemetry".to_string(),
            taskbar: TaskbarConfig {
                style: TaskbarStyle::Blur,
                floating_dock: false,
                floating_margin: 0,
            },
            accent_color: 0x002E3440,
            audio_reactive: false,
            clock_widget_enabled: true,
            hardware_widget_enabled: true,
            media_widget_enabled: false,
        }
    }

    pub fn default_presets() -> Vec<Self> {
        vec![
            Self::minimal_clean(),
            Self::cyberpunk_neon(),
            Self::frosted_glass(),
            Self::nordic_dark(),
        ]
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(AtelierError::Serialization)
    }

    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(AtelierError::Serialization)
    }
}
