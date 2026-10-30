use atelier_common::error::{AtelierError, Result};
use windows::core::w;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, EnumWindows, FindWindowExW, FindWindowW,
    GetShellWindow, GetSystemMetrics, GetWindowLongPtrW, RegisterClassW, SendMessageTimeoutW,
    SetParent, GWL_EXSTYLE, SMTO_NORMAL, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_DESTROY, WNDCLASSW, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS,
    WS_EX_NOREDIRECTIONBITMAP, WS_VISIBLE,
};

pub fn acquire_workerw() -> Result<HWND> {
    let progman = unsafe {
        FindWindowW(w!("Progman"), None)
            .or_else(|_| FindWindowW(w!("Progman"), w!("Program Manager")))
            .or_else(|_| {
                let shell = GetShellWindow();
                if shell.0.is_null() {
                    Err(windows::core::Error::from_win32())
                } else {
                    Ok(shell)
                }
            })
    }
    .map_err(|e| AtelierError::Platform(format!("Failed to find Progman window: {e}")))?;

    if progman.0.is_null() {
        return Err(AtelierError::Platform("Progman handle is null".to_string()));
    }

    let mut result: usize = 0;
    let _ = unsafe {
        SendMessageTimeoutW(
            progman,
            0x052C,
            WPARAM(0x0000000D),
            LPARAM(1),
            SMTO_NORMAL,
            1000,
            Some(&mut result),
        )
    };
    let _ = unsafe {
        SendMessageTimeoutW(
            progman,
            0x052C,
            WPARAM(0x0000000D),
            LPARAM(0),
            SMTO_NORMAL,
            1000,
            Some(&mut result),
        )
    };

    if unsafe { GetWindowLongPtrW(progman, GWL_EXSTYLE) } & WS_EX_NOREDIRECTIONBITMAP.0 as isize
        != 0
    {
        return Ok(progman);
    }
    let mut workerw_hwnd = HWND::default();

    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let p_workerw = unsafe { &mut *(lparam.0 as *mut HWND) };
        let shelldll =
            unsafe { FindWindowExW(hwnd, HWND::default(), w!("SHELLDLL_DefView"), None) };

        if let Ok(shelldll_hwnd) = shelldll {
            if !shelldll_hwnd.0.is_null() {
                if let Ok(workerw) =
                    unsafe { FindWindowExW(HWND::default(), hwnd, w!("WorkerW"), None) }
                {
                    if !workerw.0.is_null() {
                        *p_workerw = workerw;
                        return BOOL(0);
                    }
                }
                if let Ok(workerw) =
                    unsafe { FindWindowExW(hwnd, HWND::default(), w!("WorkerW"), None) }
                {
                    if !workerw.0.is_null() {
                        *p_workerw = workerw;
                        return BOOL(0);
                    }
                }
            }
        }

        BOOL(1)
    }

    let _ = unsafe {
        EnumWindows(
            Some(enum_windows_proc),
            LPARAM(&mut workerw_hwnd as *mut HWND as isize),
        )
    };

    if workerw_hwnd.0.is_null() {
        return Err(AtelierError::Platform(
            "Could not locate the desktop wallpaper layer".into(),
        ));
    }

    Ok(workerw_hwnd)
}

pub fn create_wallpaper_host_window(workerw: HWND) -> Result<HWND> {
    let class_name = w!("AtelierWallpaperHostClass");
    let instance = unsafe { GetModuleHandleW(None).unwrap_or_default() };

    let wnd_class = WNDCLASSW {
        lpfnWndProc: Some(wallpaper_wnd_proc),
        hInstance: instance.into(),
        lpszClassName: class_name,
        ..Default::default()
    };

    unsafe {
        let _ = RegisterClassW(&wnd_class);
    }

    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("AtelierWallpaperHost"),
            WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_CLIPCHILDREN.0 | WS_CLIPSIBLINGS.0),
            0,
            0,
            width,
            height,
            workerw,
            None,
            instance,
            None,
        )
    }
    .map_err(|e| AtelierError::Platform(format!("Failed to create host window: {e}")))?;

    let _ = unsafe { SetParent(hwnd, workerw) };

    Ok(hwnd)
}

pub fn destroy_wallpaper_host_window(hwnd: HWND) -> Result<()> {
    if !hwnd.0.is_null() {
        unsafe { DestroyWindow(hwnd) }
            .map_err(|e| AtelierError::Platform(format!("Failed to destroy host window: {e}")))?;
    }
    Ok(())
}

unsafe extern "system" fn wallpaper_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => LRESULT(0),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
