use windows::core::w;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::System::StationsAndDesktops::{
    OpenInputDesktop, SetThreadDesktop, DESKTOP_ACCESS_FLAGS, DF_ALLOWOTHERACCOUNTHOOK,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowExW, FindWindowW, GetClassNameW,
};

pub fn ensure_interactive_desktop() {
    unsafe {
        let access = DESKTOP_ACCESS_FLAGS(0x01FF);
        if let Ok(desk) = OpenInputDesktop(DF_ALLOWOTHERACCOUNTHOOK, false, access) {
            let _ = SetThreadDesktop(desk);
        }
    }
}

pub fn find_primary_taskbar() -> Option<HWND> {
    unsafe {
        if let Ok(hwnd) = FindWindowW(w!("Shell_TrayWnd"), None) {
            if !hwnd.0.is_null() {
                return Some(hwnd);
            }
        }
    }
    ensure_interactive_desktop();
    unsafe {
        let hwnd = FindWindowW(w!("Shell_TrayWnd"), None).ok()?;
        if hwnd.0.is_null() {
            None
        } else {
            Some(hwnd)
        }
    }
}

pub fn find_secondary_taskbars() -> Vec<HWND> {
    let mut taskbars = Vec::new();
    let mut current = HWND::default();

    loop {
        let hwnd = unsafe {
            FindWindowExW(HWND::default(), current, w!("Shell_SecondaryTrayWnd"), None).ok()
        };

        match hwnd {
            Some(h) if !h.0.is_null() => {
                taskbars.push(h);
                current = h;
            }
            _ => break,
        }
    }

    taskbars
}

pub fn find_all_taskbars() -> Vec<HWND> {
    let mut all = Vec::new();

    if let Some(primary) = find_primary_taskbar() {
        all.push(primary);
    }

    let mut secondaries = find_secondary_taskbars();
    all.append(&mut secondaries);

    if all.is_empty() {
        ensure_interactive_desktop();
        if let Some(primary) = find_primary_taskbar() {
            all.push(primary);
        }
        all.append(&mut find_secondary_taskbars());
        if all.is_empty() {
            all = find_taskbars_by_enumeration();
        }
    }

    all
}

pub fn has_xaml_background(hwnd: HWND) -> bool {
    unsafe {
        FindWindowExW(
            hwnd,
            HWND::default(),
            w!("Windows.UI.Composition.DesktopWindowContentBridge"),
            None,
        )
        .is_ok_and(|child| !child.0.is_null())
    }
}

fn find_taskbars_by_enumeration() -> Vec<HWND> {
    ensure_interactive_desktop();
    let mut taskbars = Vec::new();

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let taskbars = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
        let mut class_name = [0u16; 256];
        let len = unsafe { GetClassNameW(hwnd, &mut class_name) };

        if len > 0 {
            let class_str = String::from_utf16_lossy(&class_name[..len as usize]);
            if class_str == "Shell_TrayWnd" || class_str == "Shell_SecondaryTrayWnd" {
                taskbars.push(hwnd);
            }
        }

        BOOL(1)
    }

    let _ = unsafe {
        EnumWindows(
            Some(enum_proc),
            LPARAM(&mut taskbars as *mut Vec<HWND> as isize),
        )
    };

    taskbars
}
