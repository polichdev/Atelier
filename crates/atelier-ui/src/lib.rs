use atelier_audio::AudioManager;
use atelier_common::config::{AtelierConfig, TaskbarConfig, TaskbarStyle};
use atelier_common::presets::ThemePreset;
use atelier_taskbar::TaskbarManager;
use atelier_wallpaper::WallpaperEngine;
use atelier_widget::WidgetSubsystem;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub taskbar: Arc<TaskbarManager>,
    pub wallpaper: Option<Arc<WallpaperEngine>>,
    pub audio: Arc<Mutex<AudioManager>>,
    pub widget: Arc<WidgetSubsystem>,
}

#[tauri::command]
fn apply_style(
    state: tauri::State<AppState>,
    style_name: String,
    tint: Option<u32>,
) -> Result<(), String> {
    let style = match style_name.to_lowercase().as_str() {
        "clear" => TaskbarStyle::Clear,
        "blur" => TaskbarStyle::Blur,
        "acrylic" => TaskbarStyle::Acrylic {
            tint: tint.unwrap_or(0xCC1A0033),
        },
        "mica" => TaskbarStyle::Mica,
        _ => TaskbarStyle::Default,
    };

    state.taskbar.set_style(style).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_floating_dock(
    state: tauri::State<AppState>,
    enabled: bool,
    margin: i32,
) -> Result<(), String> {
    state
        .taskbar
        .set_floating_dock(enabled, margin)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn apply_preset(state: tauri::State<AppState>, name: String) -> Result<(), String> {
    let preset = match name.to_lowercase().as_str() {
        "minimal clean" | "minimal" => ThemePreset::minimal_clean(),
        "cyberpunk neon" | "cyberpunk" => ThemePreset::cyberpunk_neon(),
        "frosted glass" | "frosted" => ThemePreset::frosted_glass(),
        "nordic dark" | "nordic" => ThemePreset::nordic_dark(),
        _ => ThemePreset::minimal_clean(),
    };

    state
        .taskbar
        .set_style(preset.taskbar.style)
        .map_err(|e| e.to_string())?;

    state
        .taskbar
        .set_floating_dock(preset.taskbar.floating_dock, preset.taskbar.floating_margin)
        .map_err(|e| e.to_string())?;

    if let Ok(a) = state.audio.lock() {
        a.set_tint(preset.accent_color);
    }

    Ok(())
}

#[tauri::command]
fn reset_defaults(state: tauri::State<AppState>) -> Result<(), String> {
    state.taskbar.reset_all().map_err(|e| e.to_string())
}

pub fn run() {
    let config = AtelierConfig {
        taskbar: TaskbarConfig {
            style: TaskbarStyle::Clear,
            floating_dock: false,
            floating_margin: 8,
        },
        ..Default::default()
    };

    let taskbar = Arc::new(TaskbarManager::new(config.taskbar));
    let _ = taskbar.apply_current_style();
    let _ = taskbar.start_recovery();

    let wallpaper = WallpaperEngine::new(true).ok().map(Arc::new);

    let audio = Arc::new(Mutex::new(AudioManager::new(0x00FF8800)));
    let _ = audio.lock().map(|mut a| a.start());

    let widget = Arc::new(WidgetSubsystem::new());

    let state = AppState {
        taskbar,
        wallpaper,
        audio,
        widget,
    };

    let _ = tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            apply_style,
            set_floating_dock,
            apply_preset,
            reset_defaults,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!());
}
