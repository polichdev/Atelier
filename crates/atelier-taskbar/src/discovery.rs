use windows::core::w;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowExW, FindWindowW, GetClassNameW,
};

pub fn find_primary_taskbar() -> Option<HWND> {
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
        all = find_taskbars_by_enumeration();
    }

    all
}

fn find_taskbars_by_enumeration() -> Vec<HWND> {
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
