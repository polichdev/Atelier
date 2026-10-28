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
}

impl AudioManager {
    pub fn new(base_tint: u32) -> Self {
        Self {
            capture: None,
            visualizer: Arc::new(Mutex::new(AudioVisualizer::new(base_tint))),
        }
    }

    pub fn start(&mut self) -> Result<()> {
        if self.capture.is_none() {
            let capture = WasapiCapture::start()?;
            self.capture = Some(capture);
        }
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.capture.is_some()
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
