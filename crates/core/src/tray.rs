use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetCursorPos, GetMessageW, LoadIconW, PostMessageW, PostQuitMessage,
    RegisterClassW, SetForegroundWindow, TrackPopupMenu, HWND_MESSAGE, IDI_APPLICATION,
    MF_SEPARATOR, MF_STRING, MSG, TPM_BOTTOMALIGN, TPM_LEFTALIGN, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_APP, WM_COMMAND, WM_DESTROY, WM_LBUTTONDBLCLK, WM_RBUTTONUP, WNDCLASSW,
};

pub const WM_TRAYICON: u32 = WM_APP + 201;

pub const IDM_STUDIO: usize = 1001;
pub const IDM_TOGGLE_WALLPAPER: usize = 1002;
pub const IDM_PRESET_MINIMAL: usize = 1003;
pub const IDM_PRESET_CYBERPUNK: usize = 1004;
pub const IDM_PRESET_FROSTED: usize = 1005;
pub const IDM_PRESET_NORDIC: usize = 1006;
pub const IDM_RESET_DEFAULTS: usize = 1007;
pub const IDM_EXIT: usize = 1008;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    OpenStudio,
    ToggleWallpaper,
    ApplyPresetMinimal,
    ApplyPresetCyberpunk,
    ApplyPresetFrosted,
    ApplyPresetNordic,
    ResetDefaults,
    Exit,
}

pub struct SystemTrayManager {
    hwnd: HWND,
    nid: NOTIFYICONDATAW,
    is_running: Arc<AtomicBool>,
}

unsafe impl Send for SystemTrayManager {}
unsafe impl Sync for SystemTrayManager {}

impl SystemTrayManager {
    pub fn new<F>(on_action: F) -> Result<Self>
    where
        F: Fn(TrayAction) + Send + Sync + 'static,
    {
        let instance = unsafe { GetModuleHandleW(None)? };
        let class_name = w!("AtelierSystemTrayClass");

        let wnd_class = WNDCLASSW {
            lpfnWndProc: Some(tray_wnd_proc),
            hInstance: instance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };

        unsafe {
            let _ = RegisterClassW(&wnd_class);
        }

        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                w!("AtelierSystemTray"),
                WINDOW_STYLE::default(),
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                None,
                instance,
                None,
            )
        }?;

        let callback = Arc::new(on_action);
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                hwnd,
                windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                Arc::into_raw(callback) as isize,
            );
        }

        let icon = unsafe { LoadIconW(None, IDI_APPLICATION)? };

        let mut tip = [0u16; 128];
        let tip_str = "Atelier Workspace Suite";
        for (i, c) in tip_str.encode_utf16().enumerate() {
            if i < 127 {
                tip[i] = c;
            }
        }

        let nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: icon,
            szTip: tip,
            ..Default::default()
        };

        let add_res = unsafe { Shell_NotifyIconW(NIM_ADD, &nid) };
        if !add_res.as_bool() {
            return Err(anyhow!("Failed to add notification icon"));
        }

        Ok(Self {
            hwnd,
            nid,
            is_running: Arc::new(AtomicBool::new(true)),
        })
    }

    pub fn run_loop(&self) {
        let mut msg = MSG::default();
        while self.is_running.load(Ordering::SeqCst) {
            let res = unsafe { GetMessageW(&mut msg, HWND::default(), 0, 0) };
            if res.0 <= 0 {
                break;
            }
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    pub fn shutdown(&mut self) {
        if self.is_running.swap(false, Ordering::SeqCst) {
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &self.nid);
                let _ = PostMessageW(
                    self.hwnd,
                    windows::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        }
    }
}

impl Drop for SystemTrayManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            let event = lparam.0 as u32;
            if event == WM_RBUTTONUP {
                show_tray_menu(hwnd);
            } else if event == WM_LBUTTONDBLCLK
                || event == windows::Win32::UI::WindowsAndMessaging::WM_LBUTTONUP
            {
                trigger_action(hwnd, TrayAction::OpenStudio);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = wparam.0 & 0xFFFF;
            match id {
                IDM_STUDIO => trigger_action(hwnd, TrayAction::OpenStudio),
                IDM_TOGGLE_WALLPAPER => trigger_action(hwnd, TrayAction::ToggleWallpaper),
                IDM_PRESET_MINIMAL => trigger_action(hwnd, TrayAction::ApplyPresetMinimal),
                IDM_PRESET_CYBERPUNK => trigger_action(hwnd, TrayAction::ApplyPresetCyberpunk),
                IDM_PRESET_FROSTED => trigger_action(hwnd, TrayAction::ApplyPresetFrosted),
                IDM_PRESET_NORDIC => trigger_action(hwnd, TrayAction::ApplyPresetNordic),
                IDM_RESET_DEFAULTS => trigger_action(hwnd, TrayAction::ResetDefaults),
                IDM_EXIT => {
                    trigger_action(hwnd, TrayAction::Exit);
                    PostQuitMessage(0);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let ptr = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
                hwnd,
                windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
            );
            if ptr != 0 {
                let _ = Arc::from_raw(ptr as *const Box<dyn Fn(TrayAction) + Send + Sync>);
            }
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn trigger_action(hwnd: HWND, action: TrayAction) {
    let ptr = unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
        )
    };
    if ptr != 0 {
        let callback = unsafe { &*(ptr as *const Arc<dyn Fn(TrayAction) + Send + Sync>) };
        callback(action);
    }
}

fn show_tray_menu(hwnd: HWND) {
    let mut pt = POINT::default();
    if unsafe { GetCursorPos(&mut pt) }.is_err() {
        return;
    }

    let menu = match unsafe { CreatePopupMenu() } {
        Ok(m) => m,
        Err(_) => return,
    };

    unsafe {
        let _ = SetForegroundWindow(hwnd);

        let _ = AppendMenuW(menu, MF_STRING, IDM_STUDIO, w!("Theme Studio..."));
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_TOGGLE_WALLPAPER,
            w!("Pause / Resume Wallpaper"),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_PRESET_MINIMAL,
            w!("Preset: Minimal Clean"),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_PRESET_CYBERPUNK,
            w!("Preset: Cyberpunk Neon"),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_PRESET_FROSTED,
            w!("Preset: Frosted Glass"),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_PRESET_NORDIC,
            w!("Preset: Nordic Dark"),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_RESET_DEFAULTS,
            w!("Restore Windows Defaults"),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(menu, MF_STRING, IDM_EXIT, w!("Exit Atelier"));

        let _ = TrackPopupMenu(
            menu,
            TPM_BOTTOMALIGN | TPM_LEFTALIGN,
            pt.x,
            pt.y,
            0,
            hwnd,
            None,
        );

        let _ = DestroyMenu(menu);
    }
}
