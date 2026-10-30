# Audio Reactivity

Atelier includes an advanced real-time audio reactivity subsystem powered by low-latency WASAPI loopback capture and a 64-band logarithmic FFT frequency analyzer. It can drive three independent visual consumers simultaneously.

```mermaid
flowchart LR
    subgraph AudioEngine ["atelier-audio Subsystem"]
        WASAPI["WASAPI Loopback<br/>(capture.rs)"] --> Samples["Raw Audio Samples<br/>(32-bit Float)"]
        Samples --> FFT["Radix-2 FFT (512 Samples)<br/>(fft.rs)"]
        FFT --> Bands["64 Logarithmic Bands<br/>(35 Hz to 16,000 Hz)"]
        Bands --> Metrics["Metrics Calculator<br/>(Bass, Mid, Treble, Peak Energy)"]
    end

    subgraph Consumers ["Independent Visual Consumers"]
        Metrics --> C1["1. Desktop Visualizer Widget<br/>(Customizable On-Screen Spectrum)"]
        Metrics --> C2["2. Desktop Dock Glow & Shape<br/>(Dynamic Base Glow & Breathing)"]
        Metrics --> C3["3. Wallpaper Pulse & Glow<br/>(Background Scene / Video Beat Reactivity)"]
    end
```

---

## Consumer Breakdown

### 1. Desktop Audio Visualizer Widget
An ambient, transparent desktop widget that visualizes system audio at 60 FPS:

```mermaid
flowchart TD
    subgraph Styles ["Visualizer Rendering Modes (audio-visualizer.js)"]
        Bars["Bars Mode<br/>Vertical frequency bars with gradients"]
        Wave["Wave Mode<br/>Smooth continuous bezier waveform"]
        Dots["Dots Mode<br/>Floating particles scaling with amplitude"]
        Circle["Circle Mode<br/>Radial circular spectrum radiating outward"]
        SymWave["Symmetric Wave<br/>Mirrored dual-sided audio oscillation"]
    end
```

- **Customization Options**:
  - **Color & Gradients**: Single solid color or two-tone linear gradients with glow intensity.
  - **Bar Count & Thickness**: Adjustable from 16 to 64 bars with variable spacing.
  - **Smoothing & Decay**: Tunable attack (fast rise on beats) and decay (smooth falling tail).
  - **Silence Hiding**: Automatically fades out the visualizer when no audio is playing, leaving the desktop clean while continuing to listen in the background.

---

### 2. Dock Reactivity
Provides ambient illumination behind the floating macOS-style dock:
- **Audio Glow**: Emits an ethereal colored glow beneath the dock container proportional to system audio volume.
- **Dynamic Deformations**:
  - `Breathing`: Expands dock vertical padding rhythmically on heavy bass hits.
  - `Corners`: Pulses dock corner radius between sharp and pill shapes on transient attacks.
- **Frequency Filtering**: Select whether the dock reacts strictly to low bass frequencies (kicks, 808s), midrange frequencies (vocals, instruments), or overall energy.

---

### 3. Wallpaper Reactivity
Applies ambient visual reactions to any active video or 2D scene wallpaper:
- **Audio Pulse**: Slightly scales the wallpaper image from the screen center on bass drops (beat detection).
- **Audio Glow**: Blends a radial luminance bloom across the wallpaper when peak audio energy exceeds threshold.

---

## Technical Specifications

| Parameter | Specification |
| :--- | :--- |
| **Capture API** | Windows Audio Session API (WASAPI) in loopback mode |
| **Endpoint** | Default audio render endpoint (`eRender`, `eMultimedia`) |
| **FFT Frame Size** | 512 samples per transform |
| **Window Function** | Periodic Hanning window ($w(n) = 0.5 - 0.5 \cos(2\pi n / N)$) |
| **Frequency Range** | 35 Hz to 16,000 Hz mapped across 64 logarithmic bands |
| **Idle Timeout** | Automatically stops WASAPI capture thread after 3s of silence/inactivity (0% CPU) |

---

## Troubleshooting Guide

```mermaid
flowchart TD
    Start["Visualizer / Glow Not Reacting"] --> Q1{"Is system audio playing through default speaker/headphone?"}
    Q1 -- "No" --> Fix1["Play audio through Windows default output device"]
    Q1 -- "Yes" --> Q2{"Is 'Show on desktop' or 'Dock reactivity' checked?"}
    Q2 -- "No" --> Fix2["Enable the toggle in Studio > Audio tab"]
    Q2 -- "Yes" --> Q3{"Is 'Hide when silent' enabled with volume too low?"}
    Q3 -- "Yes" --> Fix3["Increase audio volume or disable 'Hide when silent'"]
    Q3 -- "No" --> Q4{"Is a full-screen application currently focused?"}
    Q4 -- "Yes" --> Fix4["Disable 'Pause in fullscreen' if you want continuous visualizer"]
    Q4 -- "No" --> Fix5["Check Studio > Audio status indicator for audio endpoint errors"]
```
