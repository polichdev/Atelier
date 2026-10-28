use atelier_common::error::{AtelierError, Result};
use std::ffi::c_void;
use std::sync::OnceLock;
use windows::core::w;
use windows::Win32::Foundation::{BOOL, HWND};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

pub const ACCENT_DISABLED: u32 = 0;
pub const ACCENT_ENABLE_GRADIENT: u32 = 1;
pub const ACCENT_ENABLE_TRANSPARENTGRADIENT: u32 = 2;
pub const ACCENT_ENABLE_BLURBEHIND: u32 = 3;
pub const ACCENT_ENABLE_ACRYLICBLURBEHIND: u32 = 4;

const WCA_ACCENT_POLICY: u32 = 19;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AccentPolicy {
    pub accent_state: u32,
    pub accent_flags: u32,
    pub gradient_color: u32,
    pub animation_id: u32,
}

#[repr(C)]
struct WindowCompositionAttributeData {
    attribute: u32,
    data: *mut c_void,
    size_of_data: usize,
}

type SetWindowCompositionAttributeFn =
    unsafe extern "system" fn(HWND, *mut WindowCompositionAttributeData) -> BOOL;

static SET_WINDOW_COMPOSITION_ATTRIBUTE: OnceLock<Option<SetWindowCompositionAttributeFn>> =
    OnceLock::new();

fn get_set_window_composition_attribute() -> Option<SetWindowCompositionAttributeFn> {
    *SET_WINDOW_COMPOSITION_ATTRIBUTE.get_or_init(|| unsafe {
        let user32 = LoadLibraryW(w!("user32.dll")).ok()?;
        let proc = GetProcAddress(user32, windows::core::s!("SetWindowCompositionAttribute"))?;
        Some(std::mem::transmute::<
            unsafe extern "system" fn() -> isize,
            SetWindowCompositionAttributeFn,
        >(proc))
    })
}

pub fn set_taskbar_accent_policy(hwnd: HWND, policy: AccentPolicy) -> Result<()> {
    let func = get_set_window_composition_attribute().ok_or_else(|| {
        AtelierError::Platform("Failed to resolve SetWindowCompositionAttribute".to_string())
    })?;

    let mut policy = policy;
    let mut data = WindowCompositionAttributeData {
        attribute: WCA_ACCENT_POLICY,
        data: &mut policy as *mut _ as *mut c_void,
        size_of_data: std::mem::size_of::<AccentPolicy>(),
    };

    let result = unsafe { func(hwnd, &mut data) };
    if result.as_bool() {
        unsafe {
            use windows::Win32::Graphics::Gdi::{
                RedrawWindow, HRGN, RDW_ALLCHILDREN, RDW_ERASE, RDW_FRAME, RDW_INVALIDATE,
                RDW_UPDATENOW,
            };
            use windows::Win32::UI::WindowsAndMessaging::{
                SetWindowPos, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
                SWP_NOZORDER,
            };
            let _ = SetWindowPos(
                hwnd,
                HWND::default(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
            let _ = RedrawWindow(
                hwnd,
                None,
                HRGN::default(),
                RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_FRAME | RDW_UPDATENOW,
            );
        }
        Ok(())
    } else {
        Err(AtelierError::Platform(format!(
            "SetWindowCompositionAttribute returned FALSE: {}",
            std::io::Error::last_os_error()
        )))
    }
}
