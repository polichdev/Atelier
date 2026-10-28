use atelier_common::config::{TaskbarConfig, TaskbarStyle};
use atelier_taskbar::TaskbarManager;

#[test]
fn test_apply_macos_dock_style() {
    let config = TaskbarConfig {
        style: TaskbarStyle::Acrylic { tint: 0x501C2028 },
        floating_dock: true,
        floating_margin: 220,
        corner_radius: 18,
        vertical_gap: 4,
        hide_top_border: true,
                appearance: Default::default(),
    };
    let manager = TaskbarManager::new(config);
    let res = manager.apply_current_style();
    assert!(res.is_ok());
}
