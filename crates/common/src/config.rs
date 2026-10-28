use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TaskbarStyle {
    #[default]
    Default,
    Clear,
    Blur,
    Acrylic {
        tint: u32,
    },
    Mica,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskbarConfig {
    pub style: TaskbarStyle,
    pub floating_dock: bool,
    pub floating_margin: i32,
    #[serde(default = "default_corner_radius")]
    pub corner_radius: i32,
    #[serde(default = "default_vertical_gap")]
    pub vertical_gap: i32,
    #[serde(default = "default_hide_top_border")]
    pub hide_top_border: bool,
    #[serde(default)]
    pub appearance: TaskbarAppearance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TaskbarAppearance {
    pub position: i32,
    pub icon_size: u32,
    pub button_width: u32,
    pub button_radius: i32,
    pub inactive_opacity: u32,
    pub running_indicator: u32,
    pub hover_scale: u32,
}
impl Default for TaskbarAppearance {
    fn default() -> Self {
        Self {
            position: -1,
            icon_size: 0,
            button_width: 0,
            button_radius: -1,
            inactive_opacity: 100,
            running_indicator: 0,
            hover_scale: 100,
        }
    }
}
impl TaskbarAppearance {
    pub fn normalize(&mut self) {
        self.position = self.position.clamp(-1, 3);
        if self.icon_size > 0 {
            self.icon_size = self.icon_size.clamp(16, 64)
        }
        if self.button_width > 0 {
            self.button_width = self.button_width.clamp(self.icon_size.max(16) + 8, 120)
        }
        self.button_radius = self.button_radius.clamp(-1, 24);
        self.inactive_opacity = self.inactive_opacity.clamp(30, 100);
        self.running_indicator = self.running_indicator.min(3);
        self.hover_scale = self.hover_scale.clamp(100, 150);
    }
}

fn default_corner_radius() -> i32 {
    18
}

fn default_vertical_gap() -> i32 {
    4
}

fn default_hide_top_border() -> bool {
    true
}

impl Default for TaskbarConfig {
    fn default() -> Self {
        Self {
            style: TaskbarStyle::Default,
            floating_dock: false,
            floating_margin: 0,
            corner_radius: 0,
            vertical_gap: 0,
            hide_top_border: false,
            appearance: TaskbarAppearance::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AtelierConfig {
    pub autostart: bool,
    pub taskbar: TaskbarConfig,
}
#[cfg(test)]
mod appearance_tests {
    use super::*;
    #[test]
    fn new_config_preserves_windows_appearance() {
        let config = TaskbarConfig::default();
        assert_eq!(config.style, TaskbarStyle::Default);
        assert!(!config.floating_dock);
        assert_eq!(config.appearance, TaskbarAppearance::default());
    }
    #[test]
    fn old_settings_preserve_windows_button_style() {
        let c: TaskbarConfig = serde_json::from_str(
            r#"{"style":"Default","floating_dock":false,"floating_margin":0}"#,
        )
        .unwrap();
        assert_eq!(c.appearance, TaskbarAppearance::default());
    }
    #[test]
    fn button_width_fits_the_requested_icon() {
        let mut a = TaskbarAppearance {
            icon_size: 32,
            button_width: 20,
            ..Default::default()
        };
        a.normalize();
        assert_eq!(a.button_width, 40);
    }
}
