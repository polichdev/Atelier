# wallpaper

Directory: [`crates/wallpaper`](../../crates/wallpaper/) · Cargo package: `atelier-wallpaper`

The wallpaper subsystem contains `workerw.rs` for desktop host windows, `renderer.rs` for Direct3D 11 rendering, `video.rs` for Media Foundation and player state, and `occlusion.rs` for pause decisions. `WallpaperEngine` connects these pieces and cleans up its host window on drop.

Studio attempts initialization, but its wallpaper tab is marked as under development. The presence of Media Foundation and player components does not establish a complete video selection, decoding and playback workflow.
