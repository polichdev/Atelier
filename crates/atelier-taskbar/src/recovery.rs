use atelier_common::error::{AtelierError, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, PostMessageW,
    RegisterClassW, RegisterWindowMessageW, TranslateMessage, HWND_MESSAGE, MSG, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_APP, WM_DESTROY, WNDCLASSW,
};

const WM_STOP_LISTENER: u32 = WM_APP + 101;

#[derive(Clone, Copy)]
pub struct SendHwnd(pub HWND);
unsafe impl Send for SendHwnd {}
unsafe impl Sync for SendHwnd {}

pub struct TaskbarRecoveryListener {
    thread_handle: Option<JoinHandle<()>>,
    window_hwnd: SendHwnd,
    is_running: Arc<AtomicBool>,
}

impl TaskbarRecoveryListener {
    pub fn start<F>(on_taskbar_created: F) -> Result<Self>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let is_running = Arc::new(AtomicBool::new(true));
        let is_running_clone = is_running.clone();
        let (hwnd_sender, hwnd_receiver) = std::sync::mpsc::channel();
        let callback = Arc::new(on_taskbar_created);

        let thread_handle = std::thread::spawn(move || {
            let taskbar_created_msg = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
            if taskbar_created_msg == 0 {
                let _ = hwnd_sender.send(SendHwnd(HWND::default()));
                return;
            }

            let class_name = w!("AtelierRecoveryWindowClass");
            let instance = unsafe { GetModuleHandleW(None).unwrap_or_default() };

            let wnd_class = WNDCLASSW {
                lpfnWndProc: Some(recovery_wnd_proc),
                hInstance: instance.into(),
                lpszClassName: class_name,
                ..Default::default()
            };

            unsafe {
                let _ = RegisterClassW(&wnd_class);
            }

            let hwnd = match unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    class_name,
                    w!("AtelierRecoveryWindow"),
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
            } {
                Ok(h) if !h.0.is_null() => h,
                _ => {
                    let _ = hwnd_sender.send(SendHwnd(HWND::default()));
                    return;
                }
            };

            let _ = hwnd_sender.send(SendHwnd(hwnd));

            let mut msg = MSG::default();
            while is_running_clone.load(Ordering::SeqCst) {
                let res = unsafe { GetMessageW(&mut msg, HWND::default(), 0, 0) };
                if res.0 <= 0 {
                    break;
                }

                if msg.message == WM_STOP_LISTENER {
                    break;
                }

                if msg.message == taskbar_created_msg {
                    callback();
                }

                unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        });

        let hwnd_wrapper = hwnd_receiver.recv().map_err(|_| {
            AtelierError::Platform("Failed to receive recovery listener HWND".to_string())
        })?;

        if hwnd_wrapper.0 .0.is_null() {
            return Err(AtelierError::Platform(
                "Failed to create recovery listener window".to_string(),
            ));
        }

        Ok(Self {
            thread_handle: Some(thread_handle),
            window_hwnd: hwnd_wrapper,
            is_running,
        })
    }

    pub fn stop(&mut self) {
        if self.is_running.swap(false, Ordering::SeqCst) {
            unsafe {
                let _ = PostMessageW(self.window_hwnd.0, WM_STOP_LISTENER, WPARAM(0), LPARAM(0));
            }

            if let Some(handle) = self.thread_handle.take() {
                let _ = handle.join();
            }
        }
    }
}

impl Drop for TaskbarRecoveryListener {
    fn drop(&mut self) {
        self.stop();
    }
}

unsafe extern "system" fn recovery_wnd_proc(
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
