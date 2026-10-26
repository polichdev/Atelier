use std::f32::consts::PI;

pub const FFT_SIZE: usize = 512;
pub const HALF_SIZE: usize = FFT_SIZE / 2;
pub const NUM_BINS: usize = 64;

#[derive(Debug, Clone, Copy)]
pub struct SpectrumData {
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    pub energy: f32,
    pub bins: [f32; NUM_BINS],
}

impl Default for SpectrumData {
    fn default() -> Self {
        Self {
            bass: 0.0,
            mid: 0.0,
            treble: 0.0,
            energy: 0.0,
            bins: [0.0; NUM_BINS],
        }
    }
}

pub struct SpectrumAnalyzer {
    window: [f32; FFT_SIZE],
    real: [f32; FFT_SIZE],
    imag: [f32; FFT_SIZE],
    smoothed_bins: [f32; NUM_BINS],
    sample_rate: f32,
    smoothing_factor: f32,
}

impl SpectrumAnalyzer {
    pub fn new(sample_rate: f32) -> Self {
        let mut window = [0.0; FFT_SIZE];
        for (i, item) in window.iter_mut().enumerate() {
            *item = 0.5 * (1.0 - (2.0 * PI * i as f32 / (FFT_SIZE as f32 - 1.0)).cos());
        }

        Self {
            window,
            real: [0.0; FFT_SIZE],
            imag: [0.0; FFT_SIZE],
            smoothed_bins: [0.0; NUM_BINS],
            sample_rate,
            smoothing_factor: 0.75,
        }
    }

    pub fn process(&mut self, pcm: &[f32]) -> SpectrumData {
        let count = pcm.len().min(FFT_SIZE);
        for (i, &sample) in pcm.iter().enumerate().take(count) {
            self.real[i] = sample * self.window[i];
            self.imag[i] = 0.0;
        }
        for i in count..FFT_SIZE {
            self.real[i] = 0.0;
            self.imag[i] = 0.0;
        }

        compute_radix2_fft(&mut self.real, &mut self.imag);

        let bin_width = self.sample_rate / FFT_SIZE as f32;
        let mut magnitudes = [0.0; HALF_SIZE];
        for (i, mag) in magnitudes.iter_mut().enumerate() {
            *mag = (self.real[i] * self.real[i] + self.imag[i] * self.imag[i]).sqrt()
                / (FFT_SIZE as f32 / 2.0);
        }

        let mut bass_sum = 0.0f32;
        let mut bass_count = 0usize;
        let mut mid_sum = 0.0f32;
        let mut mid_count = 0usize;
        let mut treble_sum = 0.0f32;
        let mut treble_count = 0usize;

        for (i, &mag) in magnitudes.iter().enumerate() {
            let freq = i as f32 * bin_width;
            if (20.0..250.0).contains(&freq) {
                bass_sum += mag;
                bass_count += 1;
            } else if (250.0..4000.0).contains(&freq) {
                mid_sum += mag;
                mid_count += 1;
            } else if (4000.0..=20000.0).contains(&freq) {
                treble_sum += mag;
                treble_count += 1;
            }
        }

        let bass = if bass_count > 0 {
            (bass_sum / bass_count as f32 * 4.0).min(1.0)
        } else {
            0.0
        };

        let mid = if mid_count > 0 {
            (mid_sum / mid_count as f32 * 6.0).min(1.0)
        } else {
            0.0
        };

        let treble = if treble_count > 0 {
            (treble_sum / treble_count as f32 * 8.0).min(1.0)
        } else {
            0.0
        };

        let energy = (bass * 0.5 + mid * 0.3 + treble * 0.2).min(1.0);

        let step = HALF_SIZE / NUM_BINS;
        for b in 0..NUM_BINS {
            let start = b * step;
            let end = (start + step).min(HALF_SIZE);
            let mut sum = 0.0;
            for mag in magnitudes[start..end].iter() {
                sum += mag;
            }
            let raw_val = (sum / (end - start).max(1) as f32 * 5.0).min(1.0);
            self.smoothed_bins[b] = self.smoothed_bins[b] * self.smoothing_factor
                + raw_val * (1.0 - self.smoothing_factor);
        }

        SpectrumData {
            bass,
            mid,
            treble,
            energy,
            bins: self.smoothed_bins,
        }
    }
}

fn compute_radix2_fft(real: &mut [f32; FFT_SIZE], imag: &mut [f32; FFT_SIZE]) {
    let mut j = 0usize;
    for i in 0..FFT_SIZE {
        if i < j {
            real.swap(i, j);
            imag.swap(i, j);
        }
        let mut k = FFT_SIZE >> 1;
        while k > 0 && k <= j {
            j -= k;
            k >>= 1;
        }
        j += k;
    }

    let mut len = 2usize;
    while len <= FFT_SIZE {
        let half = len >> 1;
        let angle = -2.0 * PI / len as f32;
        let w_step_real = angle.cos();
        let w_step_imag = angle.sin();

        let mut i = 0;
        while i < FFT_SIZE {
            let mut w_real = 1.0;
            let mut w_imag = 0.0;

            for k in 0..half {
                let pos = i + k;
                let match_pos = pos + half;

                let u_real = real[pos];
                let u_imag = imag[pos];

                let v_real = real[match_pos] * w_real - imag[match_pos] * w_imag;
                let v_imag = real[match_pos] * w_imag + imag[match_pos] * w_real;

                real[pos] = u_real + v_real;
                imag[pos] = u_imag + v_imag;

                real[match_pos] = u_real - v_real;
                imag[match_pos] = u_imag - v_imag;

                let next_w_real = w_real * w_step_real - w_imag * w_step_imag;
                let next_w_imag = w_real * w_step_imag + w_imag * w_step_real;
                w_real = next_w_real;
                w_imag = next_w_imag;
            }
            i += len;
        }
        len <<= 1;
    }
}
