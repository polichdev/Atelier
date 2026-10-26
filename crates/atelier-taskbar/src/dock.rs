use crate::discovery::ensure_interactive_desktop;
use atelier_common::error::{AtelierError, Result};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateRoundRectRgn, DeleteObject, RedrawWindow, SetWindowRgn, HRGN, RDW_ALLCHILDREN, RDW_ERASE,
    RDW_FRAME, RDW_INVALIDATE, RDW_UPDATENOW,
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

pub fn apply_floating_dock(hwnd: HWND, margin: i32, corner_radius: i32) -> Result<()> {
    ensure_interactive_desktop();
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

    let rgn = unsafe {
        CreateRoundRectRgn(
            margin,
            margin.min(6),
            width - margin,
            height - margin.min(6),
            corner_radius,
            corner_radius,
        )
    };
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
        let _ = RedrawWindow(
            hwnd,
            None,
            HRGN::default(),
            RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_FRAME | RDW_UPDATENOW,
        );
    }

    Ok(())
}

pub fn reset_floating_dock(hwnd: HWND) -> Result<()> {
    ensure_interactive_desktop();
    let _ = unsafe { SetWindowRgn(hwnd, HRGN::default(), true) };
    unsafe {
        let _ = RedrawWindow(
            hwnd,
            None,
            HRGN::default(),
            RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_FRAME | RDW_UPDATENOW,
        );
    }
    Ok(())
}
