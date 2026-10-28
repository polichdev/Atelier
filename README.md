# Atelier 🎨

> Ultra-lightweight, hardware-accelerated Windows 10/11 customization suite and aesthetic workspace engine.

Atelier is a Windows 10/11 personalization project built with Rust and Tauri v2. Taskbar styling is connected to the Studio; wallpaper, audio, and widget controls are in progress.

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

## Desktop Studio

The **Taskbar** tab styles the real Explorer taskbar. Its native Windows 11 XAML controls adjust app icon size, button width, button corners, inactive opacity, running indicators, and hover scale. The native properties are preserved before modification and restored on reset or controller exit. Desktop Dock settings appear in a collapsible section below those controls. Older saved dock configurations require an explicit opt-in before that additional overlay can start again.


Atelier includes an independent **Desktop Dock**, enabled from the Taskbar tab. The dock launches pinned Windows shortcuts and switches between real running desktop windows. It supports bottom/top/left/right placement, monitor selection, alignment, 20–96 px icons, spacing, padding, edge distance, tint, opacity, corners, hover magnification, running indicators, active highlights, inactive opacity, labels, a clock, and auto-hide with an edge reveal strip. The macOS, Compact, and Side dock buttons apply complete starting layouts.

Pinned shortcuts are imported on first enable. Right-click a dock icon to open a new instance, pin/unpin it, or open Studio. Drag pinned icons to reorder them, or use the arrows in Studio. The **Icon** button on a pinned app accepts a local image, converts it to a small PNG, and stores it with the dock settings; **Reset icon** restores the application icon. Dock settings are stored separately in `dock-settings.json` beside Studio's other settings. The Windows reset action disables and resets Desktop Dock as well as restoring native taskbar styling.

Enabling Desktop Dock hides Explorer's taskbars. A separate recovery process is started and acknowledges readiness before any taskbar is hidden; it restores the original visible taskbar windows if Atelier is killed or crashes. Disabling the dock or quitting normally also restores them. Windows' notification area is not replicated in Desktop Dock, so disable the dock to access the native system tray. The custom dock does not alter Explorer's icon order, installed app icons, or the desktop work-area reservation.

Desktop dock verification:

- `cargo test -p atelier-ui --lib` checks configuration compatibility and placement, including scaling and negative monitor coordinates.
- `powershell -File scripts/test-desktop-dock.ps1` builds and runs an isolated native smoke test. It exercises real Tauri IPC, all four positions, auto-hide, shortcut launch, native window activation, pinning/order persistence, reset, and crash recovery against fixture windows. It uses a fresh directory under `target/` and never hides or restyles Explorer.
- `node --test scripts/tests/desktop-dock.test.cjs` tests the UI with Playwright. Set `NODE_PATH` if using a bundled Playwright install and `ATELIER_TEST_BROWSER=msedge` to use an installed Edge rather than Playwright's Chromium. Visual QA images are written under `target/dock-tests/`.

For development, launch with `.\scripts\run-dev.cmd`. The script closes a previous Atelier development process, builds the app, copies it to `target\dev-run\Atelier.exe`, and runs that copy. This leaves Cargo's `target\debug\Atelier.exe` free for subsequent IDE builds while the Studio is open. Launching `target\debug\Atelier.exe` directly will lock that file again.

For a portable setup or isolated testing, set `ATELIER_DATA_DIR` to a writable folder before launching Atelier. Settings will be stored there instead of `%APPDATA%\Atelier`.

Run the Tauri application with `cargo run -p atelier-ui --bin Atelier`. It starts with the Windows taskbar appearance and applies a style only when selected in Studio. Changes are saved at `%APPDATA%\Atelier\settings.json` and reapplied if Explorer recreates the taskbar during the current session; saved styles are not applied automatically on the next launch. On Windows 11 builds with a XAML taskbar background, the native composition policies may not change the visible background; the Studio detects that layer and reports the limitation. The tray menu can reopen the Studio or restore the Windows appearance.

Wallpaper, audio, and widget modules are present in the workspace, but their Studio controls are still being integrated. The Studio labels those sections accordingly instead of showing nonfunctional toggles. The separate `atelier-core` daemon currently opens the Studio HTML without Tauri IPC; use the Tauri application for functional controls.

---

## 📜 License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
