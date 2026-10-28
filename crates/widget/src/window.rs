use atelier_common::error::{AtelierError, Result};
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, HDC,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, RegisterClassW,
    SetWindowLongPtrW, SetWindowPos, UpdateLayeredWindow, GWL_EXSTYLE, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, ULW_ALPHA, WINDOW_EX_STYLE, WINDOW_STYLE, WM_DESTROY, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
};

#[derive(Clone, Copy)]
pub struct SendHwnd(pub HWND);
unsafe impl Send for SendHwnd {}
unsafe impl Sync for SendHwnd {}

pub struct LayeredWidgetWindow {
    hwnd: SendHwnd,
    width: i32,
    height: i32,
    mem_dc: HDC,
    bitmap: HBITMAP,
    bits_ptr: *mut u32,
}

unsafe impl Send for LayeredWidgetWindow {}
unsafe impl Sync for LayeredWidgetWindow {}

impl LayeredWidgetWindow {
    pub fn new(
        title: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        click_through: bool,
    ) -> Result<Self> {
        let instance = unsafe { GetModuleHandleW(None).unwrap_or_default() };
        let class_name = w!("AtelierLayeredWidgetClass");

        let wnd_class = WNDCLASSW {
            lpfnWndProc: Some(widget_wnd_proc),
            hInstance: instance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };

        unsafe {
            let _ = RegisterClassW(&wnd_class);
        }

        let mut ex_style = WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST;
        if click_through {
            ex_style |= WS_EX_TRANSPARENT;
        }

        let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();

        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(ex_style.0),
                class_name,
                windows::core::PCWSTR(title_wide.as_ptr()),
                WINDOW_STYLE(WS_POPUP.0 | WS_VISIBLE.0),
                x,
                y,
                width,
                height,
                HWND::default(),
                None,
                instance,
                None,
            )
        }
        .map_err(|e| AtelierError::Platform(format!("Failed to create layered window: {e}")))?;

        let screen_dc = unsafe { GetDC(HWND::default()) };
        let mem_dc = unsafe { CreateCompatibleDC(screen_dc) };

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let bitmap =
            unsafe { CreateDIBSection(screen_dc, &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0) }
                .map_err(|e| {
                    AtelierError::Platform(format!("Failed to create DIB section: {e}"))
                })?;

        unsafe {
            let _ = SelectObject(mem_dc, bitmap);
            let _ = ReleaseDC(HWND::default(), screen_dc);
        }

        Ok(Self {
            hwnd: SendHwnd(hwnd),
            width,
            height,
            mem_dc,
            bitmap,
            bits_ptr: bits_ptr as *mut u32,
        })
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd.0
    }

    pub fn set_click_through(&self, enabled: bool) {
        let current_style = unsafe { GetWindowLongPtrW(self.hwnd.0, GWL_EXSTYLE) };
        let new_style = if enabled {
            current_style | (WS_EX_TRANSPARENT.0 as isize)
        } else {
            current_style & !(WS_EX_TRANSPARENT.0 as isize)
        };
        unsafe {
            let _ = SetWindowLongPtrW(self.hwnd.0, GWL_EXSTYLE, new_style);
            let _ = SetWindowPos(
                self.hwnd.0,
                HWND::default(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    pub fn buffer_mut(&mut self) -> &mut [u32] {
        let size = (self.width * self.height) as usize;
        unsafe { std::slice::from_raw_parts_mut(self.bits_ptr, size) }
    }

    pub fn update_surface(&self) -> Result<()> {
        let screen_dc = unsafe { GetDC(HWND::default()) };
        let pt_src = POINT { x: 0, y: 0 };
        let size = SIZE {
            cx: self.width,
            cy: self.height,
        };

        let blend = BLENDFUNCTION {
            BlendOp: 0,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: 1,
        };

        let res = unsafe {
            UpdateLayeredWindow(
                self.hwnd.0,
                screen_dc,
                None,
                Some(&size),
                self.mem_dc,
                Some(&pt_src),
                windows::Win32::Foundation::COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            )
        };

        unsafe {
            let _ = ReleaseDC(HWND::default(), screen_dc);
        }

        if res.is_ok() {
            Ok(())
        } else {
            Err(AtelierError::Platform(
                "UpdateLayeredWindow failed".to_string(),
            ))
        }
    }
}

impl Drop for LayeredWidgetWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.bitmap);
            let _ = DeleteDC(self.mem_dc);
            let _ = DestroyWindow(self.hwnd.0);
        }
    }
}

unsafe extern "system" fn widget_wnd_proc(
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
