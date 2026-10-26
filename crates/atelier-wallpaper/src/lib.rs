pub mod renderer;
pub mod workerw;

use atelier_common::error::Result;
use renderer::D3D11Renderer;
use std::sync::{Arc, Mutex};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
};
use workerw::{acquire_workerw, create_wallpaper_host_window, destroy_wallpaper_host_window};

#[derive(Clone, Copy)]
pub struct SendHwnd(pub HWND);
unsafe impl Send for SendHwnd {}
unsafe impl Sync for SendHwnd {}

pub struct WallpaperEngine {
    workerw: SendHwnd,
    host_window: SendHwnd,
    renderer: Arc<Mutex<D3D11Renderer>>,
}

impl WallpaperEngine {
    pub fn new() -> Result<Self> {
        let workerw = acquire_workerw()?;
        let host_window = create_wallpaper_host_window(workerw)?;

        let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) } as u32;
        let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) } as u32;

        let renderer = D3D11Renderer::new(host_window, width, height)?;

        Ok(Self {
            workerw: SendHwnd(workerw),
            host_window: SendHwnd(host_window),
            renderer: Arc::new(Mutex::new(renderer)),
        })
    }

    pub fn workerw(&self) -> HWND {
        self.workerw.0
    }

    pub fn host_window(&self) -> HWND {
        self.host_window.0
    }

    pub fn render_frame(&self, color: [f32; 4]) -> Result<()> {
        let renderer = self
            .renderer
            .lock()
            .map_err(|e| atelier_common::error::AtelierError::Generic(e.to_string()))?;
        renderer.clear(color);
        renderer.present()
    }
}

impl Drop for WallpaperEngine {
    fn drop(&mut self) {
        let _ = destroy_wallpaper_host_window(self.host_window.0);
    }
}
