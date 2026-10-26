# Atelier 🎨

> Ultra-lightweight, hardware-accelerated Windows 10/11 customization suite and aesthetic workspace engine.

Atelier unifies taskbar morphing, live video/shader wallpapers, audio-reactive glow, and glass widgets into a single high-performance native core (< 30 MB RAM) built with Rust, Direct3D 11, and Tauri v2.

---

## 📦 Workspace Layout

```
atelier/
├── crates/
│   ├── atelier-common/     # Shared types, error definitions, and config models
│   ├── atelier-core/       # Central native daemon, state engine & Win32 event loop
│   ├── atelier-taskbar/    # Taskbar morphing (Clear, Acrylic, Mica, Floating Island)
│   ├── atelier-wallpaper/  # Direct3D 11 live wallpaper engine & WorkerW injection
│   ├── atelier-audio/      # WASAPI loopback capture & real-time FFT spectrum
│   └── atelier-widget/     # Frameless click-through glass desktop widgets
├── Cargo.toml              # Cargo workspace configuration
└── README.md
```

---

## 📜 License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
