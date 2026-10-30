pub mod capture;
pub mod fft;
pub mod visualizer;

use atelier_common::error::Result;
use capture::WasapiCapture;
use fft::SpectrumData;
use std::sync::{Arc, Mutex};
use visualizer::{AudioPulseState, AudioVisualizer};

pub struct AudioManager {
    capture: Option<WasapiCapture>,
    visualizer: Arc<Mutex<AudioVisualizer>>,
    last_request: Option<std::time::Instant>,
}

impl AudioManager {
    pub fn new(base_tint: u32) -> Self {
        Self {
            capture: None,
            last_request: None,
            visualizer: Arc::new(Mutex::new(AudioVisualizer::new(base_tint))),
        }
    }

    pub fn start(&mut self) -> Result<()> {
        if !self.is_active() {
            self.stop();
            let capture = WasapiCapture::start()?;
            self.capture = Some(capture);
        }
        Ok(())
    }

    pub fn request_spectrum(&mut self) -> Result<SpectrumData> {
        self.start()?;
        self.last_request = Some(std::time::Instant::now());
        Ok(self.spectrum())
    }

    pub fn stop_if_idle(&mut self, idle: std::time::Duration) {
        if self.last_request.is_some_and(|last| last.elapsed() >= idle) {
            self.stop();
        }
    }

    pub fn is_active(&self) -> bool {
        self.capture.as_ref().is_some_and(WasapiCapture::is_active)
    }

    pub fn spectrum(&self) -> SpectrumData {
        if let Some(c) = &self.capture {
            c.latest_spectrum()
        } else {
            SpectrumData::default()
        }
    }

    pub fn update_pulse(&self) -> AudioPulseState {
        let spectrum = self.spectrum();
        if let Ok(mut vis) = self.visualizer.lock() {
            vis.update(&spectrum)
        } else {
            AudioPulseState {
                glow_alpha: 0,
                glow_color: 0,
                pulse_scale: 1.0,
                bass_level: 0.0,
                mid_level: 0.0,
            }
        }
    }

    pub fn set_tint(&self, tint: u32) {
        if let Ok(mut vis) = self.visualizer.lock() {
            vis.set_base_tint(tint);
        }
    }

    pub fn stop(&mut self) {
        self.last_request = None;
        if let Some(mut c) = self.capture.take() {
            c.stop();
        }
    }
}

impl Default for AudioManager {
    fn default() -> Self {
        Self::new(0x00FF8800)
    }
}

impl Drop for AudioManager {
    fn drop(&mut self) {
        self.stop();
    }
}
