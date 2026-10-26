use atelier_common::error::{AtelierError, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D11::ID3D11Device;
use windows::Win32::Media::MediaFoundation::{
    IMFDXGIDeviceManager, MFCreateDXGIDeviceManager, MFShutdown, MFStartup, MF_VERSION,
};

pub struct MediaEngineContext {
    device_manager: IMFDXGIDeviceManager,
    reset_token: u32,
    initialized: bool,
}

impl MediaEngineContext {
    pub fn new(device: &ID3D11Device) -> Result<Self> {
        unsafe { MFStartup(MF_VERSION, 0) }.map_err(|e| {
            AtelierError::Platform(format!("Failed to initialize Media Foundation: {e}"))
        })?;

        let mut reset_token = 0u32;
        let mut device_manager_opt: Option<IMFDXGIDeviceManager> = None;

        unsafe { MFCreateDXGIDeviceManager(&mut reset_token, &mut device_manager_opt) }.map_err(
            |e| AtelierError::Platform(format!("Failed to create DXGI device manager: {e}")),
        )?;

        let device_manager = device_manager_opt
            .ok_or_else(|| AtelierError::Platform("DXGI device manager is null".to_string()))?;

        let unknown_device: windows::core::IUnknown = device
            .cast()
            .map_err(|e| AtelierError::Platform(format!("Failed to cast D3D11 device: {e}")))?;

        unsafe { device_manager.ResetDevice(&unknown_device, reset_token) }.map_err(|e| {
            AtelierError::Platform(format!("Failed to reset D3D11 device in manager: {e}"))
        })?;

        Ok(Self {
            device_manager,
            reset_token,
            initialized: true,
        })
    }

    pub fn device_manager(&self) -> &IMFDXGIDeviceManager {
        &self.device_manager
    }

    pub fn reset_token(&self) -> u32 {
        self.reset_token
    }
}

impl Drop for MediaEngineContext {
    fn drop(&mut self) {
        if self.initialized {
            let _ = unsafe { MFShutdown() };
        }
    }
}

pub struct VideoWallpaperPlayer {
    source_path: Option<PathBuf>,
    is_playing: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    loop_video: bool,
}

impl VideoWallpaperPlayer {
    pub fn new() -> Self {
        Self {
            source_path: None,
            is_playing: Arc::new(AtomicBool::new(false)),
            is_paused: Arc::new(AtomicBool::new(false)),
            loop_video: true,
        }
    }

    pub fn load_video<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let p = path.as_ref();
        if !p.exists() {
            return Err(AtelierError::Platform(format!(
                "Video wallpaper file does not exist: {}",
                p.display()
            )));
        }

        self.source_path = Some(p.to_path_buf());
        self.is_playing.store(true, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        Ok(())
    }

    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::SeqCst)
    }

    pub fn is_looping(&self) -> bool {
        self.loop_video
    }

    pub fn set_looping(&mut self, loop_video: bool) {
        self.loop_video = loop_video;
    }

    pub fn current_source(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }
}

impl Default for VideoWallpaperPlayer {
    fn default() -> Self {
        Self::new()
    }
}
