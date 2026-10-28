use crate::accent::{
    set_taskbar_accent_policy, AccentPolicy, ACCENT_DISABLED, ACCENT_ENABLE_ACRYLICBLURBEHIND,
    ACCENT_ENABLE_TRANSPARENTGRADIENT,
};
use atelier_common::config::TaskbarStyle;
use atelier_common::error::Result;
use std::ffi::c_void;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};

const DWMWA_SYSTEMBACKDROP_TYPE_ATTR: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(38);

pub fn apply_taskbar_style(hwnd: HWND, style: TaskbarStyle, hide_top_border: bool) -> Result<()> {
    let backdrop_type: u32 = match style {
        TaskbarStyle::Default => 0,
        TaskbarStyle::Clear => 0,
        TaskbarStyle::Blur => 0,
        TaskbarStyle::Acrylic { .. } => 0,
        TaskbarStyle::Mica => 2,
    };
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE_ATTR,
            &backdrop_type as *const _ as *const c_void,
            std::mem::size_of::<u32>() as u32,
        )
    };

    let flags = if hide_top_border { 0 } else { 0x40 };

    let res = match style {
        TaskbarStyle::Default => {
            let policy = AccentPolicy {
                accent_state: ACCENT_DISABLED,
                accent_flags: 0,
                gradient_color: 0,
                animation_id: 0,
            };
            set_taskbar_accent_policy(hwnd, policy)
        }
        TaskbarStyle::Clear => {
            let policy = AccentPolicy {
                accent_state: ACCENT_ENABLE_TRANSPARENTGRADIENT,
                accent_flags: 0,
                gradient_color: 0x01000000,
                animation_id: 0,
            };
            set_taskbar_accent_policy(hwnd, policy)
        }
        TaskbarStyle::Blur => {
            let policy = AccentPolicy {
                accent_state: ACCENT_ENABLE_ACRYLICBLURBEHIND,
                accent_flags: flags,
                gradient_color: 0x01000000,
                animation_id: 0,
            };
            set_taskbar_accent_policy(hwnd, policy)
        }
        TaskbarStyle::Acrylic { tint } => {
            let policy = AccentPolicy {
                accent_state: ACCENT_ENABLE_ACRYLICBLURBEHIND,
                accent_flags: flags,
                gradient_color: tint,
                animation_id: 0,
            };
            set_taskbar_accent_policy(hwnd, policy)
        }
        TaskbarStyle::Mica => {
            let policy = AccentPolicy {
                accent_state: ACCENT_ENABLE_TRANSPARENTGRADIENT,
                accent_flags: flags,
                gradient_color: 0x01000000,
                animation_id: 0,
            };
            set_taskbar_accent_policy(hwnd, policy)
        }
    };

    res?;

    if crate::discovery::has_xaml_background(hwnd) {
        if let Err(err) = crate::xaml::apply_xaml_brush(hwnd, style) {
            tracing::warn!("apply_xaml_brush non-fatal: {err}");
        }
    }

    Ok(())
}
