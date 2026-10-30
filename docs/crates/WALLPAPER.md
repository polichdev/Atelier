# wallpaper

Directory: [`crates/wallpaper`](../../crates/wallpaper/) · Cargo package: `atelier-wallpaper`

The wallpaper subsystem contains `workerw.rs` for desktop host windows, `renderer.rs` for Direct3D 11 rendering, `video.rs` for Media Foundation and player state, and `occlusion.rs` for pause decisions. `WallpaperEngine` connects these pieces and cleans up its host window on drop.

Studio's Wallpapers tab imports MP4/WebM video files and supported 2D scenes, applies wallpapers to the desktop and saves per-wallpaper and global playback settings. The Workshop tab discovers items already downloaded by Steam and imports supported video and scene items; it does not download them directly. The Tauri player and library implementation live in `crates/ui/src/desktop_wallpaper.rs`, `steam_workshop.rs` and the associated frontend files.

Scene playback is a partial implementation of Wallpaper Engine formats; see [Scene playback](../SCENE_PLAYBACK.md) for supported features and limitations.
