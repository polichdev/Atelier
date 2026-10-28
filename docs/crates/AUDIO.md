# atelier-audio

`capture.rs` starts a WASAPI loopback capture thread, `fft.rs` computes spectrum data, and `visualizer.rs` derives pulse state. `AudioManager` owns capture and exposes spectrum and tint operations. Studio attempts to start audio capture, but its audio tab and live visualizer controls remain under development.
