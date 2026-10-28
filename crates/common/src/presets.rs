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
            description: "Transparent floating island dock".to_string(),
            taskbar: TaskbarConfig {
                style: TaskbarStyle::Clear,
                floating_dock: true,
                floating_margin: 24,
                corner_radius: 16,
                vertical_gap: 8,
                hide_top_border: true,
                appearance: Default::default(),
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
            description: "Vivid acrylic tint with an island shape".to_string(),
            taskbar: TaskbarConfig {
                style: TaskbarStyle::Acrylic { tint: 0xCC7F00FF },
                floating_dock: true,
                floating_margin: 24,
                corner_radius: 16,
                vertical_gap: 8,
                hide_top_border: true,
                appearance: Default::default(),
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
            description: "Mica backdrop with rounded floating shape".to_string(),
            taskbar: TaskbarConfig {
                style: TaskbarStyle::Mica,
                floating_dock: true,
                floating_margin: 24,
                corner_radius: 16,
                vertical_gap: 8,
                hide_top_border: true,
                appearance: Default::default(),
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
            description: "Subtle blurred taskbar floating island".to_string(),
            taskbar: TaskbarConfig {
                style: TaskbarStyle::Blur,
                floating_dock: true,
                floating_margin: 24,
                corner_radius: 16,
                vertical_gap: 8,
                hide_top_border: true,
                appearance: Default::default(),
            },
            accent_color: 0x002E3440,
            audio_reactive: false,
            clock_widget_enabled: true,
            hardware_widget_enabled: true,
            media_widget_enabled: false,
        }
    }

    pub fn macos_sonoma() -> Self {
        Self {
            name: "macOS Sonoma".to_string(),
            description: "Translucent glass floating dock with elevated pill".to_string(),
            taskbar: TaskbarConfig {
                style: TaskbarStyle::Acrylic { tint: 0x501C2028 },
                floating_dock: true,
                floating_margin: 220,
                corner_radius: 18,
                vertical_gap: 4,
                hide_top_border: true,
                appearance: Default::default(),
            },
            accent_color: 0x00FFFFFF,
            audio_reactive: false,
            clock_widget_enabled: true,
            hardware_widget_enabled: false,
            media_widget_enabled: true,
        }
    }

    pub fn macos_big_sur() -> Self {
        Self {
            name: "macOS Big Sur".to_string(),
            description: "Frosted acrylic glass floating dock with deep rounded corners"
                .to_string(),
            taskbar: TaskbarConfig {
                style: TaskbarStyle::Acrylic { tint: 0x65222632 },
                floating_dock: true,
                floating_margin: 240,
                corner_radius: 20,
                vertical_gap: 4,
                hide_top_border: true,
                appearance: Default::default(),
            },
            accent_color: 0x000A84FF,
            audio_reactive: false,
            clock_widget_enabled: true,
            hardware_widget_enabled: false,
            media_widget_enabled: true,
        }
    }

    pub fn cupertino_dark() -> Self {
        Self {
            name: "Cupertino Dark".to_string(),
            description: "Deep blurred floating pill with maximum glass blur".to_string(),
            taskbar: TaskbarConfig {
                style: TaskbarStyle::Blur,
                floating_dock: true,
                floating_margin: 220,
                corner_radius: 18,
                vertical_gap: 4,
                hide_top_border: true,
                appearance: Default::default(),
            },
            accent_color: 0x001C1C1E,
            audio_reactive: false,
            clock_widget_enabled: true,
            hardware_widget_enabled: false,
            media_widget_enabled: true,
        }
    }

    pub fn cupertino_light() -> Self {
        Self {
            name: "Cupertino Light".to_string(),
            description: "Translucent frosted white glass dock".to_string(),
            taskbar: TaskbarConfig {
                style: TaskbarStyle::Acrylic { tint: 0x48FFFFFF },
                floating_dock: true,
                floating_margin: 220,
                corner_radius: 18,
                vertical_gap: 4,
                hide_top_border: true,
                appearance: Default::default(),
            },
            accent_color: 0x00F2F2F7,
            audio_reactive: false,
            clock_widget_enabled: true,
            hardware_widget_enabled: false,
            media_widget_enabled: true,
        }
    }

    pub fn clean_glass() -> Self {
        Self {
            name: "Clean Glass".to_string(),
            description: "Floating transparent island dock".to_string(),
            taskbar: TaskbarConfig {
                style: TaskbarStyle::Clear,
                floating_dock: true,
                floating_margin: 200,
                corner_radius: 18,
                vertical_gap: 4,
                hide_top_border: true,
                appearance: Default::default(),
            },
            accent_color: 0x00FFFFFF,
            audio_reactive: false,
            clock_widget_enabled: true,
            hardware_widget_enabled: false,
            media_widget_enabled: false,
        }
    }

    pub fn windows_default() -> Self {
        Self {
            name: "Windows Stock".to_string(),
            description: "Default standard Windows taskbar".to_string(),
            taskbar: TaskbarConfig {
                style: TaskbarStyle::Default,
                floating_dock: false,
                floating_margin: 0,
                corner_radius: 0,
                vertical_gap: 0,
                hide_top_border: false,
                appearance: Default::default(),
            },
            accent_color: 0x000078D4,
            audio_reactive: false,
            clock_widget_enabled: false,
            hardware_widget_enabled: false,
            media_widget_enabled: false,
        }
    }

    pub fn default_presets() -> Vec<Self> {
        vec![
            Self::macos_sonoma(),
            Self::macos_big_sur(),
            Self::cupertino_dark(),
            Self::cupertino_light(),
            Self::clean_glass(),
            Self::windows_default(),
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
