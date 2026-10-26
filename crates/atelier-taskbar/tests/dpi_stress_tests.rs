use atelier_common::config::{TaskbarConfig, TaskbarStyle};
use atelier_taskbar::TaskbarManager;

#[test]
fn test_taskbar_manager_lifecycle() {
    let config = TaskbarConfig {
        style: TaskbarStyle::Clear,
        floating_dock: false,
        floating_margin: 8,
    };

    let manager = TaskbarManager::new(config);
    let apply_res = manager.apply_current_style();
    assert!(apply_res.is_ok());

    let set_style_res = manager.set_style(TaskbarStyle::Blur);
    assert!(set_style_res.is_ok());

    let reset_res = manager.reset_all();
    assert!(reset_res.is_ok());
}

#[test]
fn test_mixed_dpi_margin_scaling() {
    let base_margin = 10;
    let dpi_scales = [1.0f32, 1.25f32, 1.5f32, 2.0f32];

    for &scale in &dpi_scales {
        let scaled_margin = (base_margin as f32 * scale).round() as i32;
        assert!(scaled_margin >= 10);
        assert!(scaled_margin <= 20);
    }
}

#[test]
fn test_floating_dock_bounds_constraint() {
    let width = 1920;
    let height = 48;
    let margin = 12;

    let dock_width = width - (margin * 2);
    let dock_height = height - (margin * 2);

    assert_eq!(dock_width, 1896);
    assert_eq!(dock_height, 24);
}
