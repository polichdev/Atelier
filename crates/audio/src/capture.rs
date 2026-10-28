use crate::fft::{SpectrumAnalyzer, SpectrumData, FFT_SIZE};
use atelier_common::error::{AtelierError, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use windows::Win32::Media::Audio::{
    eMultimedia, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator,
    MMDeviceEnumerator, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};

pub struct WasapiCapture {
    thread_handle: Option<JoinHandle<()>>,
    is_running: Arc<AtomicBool>,
    latest_spectrum: Arc<Mutex<SpectrumData>>,
}

impl WasapiCapture {
    pub fn start() -> Result<Self> {
        let is_running = Arc::new(AtomicBool::new(true));
        let is_running_clone = is_running.clone();
        let latest_spectrum = Arc::new(Mutex::new(SpectrumData::default()));
        let latest_spectrum_clone = latest_spectrum.clone();

        let (init_tx, init_rx) = std::sync::mpsc::channel();

        let thread_handle = std::thread::spawn(move || {
            let com_init = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            if com_init.is_err() {
                let _ = init_tx.send(Err(AtelierError::Platform(
                    "Failed to initialize COM for WASAPI capture".to_string(),
                )));
                return;
            }

            let capture_setup = (|| -> Result<(IAudioClient, IAudioCaptureClient, u32, usize)> {
                let enumerator: IMMDeviceEnumerator =
                    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }.map_err(
                        |e| {
                            AtelierError::Platform(format!(
                                "Failed to create MMDeviceEnumerator: {e}"
                            ))
                        },
                    )?;

                let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) }
                    .map_err(|e| {
                        AtelierError::Platform(format!("Failed to get default audio endpoint: {e}"))
                    })?;

                let audio_client: IAudioClient = unsafe { device.Activate(CLSCTX_ALL, None) }
                    .map_err(|e| {
                        AtelierError::Platform(format!("Failed to activate IAudioClient: {e}"))
                    })?;

                let pwfx = unsafe { audio_client.GetMixFormat() }.map_err(|e| {
                    AtelierError::Platform(format!("Failed to get mix format: {e}"))
                })?;

                let (sample_rate, channels) = unsafe {
                    let wfx = &*pwfx;
                    let channels = wfx.nChannels as usize;
                    let rate = wfx.nSamplesPerSec;
                    (rate, channels)
                };

                unsafe {
                    audio_client.Initialize(
                        AUDCLNT_SHAREMODE_SHARED,
                        AUDCLNT_STREAMFLAGS_LOOPBACK,
                        1000000,
                        0,
                        pwfx,
                        None,
                    )
                }
                .map_err(|e| {
                    AtelierError::Platform(format!("Failed to initialize audio client: {e}"))
                })?;

                let capture_client: IAudioCaptureClient = unsafe { audio_client.GetService() }
                    .map_err(|e| {
                        AtelierError::Platform(format!("Failed to get capture client: {e}"))
                    })?;

                unsafe { audio_client.Start() }.map_err(|e| {
                    AtelierError::Platform(format!("Failed to start audio client: {e}"))
                })?;

                Ok((audio_client, capture_client, sample_rate, channels))
            })();

            let (audio_client, capture_client, sample_rate, channels) = match capture_setup {
                Ok(tuple) => {
                    let _ = init_tx.send(Ok(()));
                    tuple
                }
                Err(e) => {
                    let _ = init_tx.send(Err(e));
                    unsafe { CoUninitialize() };
                    return;
                }
            };

            let mut analyzer = SpectrumAnalyzer::new(sample_rate as f32);
            let mut pcm_buffer = Vec::with_capacity(FFT_SIZE);

            while is_running_clone.load(Ordering::SeqCst) {
                let mut p_data = std::ptr::null_mut();
                let mut num_frames = 0u32;
                let mut flags = 0u32;

                let hr = unsafe {
                    capture_client.GetBuffer(&mut p_data, &mut num_frames, &mut flags, None, None)
                };

                if hr.is_ok() && num_frames > 0 && !p_data.is_null() {
                    let slice = unsafe {
                        std::slice::from_raw_parts(
                            p_data as *const f32,
                            (num_frames as usize) * channels,
                        )
                    };

                    for chunk in slice.chunks(channels) {
                        let mono = chunk.iter().sum::<f32>() / channels as f32;
                        pcm_buffer.push(mono);
                        if pcm_buffer.len() >= FFT_SIZE {
                            let spectrum = analyzer.process(&pcm_buffer);
                            if let Ok(mut lock) = latest_spectrum_clone.lock() {
                                *lock = spectrum;
                            }
                            pcm_buffer.drain(..FFT_SIZE / 2);
                        }
                    }

                    let _ = unsafe { capture_client.ReleaseBuffer(num_frames) };
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }

            let _ = unsafe { audio_client.Stop() };
            unsafe { CoUninitialize() };
        });

        init_rx.recv().map_err(|_| {
            AtelierError::Platform("Capture thread init channel dropped".to_string())
        })??;

        Ok(Self {
            thread_handle: Some(thread_handle),
            is_running,
            latest_spectrum,
        })
    }

    pub fn latest_spectrum(&self) -> SpectrumData {
        if let Ok(lock) = self.latest_spectrum.lock() {
            *lock
        } else {
            SpectrumData::default()
        }
    }

    pub fn stop(&mut self) {
        if self.is_running.swap(false, Ordering::SeqCst) {
            if let Some(handle) = self.thread_handle.take() {
                let _ = handle.join();
            }
        }
    }
}

impl Drop for WasapiCapture {
    fn drop(&mut self) {
        self.stop();
    }
}
