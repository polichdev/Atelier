# Scene playback

Atelier can import downloaded Wallpaper Engine `scene.pkg` projects from the Workshop tab. Import creates a self-contained scene folder in the wallpaper library; playback does not require Steam or Wallpaper Engine to be running.

The renderer supports 2D image layers, inherited transforms and visibility, static MDLV0023 puppet meshes and attachment bind poses, opacity, normal/additive compositing, masks, basic water waves, ripples, flow, shake and pulse effects, and looping MP3/OGG/WAV audio. TEXB0001–0004 raw/LZ4 containers, PNG/JPEG embedded images, and RGBA/DXT/R8/RG88 textures are supported. Scaling, mute, volume and pause settings apply to scenes as well as videos. Skeletal animation tracks are not evaluated; compatible meshes stay in their bind pose.

Downloaded scene cards use cached compositions from original textures, up to 1920×1080. Imported scenes save the composition as `256.png`. Workshop video cards use the author preview when available and load the original local video for playback on hover; imported video cards load their local video as they approach the visible area. If a scene cannot be composed (for example an unsupported model or embedded image format), the Workshop card falls back to the author's preview. A small author image for a wallpaper that has not been downloaded still has its original resolution.

Scene settings are saved per wallpaper: animation speed (0–3×), hue, saturation, brightness, contrast and RGB tint. Color grading runs once after compositing, so it affects the entire scene. Updating these settings preserves playback time and loaded textures. Reset restores both these controls and author properties to their defaults.

Manifest version 3 retains the author's `project.json` properties and declarative bindings for layer visibility, transforms, opacity, color, supported effect values/visibility, camera shake and audio volume/visibility. Boolean, slider, color and combo controls are generated in author order. Conditional visibility retains alternate character variants even when disabled by default. Unsupported controls remain disabled with a reason; file selection, shortcuts, SceneScript and arbitrary custom shaders are not executed. Display conditions use a small expression parser, never `eval`. See the official [user property overview](https://docs.wallpaperengine.io/en/scene/userproperties/overview.html).

Selecting a version 1 or 2 library scene attempts to restore its properties from a uniquely matching title in the downloaded Steam collection. The exporter stages a complete replacement before swapping directories; the desktop player then reloads that active scene. If the original is missing or ambiguous, general scene controls still work and the UI explains why author properties are unavailable.

Each imported video or scene also has optional audio-reactive pulse and glow controls in its wallpaper settings. These are Atelier effects applied to the whole wallpaper, not the scene author’s audio-reactive scripts. They respond to the default system output, pause with playback, and share audio capture with the desktop visualizer.

The third Wallpapers tab, **Settings**, holds global playback and quality preferences. Focused/maximized/fullscreen applications, battery use, other audio, and executable rules support Continue, Pause or Mute. Fullscreen and battery defaults can follow the selected wallpaper's existing preferences. When fullscreen is inherited or set to pause and maximized is at its default, the per-wallpaper fullscreen pause also applies to maximized windows, including those above the dock's reserved gap. Rules override window policies; manual pause and battery/audio policies still apply. Other audio is detected from Core Audio session peak meters, with a three-second hold and exclusion of Atelier and its WebView2 child processes. No audio is recorded.

Quality presets set scene FPS, render scale, layer effects and FXAA. These controls also remain adjustable individually and apply without reloading the scene. Video playback keeps its source resolution and frame rate. Display-sleep unloading, MSAA and arbitrary application shortcuts are not implemented.

This is partial compatibility. Effects are implemented by Atelier and can look different. Composite layers support masked blur, bloom and god rays in source order; the built-in rain presets use an Atelier rain pass. Their author controls (including blur opacity and master/group switches) are preserved. Custom shaders, other particle systems, 3D models, skeletal animation, animated texture atlases and SceneScript are not implemented. Unsupported objects/effects are listed in the exported `scene.json` warnings. A scene with no supported image layers fails import rather than using its thumbnail as wallpaper.

Package reads, decoded asset sizes, image layers, texture memory and effect buffers have explicit limits. SceneScript and packaged shader source are never evaluated. The scene media protocol exposes only generated numeric image/audio names and the manifest, with range support for audio.

Debug builds include a native WebView2 GPU check:

```powershell
Atelier.exe --scene-engine-test path/to/scene.pkg path/to/new-test-directory/report.json
```

The check exports assets into the report directory, renders two animation timestamps, verifies image coverage, changed pixels, color grading and exact reset, tests supported combo/animation properties, checks GL errors, and writes a JSON report and PNG capture. It uses a separate Tauri instance without applying desktop wallpapers. Use a fresh report directory for each run. Binding, display-condition and timeline tests run with `node --test crates/ui/tests/scene-properties.test.cjs`.

`Atelier.exe --scene-preview-test path/to/scene.pkg output.png` checks the static cover compositor. Inspect both generated images: pixel coverage and animation checks alone do not establish that scene geometry is correct. The MDL format was cross-checked with the [Open Wallpaper Engine parser](https://github.com/waywallen/open-wallpaper-engine/blob/main/src/Scene/Pkg/Parse/Puppet/MdlParser.cpp).

`Atelier.exe --scene-settings-test path/to/downloaded/scene.pkg report.json` checks the real settings form and commands in an isolated data directory. For the Kirito/Asuna fixture, it verifies legacy upgrade, author overrides, slider persistence, preservation by general controls, the High quality preset, FXAA pixel changes, half-resolution rendering and fullscreen policy persistence. It does not apply a desktop wallpaper.

Desktop presentation explicitly keeps WebView2 visible after reparenting, notifies it of parent movement and redraws on resize. On raised Windows 11 desktops, the static WorkerW is placed below Atelier. Existing desktop parents are reused on reapply. WebGL retains the last frame while paused. Playback errors hide the failed surface, restore Windows' background and expose the reason plus a Retry button.

Debug checks also include --wallpaper-runtime-test <library-scene-directory> <report.json> for the complete controller/CSS with copied user settings, and --wallpaper-desktop-test for a temporary real desktop attachment. The latter changes the desktop for the test and restores the previous background before exiting. Both checks verify nonblack frames, retained paused frames and GL errors; the desktop check also reports native visibility and parent class.
