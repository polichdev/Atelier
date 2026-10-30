# Wallpapers and Steam Workshop

Studio's **Wallpapers** page has **My wallpapers**, **Workshop** and **Settings** tabs. The library displays imported MP4/WebM videos and supported 2D scenes. Select an item to open its details and apply it, change its options or remove it from the library.

## Import a local video

1. Select **Import wallpaper** and choose an MP4 or WebM file.
2. After the import completes, select the wallpaper in **My wallpapers**.
3. Select **Apply** to start it on the desktop.

The player can fill, fit or stretch the image, adjust volume and mute, pause playback, and apply optional audio-reactive pulse and glow. Video playback loops. Atelier retains a frame during loop seeking and fades it into the next frame; a source whose end and beginning differ can still have a visible transition.

## Import a Workshop wallpaper

1. Open **Workshop** and select **Browse Workshop**.
2. Subscribe to a Wallpaper Engine wallpaper in Steam and let Steam download it.
3. Return to Atelier and select **Refresh downloads**.
4. Select the detected item and import a supported video or scene to the library, or use **Apply to Desktop** where available.

Atelier discovers files in local Steam libraries; it does not subscribe or download the wallpaper itself and does not use a bundled Depot downloader. A subscription alone does not guarantee that Steam has finished downloading the files. MP4/WebM videos and supported 2D scenes can be imported; web wallpapers and unsupported scene features cannot be assumed to work. Imported scenes are copied into the local library and do not require Steam to remain open for playback. See [Scene playback](SCENE_PLAYBACK.md) for details.

Search filters the current library or detected Workshop items. The gallery loads items in batches and defers some previews until they approach the visible area.

## Playback and performance

The selected wallpaper has pause, fullscreen/maximized and battery preferences. The **Settings** tab also provides playback policies for focused, maximized and fullscreen applications, battery use, other audio and executable-specific rules. Application rules override window policies; global fullscreen and battery choices can override the selected wallpaper's defaults. The per-wallpaper fullscreen option also pauses for maximized windows under the inherited or pause fullscreen policy when the maximized policy is at its default. Manual pause and battery/audio policies still apply.

Scene quality controls include FPS, render resolution, antialiasing and layer effects. They apply to scenes; videos retain their source resolution and frame rate. Scene settings can also expose supported author properties and color/animation controls. See [Scene playback](SCENE_PLAYBACK.md).

If a wallpaper reports a playback error, select **Retry** or reimport it. If a video has a visible loop transition, compare its first and last frames. Desktop attachment and recovery depend on Windows Explorer; see [Failure modes and recovery](FAILURES_AND_RECOVERY.md). Local files and settings are described in [Configuration](CONFIGURATION.md).
