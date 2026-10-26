use atelier_audio::AudioManager;
use atelier_common::config::{AtelierConfig, TaskbarConfig, TaskbarStyle};
use atelier_common::presets::ThemePreset;
use atelier_taskbar::TaskbarManager;
use atelier_wallpaper::WallpaperEngine;
use atelier_widget::WidgetSubsystem;
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

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
    tint: Option<i64>,
) -> Result<(), String> {
    let tint_u32 = tint.map(|t| t as u32);
    let style = match style_name.to_lowercase().as_str() {
        "clear" => TaskbarStyle::Clear,
        "blur" => TaskbarStyle::Blur,
        "acrylic" => TaskbarStyle::Acrylic {
            tint: tint_u32.unwrap_or(0xCC1A0033),
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

#[tauri::command]
fn hide_window(window: tauri::Window) {
    let _ = window.hide();
}

#[tauri::command]
fn get_system_status(state: tauri::State<AppState>) -> Result<serde_json::Value, String> {
    let audio_running = state.audio.lock().map(|a| a.is_active()).unwrap_or(false);
    let wallpaper_active = state.wallpaper.is_some();
    let is_fullscreen = state
        .wallpaper
        .as_ref()
        .and_then(|w| w.occlusion_state().ok())
        .map(|s| s != atelier_wallpaper::occlusion::OcclusionState::Active)
        .unwrap_or(false);

    Ok(serde_json::json!({
        "audio_active": audio_running,
        "wallpaper_active": wallpaper_active,
        "fullscreen_occluded": is_fullscreen,
        "widgets_active": true,
    }))
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
        .setup(|app| {
            let show_i = MenuItem::with_id(app, "show", "Open Theme Studio", true, None::<&str>)?;
            let minimal_i =
                MenuItem::with_id(app, "preset_minimal", "Minimal Clean", true, None::<&str>)?;
            let cyberpunk_i = MenuItem::with_id(
                app,
                "preset_cyberpunk",
                "Cyberpunk Neon",
                true,
                None::<&str>,
            )?;
            let frosted_i =
                MenuItem::with_id(app, "preset_frosted", "Frosted Glass", true, None::<&str>)?;
            let nordic_i =
                MenuItem::with_id(app, "preset_nordic", "Nordic Dark", true, None::<&str>)?;
            let reset_i =
                MenuItem::with_id(app, "reset", "Restore Windows Defaults", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit Atelier", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &show_i,
                    &minimal_i,
                    &cyberpunk_i,
                    &frosted_i,
                    &nordic_i,
                    &reset_i,
                    &quit_i,
                ],
            )?;

            let mut builder = TrayIconBuilder::new()
                .tooltip("Atelier Desktop Engine")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.unminimize();
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "preset_minimal" => {
                        let state = app.state::<AppState>();
                        let _ = apply_preset(state, "minimal clean".to_string());
                    }
                    "preset_cyberpunk" => {
                        let state = app.state::<AppState>();
                        let _ = apply_preset(state, "cyberpunk neon".to_string());
                    }
                    "preset_frosted" => {
                        let state = app.state::<AppState>();
                        let _ = apply_preset(state, "frosted glass".to_string());
                    }
                    "preset_nordic" => {
                        let state = app.state::<AppState>();
                        let _ = apply_preset(state, "nordic dark".to_string());
                    }
                    "reset" => {
                        let state = app.state::<AppState>();
                        let _ = reset_defaults(state);
                    }
                    "quit" => {
                        let state = app.state::<AppState>();
                        let _ = reset_defaults(state);
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                    | TrayIconEvent::DoubleClick {
                        button: MouseButton::Left,
                        ..
                    } = event
                    {
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            let _ = w.unminimize();
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                builder = builder.icon(icon.clone());
            }

            let _tray = builder.build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            apply_style,
            set_floating_dock,
            apply_preset,
            reset_defaults,
            hide_window,
            get_system_status,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!());
}
