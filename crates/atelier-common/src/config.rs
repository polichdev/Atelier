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
}

impl Default for TaskbarConfig {
    fn default() -> Self {
        Self {
            style: TaskbarStyle::Clear,
            floating_dock: false,
            floating_margin: 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AtelierConfig {
    pub autostart: bool,
    pub taskbar: TaskbarConfig,
}
