pub mod occlusion;
pub mod renderer;
pub mod video;
pub mod workerw;

use atelier_common::error::Result;
use occlusion::{OcclusionDetector, OcclusionState};
use renderer::D3D11Renderer;
use std::sync::{Arc, Mutex};
use video::{MediaEngineContext, VideoWallpaperPlayer};
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
    occlusion: Arc<Mutex<OcclusionDetector>>,
    video_player: Arc<Mutex<VideoWallpaperPlayer>>,
    _media_context: Option<Arc<MediaEngineContext>>,
}

impl WallpaperEngine {
    pub fn new(pause_on_battery: bool) -> Result<Self> {
        let workerw = acquire_workerw()?;
        let host_window = create_wallpaper_host_window(workerw)?;

        let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) } as u32;
        let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) } as u32;

        let renderer = D3D11Renderer::new(host_window, width, height)?;
        let media_context = MediaEngineContext::new(renderer.device())
            .ok()
            .map(Arc::new);

        Ok(Self {
            workerw: SendHwnd(workerw),
            host_window: SendHwnd(host_window),
            renderer: Arc::new(Mutex::new(renderer)),
            occlusion: Arc::new(Mutex::new(OcclusionDetector::new(pause_on_battery))),
            video_player: Arc::new(Mutex::new(VideoWallpaperPlayer::new())),
            _media_context: media_context,
        })
    }

    pub fn workerw(&self) -> HWND {
        self.workerw.0
    }

    pub fn host_window(&self) -> HWND {
        self.host_window.0
    }

    pub fn occlusion_state(&self) -> Result<OcclusionState> {
        let detector = self
            .occlusion
            .lock()
            .map_err(|e| atelier_common::error::AtelierError::Generic(e.to_string()))?;
        Ok(detector.check_state())
    }

    pub fn video_player(&self) -> Arc<Mutex<VideoWallpaperPlayer>> {
        self.video_player.clone()
    }

    pub fn render_frame(&self, color: [f32; 4]) -> Result<bool> {
        let is_occluded = {
            let detector = self
                .occlusion
                .lock()
                .map_err(|e| atelier_common::error::AtelierError::Generic(e.to_string()))?;
            detector.should_pause()
        };

        if is_occluded {
            return Ok(false);
        }

        let renderer = self
            .renderer
            .lock()
            .map_err(|e| atelier_common::error::AtelierError::Generic(e.to_string()))?;
        renderer.clear(color);
        renderer.present()?;

        Ok(true)
    }
}

impl Drop for WallpaperEngine {
    fn drop(&mut self) {
        let _ = destroy_wallpaper_host_window(self.host_window.0);
    }
}
