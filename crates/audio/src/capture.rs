use crate::fft::{SpectrumAnalyzer, SpectrumData, FFT_SIZE};
use atelier_common::error::{AtelierError, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use windows::core::GUID;
use windows::Win32::Media::Audio::{
    eMultimedia, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator,
    MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED,
    AUDCLNT_STREAMFLAGS_LOOPBACK, WAVEFORMATEXTENSIBLE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
    COINIT_MULTITHREADED,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum SampleFormat {
    Float32,
    Pcm16,
    Pcm24,
    Pcm32,
}

fn decode_sample(bytes: &[u8], format: SampleFormat) -> f32 {
    let value = match format {
        SampleFormat::Float32 => f32::from_le_bytes(bytes[..4].try_into().unwrap()),
        SampleFormat::Pcm16 => i16::from_le_bytes(bytes[..2].try_into().unwrap()) as f32 / 32768.0,
        SampleFormat::Pcm24 => {
            ((i32::from(bytes[0]) | i32::from(bytes[1]) << 8 | i32::from(bytes[2]) << 16) << 8 >> 8)
                as f32
                / 8388608.0
        }
        SampleFormat::Pcm32 => {
            i32::from_le_bytes(bytes[..4].try_into().unwrap()) as f32 / 2147483648.0
        }
    };
    if value.is_finite() {
        value.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

fn endpoint_id(enumerator: &IMMDeviceEnumerator) -> Option<String> {
    unsafe {
        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eMultimedia)
            .ok()?;
        let id = device.GetId().ok()?;
        let text = id.to_string().ok();
        CoTaskMemFree(Some(id.0.cast()));
        text
    }
}

pub struct WasapiCapture {
    thread_handle: Option<JoinHandle<()>>,
    is_running: Arc<AtomicBool>,
    latest_spectrum: Arc<Mutex<SpectrumData>>,
}

impl WasapiCapture {
    pub fn start() -> Result<Self> {
        let is_running = Arc::new(AtomicBool::new(true));
        let running = is_running.clone();
        let latest_spectrum = Arc::new(Mutex::new(SpectrumData::default()));
        let latest = latest_spectrum.clone();
        let (init_tx, init_rx) = std::sync::mpsc::channel();
        let thread_handle = std::thread::spawn(move || {
            if unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_err() {
                let _ = init_tx.send(Err(AtelierError::Platform(
                    "Failed to initialize audio".into(),
                )));
                running.store(false, Ordering::SeqCst);
                return;
            }
            let setup = (|| -> Result<(IMMDeviceEnumerator, String, IAudioClient, IAudioCaptureClient, u32, usize, usize, SampleFormat)> {
                let enumerator: IMMDeviceEnumerator = unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }.map_err(|e| AtelierError::Platform(e.to_string()))?;
                let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) }.map_err(|e| AtelierError::Platform(format!("No output device: {e}")))?;
                let id = endpoint_id(&enumerator).ok_or_else(|| AtelierError::Platform("Could not identify audio device".into()))?;
                let client: IAudioClient = unsafe { device.Activate(CLSCTX_ALL, None) }.map_err(|e| AtelierError::Platform(e.to_string()))?;
                let format = unsafe { client.GetMixFormat() }.map_err(|e| AtelierError::Platform(e.to_string()))?;
                let result = (|| -> Result<_> {
                    let info = unsafe { *format };
                    let channels = info.nChannels as usize;
                    let bits = info.wBitsPerSample;
                    let block = info.nBlockAlign as usize;
                    let subtype = if info.wFormatTag == 0xfffe && info.cbSize >= 22 {
                        let ext = unsafe { std::ptr::read_unaligned(format.cast::<WAVEFORMATEXTENSIBLE>()) };
                        ext.SubFormat
                    } else { GUID::from_u128((info.wFormatTag as u128) << 96 | 0x0000_0010_8000_00aa00389b71u128) };
                    let float = GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71);
                    let pcm = GUID::from_u128(0x00000001_0000_0010_8000_00aa00389b71);
                    let sample = match (subtype == float, subtype == pcm, bits) {
                        (true, _, 32) => SampleFormat::Float32,
                        (_, true, 16) => SampleFormat::Pcm16,
                        (_, true, 24) => SampleFormat::Pcm24,
                        (_, true, 32) => SampleFormat::Pcm32,
                        _ => return Err(AtelierError::Platform("Unsupported audio output format".into())),
                    };
                    if channels == 0 || block < channels * (bits as usize / 8) { return Err(AtelierError::Platform("Invalid audio output format".into())); }
                    unsafe { client.Initialize(AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, 1000000, 0, format, None) }.map_err(|e| AtelierError::Platform(e.to_string()))?;
                    Ok((info.nSamplesPerSec, channels, block, sample))
                })();
                unsafe { CoTaskMemFree(Some(format.cast())) };
                let (rate, channels, block, sample) = result?;
                let capture: IAudioCaptureClient = unsafe { client.GetService() }.map_err(|e| AtelierError::Platform(e.to_string()))?;
                unsafe { client.Start() }.map_err(|e| AtelierError::Platform(e.to_string()))?;
                Ok((enumerator, id, client, capture, rate, channels, block, sample))
            })();
            let (enumerator, device_id, client, capture, rate, channels, block, format) =
                match setup {
                    Ok(v) => {
                        let _ = init_tx.send(Ok(()));
                        v
                    }
                    Err(e) => {
                        let _ = init_tx.send(Err(e));
                        running.store(false, Ordering::SeqCst);
                        unsafe { CoUninitialize() };
                        return;
                    }
                };
            let mut analyzer = SpectrumAnalyzer::new(rate as f32);
            let mut pcm = Vec::with_capacity(FFT_SIZE);
            let mut last_audio = Instant::now();
            let mut in_silence = false;
            let mut last_device_check = Instant::now();
            let sample_bytes = match format {
                SampleFormat::Pcm16 => 2,
                SampleFormat::Pcm24 => 3,
                _ => 4,
            };
            while running.load(Ordering::SeqCst) {
                if last_device_check.elapsed() >= Duration::from_secs(2) {
                    last_device_check = Instant::now();
                    if endpoint_id(&enumerator).as_deref() != Some(device_id.as_str()) {
                        break;
                    }
                }
                let mut data = std::ptr::null_mut();
                let mut frames = 0u32;
                let mut flags = 0u32;
                match unsafe { capture.GetBuffer(&mut data, &mut frames, &mut flags, None, None) } {
                    Ok(()) if frames > 0 => {
                        let silent = flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0;
                        if !silent && data.is_null() {
                            let _ = unsafe { capture.ReleaseBuffer(frames) };
                            break;
                        }
                        let bytes = if silent {
                            None
                        } else {
                            Some(unsafe {
                                std::slice::from_raw_parts(data, frames as usize * block)
                            })
                        };
                        for frame in 0..frames as usize {
                            let value = bytes
                                .map(|bytes| {
                                    (0..channels)
                                        .map(|channel| {
                                            let at = frame * block + channel * sample_bytes;
                                            decode_sample(&bytes[at..at + sample_bytes], format)
                                        })
                                        .sum::<f32>()
                                        / channels as f32
                                })
                                .unwrap_or(0.0);
                            pcm.push(value);
                            if pcm.len() >= FFT_SIZE {
                                let spectrum = analyzer.process(&pcm);
                                if let Ok(mut lock) = latest.lock() {
                                    *lock = spectrum;
                                }
                                pcm.drain(..FFT_SIZE / 2);
                            }
                        }
                        last_audio = Instant::now();
                        in_silence = false;
                        if unsafe { capture.ReleaseBuffer(frames) }.is_err() {
                            break;
                        }
                    }
                    Ok(()) => {
                        if !in_silence && last_audio.elapsed() >= Duration::from_millis(250) {
                            in_silence = true;
                            analyzer = SpectrumAnalyzer::new(rate as f32);
                            if let Ok(mut lock) = latest.lock() {
                                *lock = SpectrumData::default();
                            }
                            pcm.clear();
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
            if let Ok(mut lock) = latest.lock() {
                *lock = SpectrumData::default();
            }
            let _ = unsafe { client.Stop() };
            running.store(false, Ordering::SeqCst);
            drop(capture);
            drop(client);
            drop(enumerator);
            unsafe { CoUninitialize() };
        });
        init_rx
            .recv()
            .map_err(|_| AtelierError::Platform("Audio capture failed to start".into()))??;
        Ok(Self {
            thread_handle: Some(thread_handle),
            is_running,
            latest_spectrum,
        })
    }
    pub fn is_active(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
    pub fn latest_spectrum(&self) -> SpectrumData {
        self.latest_spectrum.lock().map(|s| *s).unwrap_or_default()
    }
    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}
impl Drop for WasapiCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_supported_audio_and_rejects_non_finite_samples() {
        assert_eq!(
            decode_sample(&(-32768i16).to_le_bytes(), SampleFormat::Pcm16),
            -1.0
        );
        assert_eq!(decode_sample(&[0, 0, 128], SampleFormat::Pcm24), -1.0);
        assert_eq!(
            decode_sample(&i32::MIN.to_le_bytes(), SampleFormat::Pcm32),
            -1.0
        );
        assert_eq!(
            decode_sample(&f32::NAN.to_le_bytes(), SampleFormat::Float32),
            0.0
        );
        assert_eq!(
            decode_sample(&0.5f32.to_le_bytes(), SampleFormat::Float32),
            0.5
        );
    }
}
