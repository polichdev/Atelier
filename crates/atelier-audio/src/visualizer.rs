use crate::fft::SpectrumData;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioPulseState {
    pub glow_alpha: u8,
    pub glow_color: u32,
    pub pulse_scale: f32,
    pub bass_level: f32,
    pub mid_level: f32,
}

pub struct AudioVisualizer {
    base_tint: u32,
    current_glow: f32,
    decay_rate: f32,
    attack_rate: f32,
}

impl AudioVisualizer {
    pub fn new(base_tint: u32) -> Self {
        Self {
            base_tint,
            current_glow: 0.0,
            decay_rate: 0.88,
            attack_rate: 0.35,
        }
    }

    pub fn set_base_tint(&mut self, tint: u32) {
        self.base_tint = tint;
    }

    pub fn update(&mut self, spectrum: &SpectrumData) -> AudioPulseState {
        let target_glow = (spectrum.bass * 1.4 + spectrum.mid * 0.6).min(1.0);

        if target_glow > self.current_glow {
            self.current_glow += (target_glow - self.current_glow) * self.attack_rate;
        } else {
            self.current_glow *= self.decay_rate;
        }

        let glow_alpha = (self.current_glow * 255.0) as u8;

        let b = (self.base_tint & 0xFF) as f32;
        let g = ((self.base_tint >> 8) & 0xFF) as f32;
        let r = ((self.base_tint >> 16) & 0xFF) as f32;

        let dynamic_r = (r * (1.0 + self.current_glow * 0.5)).min(255.0) as u32;
        let dynamic_g = (g * (1.0 + self.current_glow * 0.5)).min(255.0) as u32;
        let dynamic_b = (b * (1.0 + self.current_glow * 0.5)).min(255.0) as u32;

        let glow_color =
            (dynamic_b) | (dynamic_g << 8) | (dynamic_r << 16) | ((glow_alpha as u32) << 24);
        let pulse_scale = 1.0 + self.current_glow * 0.15;

        AudioPulseState {
            glow_alpha,
            glow_color,
            pulse_scale,
            bass_level: spectrum.bass,
            mid_level: spectrum.mid,
        }
    }
}

impl Default for AudioVisualizer {
    fn default() -> Self {
        Self::new(0x00FF8800)
    }
}
