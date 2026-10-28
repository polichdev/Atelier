use atelier_common::error::{AtelierError, Result};
use windows::core::w;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateRoundRectRgn, DeleteObject, RedrawWindow, SetWindowRgn, HRGN, RDW_ALLCHILDREN, RDW_ERASE,
    RDW_FRAME, RDW_INVALIDATE, RDW_UPDATENOW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, GetWindowRect, SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER,
};

pub fn apply_floating_dock(
    hwnd: HWND,
    margin: i32,
    corner_radius: i32,
    vertical_gap: i32,
    hide_top_border: bool,
) -> Result<()> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.map_err(|e| {
        AtelierError::Platform(format!("Failed to retrieve taskbar window rect: {e}"))
    })?;

    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;

    if height > width {
        return Err(AtelierError::Platform(
            "Floating dock is only supported on horizontal taskbars. Disable it before moving the taskbar to a side of the screen.".to_string(),
        ));
    }

    let max_margin = ((width - 700) / 2).max(0);
    let margin = margin.clamp(0, max_margin);
    let radius = corner_radius.clamp(0, 32);
    let elevation = vertical_gap.clamp(0, 6);
    let dock_width = width - (margin * 2);
    let dock_height = height - elevation;

    if dock_width <= 0 || dock_height <= 0 {
        return Ok(());
    }

    let top_y = if hide_top_border { 1 } else { 0 };
    let rgn = unsafe {
        CreateRoundRectRgn(
            margin,
            top_y,
            width - margin,
            height - elevation,
            radius * 2,
            radius * 2,
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

    if let Ok(child) = unsafe {
        FindWindowExW(
            hwnd,
            HWND::default(),
            w!("Windows.UI.Composition.DesktopWindowContentBridge"),
            None,
        )
    } {
        if !child.0.is_null() {
            let _ = unsafe {
                SetWindowPos(
                    child,
                    HWND::default(),
                    margin,
                    0,
                    dock_width,
                    dock_height,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
        }
    }

    if let Ok(tray) = unsafe { FindWindowExW(hwnd, HWND::default(), w!("TrayNotifyWnd"), None) } {
        if !tray.0.is_null() {
            let mut tr = RECT::default();
            if unsafe { GetWindowRect(tray, &mut tr) }.is_ok() {
                let tw = tr.right - tr.left;
                let target_x = (width - margin - tw).max(margin);
                let _ = unsafe {
                    SetWindowPos(
                        tray,
                        HWND::default(),
                        target_x,
                        0,
                        tw,
                        dock_height,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    )
                };
            }
        }
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

pub fn reset_floating_dock(hwnd: HWND, hide_top_border: bool) -> Result<()> {
    let mut rect = RECT::default();
    let _ = unsafe { GetWindowRect(hwnd, &mut rect) };
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;

    if hide_top_border && width >= height {
        use windows::Win32::Graphics::Gdi::CreateRectRgn;
        let rgn = unsafe { CreateRectRgn(0, 1, width, height) };
        if !rgn.0.is_null() {
            let _ = unsafe { SetWindowRgn(hwnd, rgn, true) };
        }
    } else if unsafe { SetWindowRgn(hwnd, HRGN::default(), true) } == 0 {
        return Err(AtelierError::Platform(
            "Failed to clear the taskbar window region".to_string(),
        ));
    }

    if height > width {
        return Ok(());
    }

    if let Ok(child) = unsafe {
        FindWindowExW(
            hwnd,
            HWND::default(),
            w!("Windows.UI.Composition.DesktopWindowContentBridge"),
            None,
        )
    } {
        if !child.0.is_null() {
            let _ = unsafe {
                SetWindowPos(
                    child,
                    HWND::default(),
                    0,
                    0,
                    width,
                    height,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
            let _ = unsafe { SetWindowRgn(child, HRGN::default(), true) };
        }
    }

    if let Ok(tray) = unsafe { FindWindowExW(hwnd, HWND::default(), w!("TrayNotifyWnd"), None) } {
        if !tray.0.is_null() {
            let mut tr = RECT::default();
            if unsafe { GetWindowRect(tray, &mut tr) }.is_ok() {
                let tw = tr.right - tr.left;
                let th = tr.bottom - tr.top;
                let _ = unsafe {
                    SetWindowPos(
                        tray,
                        HWND::default(),
                        width - tw,
                        0,
                        tw,
                        th,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    )
                };
            }
        }
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
