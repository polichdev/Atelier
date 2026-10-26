use crate::accent::{
    set_taskbar_accent_policy, AccentPolicy, ACCENT_DISABLED, ACCENT_ENABLE_ACRYLICBLURBEHIND,
    ACCENT_ENABLE_BLURBEHIND, ACCENT_ENABLE_TRANSPARENTGRADIENT,
};
use atelier_common::config::TaskbarStyle;
use atelier_common::error::Result;
use std::ffi::c_void;
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW, GWL_EXSTYLE,
    LWA_ALPHA, WS_EX_LAYERED,
};

const DWMWA_SYSTEMBACKDROP_TYPE_ATTR: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(38);
const DWMSBT_MAINWINDOW: u32 = 2;

pub fn apply_taskbar_style(hwnd: HWND, style: TaskbarStyle) -> Result<()> {
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
                accent_flags: 2,
                gradient_color: 0x00000000,
                animation_id: 0,
            };
            set_taskbar_accent_policy(hwnd, policy)
        }
        TaskbarStyle::Blur => {
            let policy = AccentPolicy {
                accent_state: ACCENT_ENABLE_BLURBEHIND,
                accent_flags: 2,
                gradient_color: 0,
                animation_id: 0,
            };
            set_taskbar_accent_policy(hwnd, policy)
        }
        TaskbarStyle::Acrylic { tint } => {
            let policy = AccentPolicy {
                accent_state: ACCENT_ENABLE_ACRYLICBLURBEHIND,
                accent_flags: 2,
                gradient_color: tint,
                animation_id: 0,
            };
            set_taskbar_accent_policy(hwnd, policy)
        }
        TaskbarStyle::Mica => {
            let backdrop_type = DWMSBT_MAINWINDOW;
            let dwm_res = unsafe {
                DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_SYSTEMBACKDROP_TYPE_ATTR,
                    &backdrop_type as *const _ as *const c_void,
                    std::mem::size_of::<u32>() as u32,
                )
            };

            if dwm_res.is_err() {
                let policy = AccentPolicy {
                    accent_state: ACCENT_ENABLE_BLURBEHIND,
                    accent_flags: 2,
                    gradient_color: 0,
                    animation_id: 0,
                };
                set_taskbar_accent_policy(hwnd, policy)?;
            }

            Ok(())
        }
    };

    update_xaml_taskbar_layer(hwnd, style);

    res
}

fn update_xaml_taskbar_layer(hwnd: HWND, style: TaskbarStyle) {
    let bridge = unsafe {
        FindWindowExW(
            hwnd,
            HWND::default(),
            w!("Windows.UI.Composition.DesktopWindowContentBridge"),
            None,
        )
    };

    if let Ok(b) = bridge {
        if !b.0.is_null() {
            let alpha = match style {
                TaskbarStyle::Clear => 0u8,
                TaskbarStyle::Acrylic { tint } => ((tint >> 24) & 0xFF) as u8,
                TaskbarStyle::Blur => 60u8,
                TaskbarStyle::Mica => 120u8,
                TaskbarStyle::Default => 255u8,
            };

            unsafe {
                let ex = GetWindowLongW(b, GWL_EXSTYLE);
                if style == TaskbarStyle::Default {
                    let _ = SetWindowLongW(b, GWL_EXSTYLE, ex & !WS_EX_LAYERED.0 as i32);
                    let _ = SetLayeredWindowAttributes(b, COLORREF(0), 255, LWA_ALPHA);
                } else {
                    let _ = SetWindowLongW(b, GWL_EXSTYLE, ex | WS_EX_LAYERED.0 as i32);
                    let _ = SetLayeredWindowAttributes(b, COLORREF(0), alpha, LWA_ALPHA);
                }
            }
        }
    }
}
