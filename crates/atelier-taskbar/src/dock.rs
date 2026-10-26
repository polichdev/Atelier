use atelier_common::error::{AtelierError, Result};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, DeleteObject, SetWindowRgn, HRGN};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER,
};

pub fn apply_floating_dock(hwnd: HWND, margin: i32, corner_radius: i32) -> Result<()> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.map_err(|e| {
        AtelierError::Platform(format!("Failed to retrieve taskbar window rect: {e}"))
    })?;

    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;

    let dock_width = width - (margin * 2);
    let dock_height = height - (margin * 2);

    if dock_width <= 0 || dock_height <= 0 {
        return Ok(());
    }

    let rgn = unsafe { CreateRoundRectRgn(0, 0, width, height, corner_radius, corner_radius) };
    if rgn.0.is_null() {
        return Err(AtelierError::Platform(
            "Failed to create round rect region".to_string(),
        ));
    }

    let set_rgn_res = unsafe { SetWindowRgn(hwnd, rgn, true) };
    if set_rgn_res == 0 {
        unsafe {
            let _ = DeleteObject(rgn);
        };
        return Err(AtelierError::Platform(
            "Failed to assign window region".to_string(),
        ));
    }

    unsafe {
        SetWindowPos(
            hwnd,
            HWND::default(),
            rect.left + margin,
            rect.top + margin,
            dock_width,
            dock_height,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )
    }
    .map_err(|e| AtelierError::Platform(format!("Failed to adjust dock window pos: {e}")))?;

    Ok(())
}

pub fn reset_floating_dock(hwnd: HWND) -> Result<()> {
    let _ = unsafe { SetWindowRgn(hwnd, HRGN::default(), true) };
    Ok(())
}
