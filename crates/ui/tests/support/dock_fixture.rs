use std::{
    os::windows::process::CommandExt,
    process::Command,
    time::{Duration, Instant},
};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
};
unsafe extern "system" fn proc(window: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if message == WM_DESTROY {
        PostQuitMessage(0);
        return LRESULT(0);
    }
    DefWindowProcW(window, message, w, l)
}
fn window(class: PCWSTR, title: PCWSTR) -> windows::core::Result<HWND> {
    unsafe {
        let instance = GetModuleHandleW(None)?;
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: instance.into(),
            lpszClassName: class,
            ..Default::default()
        });
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            title,
            WS_OVERLAPPEDWINDOW,
            80,
            80,
            340,
            160,
            HWND::default(),
            None,
            instance,
            None,
        )
    }
}
fn pump() {
    unsafe {
        let mut m = MSG::default();
        while PeekMessageW(&mut m, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&m);
            DispatchMessageW(&m);
        }
    }
}
pub fn run() -> bool {
    let arg = std::env::args().nth(1).unwrap_or_default();
    if arg == "--dock-write-link" {
        use windows::{
            core::Interface,
            Win32::{
                System::Com::*,
                UI::Shell::{IShellLinkW, ShellLink},
            },
        };
        let path = std::env::args().nth(2).expect("shortcut path");
        let exe = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let wide: Vec<u16> = exe.encode_utf16().chain(Some(0)).collect();
        let destination: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
            let link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
            link.SetPath(PCWSTR(wide.as_ptr())).unwrap();
            link.SetArguments(w!("--dock-app-fixture")).unwrap();
            link.cast::<IPersistFile>()
                .unwrap()
                .Save(PCWSTR(destination.as_ptr()), true)
                .unwrap();
            CoUninitialize();
        }
        return true;
    }
    if arg == "--dock-owner-fixture" {
        std::thread::sleep(Duration::from_millis(700));
        return true;
    }
    if arg == "--dock-app-fixture" {
        let h = window(w!("AtelierDockTestWindow"), w!("Atelier Dock test window"))
            .expect("test window");
        unsafe {
            let _ = ShowWindow(h, SW_SHOWNOACTIVATE);
        }
        let deadline = Instant::now() + Duration::from_secs(40);
        while Instant::now() < deadline && unsafe { IsWindow(h) }.as_bool() {
            pump();
            std::thread::sleep(Duration::from_millis(10))
        }
        let _ = unsafe { DestroyWindow(h) };
        return true;
    }
    if arg != "--dock-recovery-test" {
        return false;
    }
    let run = || -> Result<(), String> {
        crate::desktop_dock::spawn_watchdog(0)?;
        println!("PASS: controller waits for guardian readiness before hiding a window");
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        for (class, expected) in [
            (w!("Shell_SecondaryTrayWnd"), true),
            (w!("AtelierRecoveryMustStayHidden"), false),
        ] {
            let h = window(class, w!("Atelier recovery fixture")).map_err(|e| e.to_string())?;
            let mut owner = Command::new(&exe)
                .arg("--dock-owner-fixture")
                .creation_flags(0x08000000)
                .spawn()
                .map_err(|e| e.to_string())?;
            let mut guardian = Command::new(&exe)
                .args([
                    "--dock-watchdog",
                    &owner.id().to_string(),
                    &(h.0 as isize).to_string(),
                ])
                .creation_flags(0x08000000)
                .spawn()
                .map_err(|e| e.to_string())?;
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut done = false;
            while Instant::now() < deadline {
                pump();
                if guardian.try_wait().map_err(|e| e.to_string())?.is_some() {
                    done = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(10))
            }
            let visible = unsafe { IsWindowVisible(h) }.as_bool();
            let _ = unsafe { DestroyWindow(h) };
            let _ = owner.wait();
            if !done {
                let _ = guardian.kill();
                return Err("guardian did not finish".into());
            }
            if visible != expected {
                return Err(format!(
                    "Unexpected recovery state: visible={visible}, expected={expected}"
                ));
            }
            println!(
                "PASS: guardian {}",
                if expected {
                    "restores a taskbar fixture after owner exit"
                } else {
                    "does not reveal unrelated windows"
                }
            );
        }
        Ok(())
    };
    if let Err(e) = run() {
        eprintln!("FAIL: {e}");
        std::process::exit(2)
    }
    true
}
