# atelier-audio

Directory: [`crates/audio`](../../crates/audio/) · Cargo package: `atelier-audio`

`atelier-audio` is the real-time audio capture, spectral analysis, and reactivity engine for **Atelier**. It provides low-latency WASAPI loopback recording, Cooley-Tukey Radix-2 FFT computation with 64 logarithmic frequency bins, and zero-overhead idle power management.

```mermaid
flowchart LR
    subgraph Capture ["WASAPI Loopback Capture (capture.rs)"]
        Endpoint["Default Audio Endpoint<br/>(eRender, eMultimedia)"] --> IAudioClient["IAudioCaptureClient<br/>(Loopback Stream)"]
        IAudioClient --> Decoder["Sample Decoder<br/>(Float32 / PCM16 / PCM24 / PCM32)"]
    end

    subgraph FFT ["Spectral Analyzer (fft.rs)"]
        Decoder --> RingBuffer["Ring Buffer<br/>(512 Audio Samples)"]
        RingBuffer --> Window["Hanning Window<br/>(Leakage Reduction)"]
        Window --> Radix2["Radix-2 FFT Algorithm<br/>(Complex Frequency Domain)"]
        Radix2 --> LogBins["64 Logarithmic Bands<br/>(35 Hz to 16,000 Hz)"]
        LogBins --> Smoother["Exponential Smoother<br/>(Decay & Attack Filter)"]
    end

    subgraph Output ["Spectrum Data & Consumers"]
        Smoother --> SpectrumData["SpectrumData<br/>(energy, bass, mid, treble, bins[64])"]
        SpectrumData --> Visualizer["Desktop Visualizer Widget"]
        SpectrumData --> DockGlow["Desktop Dock Audio Glow"]
        SpectrumData --> Wallpaper["Live Wallpaper Pulse / Glow"]
    end
```

---

## Architectural Highlights

### 1. Robust WASAPI Loopback Capture
- Captures system audio directly from the default playback device without requiring virtual audio cables, stereo mix drivers, or microphone permissions.
- Automatically handles diverse endpoint formats (32-bit float, 16-bit integer, 24-bit packed integer, 32-bit integer).
- Sanitizes non-finite samples (`NaN`, `±Infinity`) to prevent numeric corruption in downstream filters.
- **Dynamic Device Switching**: Continuously monitors Windows audio endpoint GUIDs. If the user plugs in headphones or changes the default output device, the capture loop gracefully cleans up COM resources and rebinds to the new device without requiring an application restart.

---

### 2. Logarithmic FFT Frequency Binning
Human hearing perceives pitch logarithmically. Linear FFT bins over-represent high frequencies and under-represent bass. `atelier-audio` maps 512 discrete frequency bins into 64 musical bands using exponential scaling:

$$\text{Frequency}(b) = 35 \times \left(\frac{\min(f_s/2, 16000)}{35}\right)^{\frac{b}{64}}$$

```mermaid
flowchart TD
    subgraph Bands ["Frequency Band Partitioning"]
        SubBass["Sub-Bass & Bass (35 Hz - 250 Hz)<br/>Drives Wallpaper Pulse & Dock Glow"]
        MidRange["Midrange (250 Hz - 4,000 Hz)<br/>Vocals, Guitars, Synths"]
        Treble["Highs & Treble (4,000 Hz - 16,000 Hz)<br/>Cymbals, Percussive Attacks"]
    end
```

Peak energy tracking within each frequency partition prevents individual quiet frequencies from being washed out by loud adjacent frequencies.

---

### 3. On-Demand Capture & Zero-Overhead Idle Management
To avoid draining CPU cycles and laptop batteries when visualizers and audio-reactive effects are hidden:
- The capture thread is **not** kept running permanently.
- Calling `request_spectrum()` automatically spins up the WASAPI capture thread if it was stopped.
- A background supervisor executes `stop_if_idle(Duration::from_secs(3))`. If no widget or wallpaper requests spectrum data for 3 consecutive seconds, the worker thread uninitializes COM and suspends execution completely (0% CPU).

```mermaid
stateDiagram-v2
    [*] --> Idle : Application Start (Capture Stopped, 0% CPU)
    Idle --> Capturing : Consumer calls request_spectrum()
    Capturing --> Capturing : Active polling at 30-60 FPS
    Capturing --> Idle : No requests for 3 seconds (auto-stop)
```

---

## Crate Modules

| Module | Source File | Description |
| :--- | :--- | :--- |
| **WASAPI Capture** | [`src/capture.rs`](../../crates/audio/src/capture.rs) | WASAPI loopback thread, sample format decoders, silence detection, and endpoint change watchdog. |
| **Spectral Analysis** | [`src/fft.rs`](../../crates/audio/src/fft.rs) | In-place Radix-2 Cooley-Tukey FFT, Hanning window generator, 64 logarithmic bins, and exponential smoothing. |
| **Visualizer State** | [`src/visualizer.rs`](../../crates/audio/src/visualizer.rs) | Dynamic color tint derivation, energy beat detection, and pulse state management. |
| **Manager Facade** | [`src/lib.rs`](../../crates/audio/src/lib.rs) | `AudioManager` struct, lazy initialization, thread-safe spectrum access, and idle timeouts. |

---

## Testing

```powershell
# Run unit tests for atelier-audio (3 tests)
cargo test -p atelier-audio
```
Unit tests verify that pure tones separate into correct bass/treble bins and stay bounded, silence produces zero energy, and sample decoders discard non-finite inputs.
