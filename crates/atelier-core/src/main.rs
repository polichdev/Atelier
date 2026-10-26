mod ipc;
mod tray;

use atelier_audio::AudioManager;
use atelier_common::config::{AtelierConfig, TaskbarConfig, TaskbarStyle};
use atelier_common::presets::ThemePreset;
use atelier_taskbar::TaskbarManager;
use atelier_wallpaper::WallpaperEngine;
use atelier_widget::WidgetSubsystem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tray::{SystemTrayManager, TrayAction};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting Atelier daemon...");

    let config = AtelierConfig {
        taskbar: TaskbarConfig {
            style: TaskbarStyle::Clear,
            floating_dock: false,
            floating_margin: 8,
        },
        ..Default::default()
    };

    let taskbar = Arc::new(TaskbarManager::new(config.taskbar));
    taskbar.apply_current_style()?;
    taskbar.start_recovery()?;

    let wallpaper = match WallpaperEngine::new(true) {
        Ok(engine) => {
            tracing::info!("Wallpaper engine initialized.");
            Some(Arc::new(engine))
        }
        Err(e) => {
            tracing::warn!("Wallpaper engine init deferred: {e}");
            None
        }
    };

    let audio = Arc::new(Mutex::new(AudioManager::new(0x00FF8800)));
    let _ = audio.lock().map(|mut a| a.start());

    let widget_subsystem = Arc::new(WidgetSubsystem::new());
    tracing::info!("Widget subsystem initialized.");

    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_clone = is_running.clone();

    let taskbar_clone = taskbar.clone();
    let wallpaper_clone = wallpaper.clone();
    let audio_clone = audio.clone();
    let _widget_clone = widget_subsystem.clone();

    let tray = SystemTrayManager::new(move |action| match action {
        TrayAction::OpenStudio => {
            tracing::info!("Tray action: Open Theme Studio");
        }
        TrayAction::ToggleWallpaper => {
            if let Some(wp) = &wallpaper_clone {
                let player = wp.video_player();
                if let Ok(p) = player.lock() {
                    if p.is_paused() {
                        p.resume();
                    } else {
                        p.pause();
                    }
                };
            }
        }
        TrayAction::ApplyPresetMinimal => {
            let preset = ThemePreset::minimal_clean();
            let _ = taskbar_clone.set_style(preset.taskbar.style);
            let _ = taskbar_clone
                .set_floating_dock(preset.taskbar.floating_dock, preset.taskbar.floating_margin);
        }
        TrayAction::ApplyPresetCyberpunk => {
            let preset = ThemePreset::cyberpunk_neon();
            let _ = taskbar_clone.set_style(preset.taskbar.style);
            let _ = taskbar_clone
                .set_floating_dock(preset.taskbar.floating_dock, preset.taskbar.floating_margin);
            if let Ok(a) = audio_clone.lock() {
                a.set_tint(preset.accent_color);
            }
        }
        TrayAction::ApplyPresetFrosted => {
            let preset = ThemePreset::frosted_glass();
            let _ = taskbar_clone.set_style(preset.taskbar.style);
            let _ = taskbar_clone
                .set_floating_dock(preset.taskbar.floating_dock, preset.taskbar.floating_margin);
        }
        TrayAction::ApplyPresetNordic => {
            let preset = ThemePreset::nordic_dark();
            let _ = taskbar_clone.set_style(preset.taskbar.style);
            let _ = taskbar_clone
                .set_floating_dock(preset.taskbar.floating_dock, preset.taskbar.floating_margin);
        }
        TrayAction::ResetDefaults => {
            let _ = taskbar_clone.reset_all();
        }
        TrayAction::Exit => {
            is_running_clone.store(false, Ordering::SeqCst);
        }
    })?;

    tracing::info!("Atelier background daemon is active in system tray.");

    let tray_thread = std::thread::spawn(move || {
        tray.run_loop();
    });

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let _ = tokio::signal::ctrl_c().await;
        is_running.store(false, Ordering::SeqCst);
    });

    tracing::info!("Shutting down Atelier daemon. Restoring OS defaults...");
    taskbar.stop_recovery()?;
    taskbar.reset_all()?;
    let _ = audio.lock().map(|mut a| a.stop());

    let _ = tray_thread.join();
    tracing::info!("Atelier daemon successfully terminated.");

    Ok(())
}
