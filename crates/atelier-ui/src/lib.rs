use atelier_common::config::TaskbarStyle;
use atelier_common::error::Result;
use atelier_common::presets::ThemePreset;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiState {
    pub active_preset: Option<String>,
    pub taskbar_style: TaskbarStyle,
    pub floating_dock: bool,
    pub floating_margin: i32,
    pub accent_glow_color: u32,
    pub audio_reactive: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            active_preset: Some("Minimal Clean".to_string()),
            taskbar_style: TaskbarStyle::Clear,
            floating_dock: false,
            floating_margin: 8,
            accent_glow_color: 0x00FF8800,
            audio_reactive: true,
        }
    }
}

pub struct ThemeStudioController {
    state: UiState,
}

impl ThemeStudioController {
    pub fn new() -> Self {
        Self {
            state: UiState::default(),
        }
    }

    pub fn state(&self) -> &UiState {
        &self.state
    }

    pub fn apply_preset(&mut self, preset: &ThemePreset) {
        self.state.active_preset = Some(preset.name.clone());
        self.state.taskbar_style = preset.taskbar.style;
        self.state.floating_dock = preset.taskbar.floating_dock;
        self.state.floating_margin = preset.taskbar.floating_margin;
        self.state.accent_glow_color = preset.accent_color;
        self.state.audio_reactive = preset.audio_reactive;
    }

    pub fn set_taskbar_style(&mut self, style: TaskbarStyle) {
        self.state.taskbar_style = style;
        self.state.active_preset = None;
    }

    pub fn set_floating_dock(&mut self, enabled: bool, margin: i32) {
        self.state.floating_dock = enabled;
        self.state.floating_margin = margin;
        self.state.active_preset = None;
    }

    pub fn set_accent_color(&mut self, color: u32) {
        self.state.accent_glow_color = color;
    }

    pub fn export_active_preset(&self) -> Result<String> {
        let preset = ThemePreset {
            name: self
                .state
                .active_preset
                .clone()
                .unwrap_or_else(|| "Custom Theme".to_string()),
            description: "Exported from Atelier Theme Studio".to_string(),
            taskbar: atelier_common::config::TaskbarConfig {
                style: self.state.taskbar_style,
                floating_dock: self.state.floating_dock,
                floating_margin: self.state.floating_margin,
            },
            accent_color: self.state.accent_glow_color,
            audio_reactive: self.state.audio_reactive,
            clock_widget_enabled: true,
            hardware_widget_enabled: false,
            media_widget_enabled: false,
        };

        preset.to_json()
    }
}

impl Default for ThemeStudioController {
    fn default() -> Self {
        Self::new()
    }
}
