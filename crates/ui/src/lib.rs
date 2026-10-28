use atelier_audio::AudioManager;
use atelier_common::config::{AtelierConfig, TaskbarConfig, TaskbarStyle};
use atelier_common::presets::ThemePreset;
use atelier_taskbar::TaskbarManager;
use atelier_wallpaper::WallpaperEngine;
use atelier_widget::WidgetSubsystem;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;
mod desktop_dock;
pub fn watchdog_args() -> bool {
    atelier_taskbar::position::guardian_args() || desktop_dock::watchdog_args()
}
#[cfg(debug_assertions)]
#[path = "../tests/support/dock_fixture.rs"]
mod dock_fixture;
pub fn desktop_test_args() -> bool {
    #[cfg(debug_assertions)]
    {
        dock_fixture::run()
    }
    #[cfg(not(debug_assertions))]
    {
        false
    }
}
fn dock_test_mode() -> bool {
    cfg!(debug_assertions) && std::env::var_os("ATELIER_DOCK_ONLY_TEST").is_some()
}

pub struct AppState {
    pub taskbar: Arc<TaskbarManager>,
    pub wallpaper: Option<Arc<WallpaperEngine>>,
    pub audio: Arc<Mutex<AudioManager>>,
    pub widget: Arc<WidgetSubsystem>,
}

fn settings_path() -> Option<PathBuf> {
    let root = std::env::var_os("ATELIER_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(|root| PathBuf::from(root).join("Atelier")))?;
    Some(root.join("settings.json"))
}

fn save_settings(config: &TaskbarConfig) -> Result<(), String> {
    let path = settings_path().ok_or("Atelier data directory is unavailable")?;
    std::fs::create_dir_all(path.parent().ok_or("Invalid settings path")?)
        .map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_taskbar_config(state: tauri::State<AppState>) -> Result<TaskbarConfig, String> {
    state.taskbar.config().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_taskbar_capabilities() -> serde_json::Value {
    let taskbars = atelier_taskbar::discovery::find_all_taskbars();
    let has_xaml = taskbars
        .iter()
        .copied()
        .any(atelier_taskbar::discovery::has_xaml_background);
    let xaml_unsupported = has_xaml && !atelier_taskbar::xaml::is_supported();
    serde_json::json!({
        "taskbar_count": taskbars.len(),
        "xaml_background": xaml_unsupported,
        "native_appearance": has_xaml && atelier_taskbar::xaml::is_supported(),
    })
}

#[tauri::command]
fn set_taskbar_config(
    state: tauri::State<AppState>,
    config: TaskbarConfig,
) -> Result<TaskbarConfig, String> {
    let mut config = config;
    config.floating_margin = config.floating_margin.clamp(0, 600);
    config.corner_radius = config.corner_radius.clamp(0, 36);
    config.vertical_gap = config.vertical_gap.clamp(0, 24);
    config.appearance.normalize();
    state
        .taskbar
        .set_config(config.clone())
        .map_err(|e| e.to_string())?;
    save_settings(&config)?;
    Ok(config)
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

    state.taskbar.set_style(style).map_err(|e| e.to_string())?;
    save_settings(&state.taskbar.config().map_err(|e| e.to_string())?)
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
        .map_err(|e| e.to_string())?;
    save_settings(&state.taskbar.config().map_err(|e| e.to_string())?)
}

#[tauri::command]
fn apply_preset(state: tauri::State<AppState>, name: String) -> Result<(), String> {
    let preset = match name.to_lowercase().as_str() {
        "macos sonoma" | "sonoma" => ThemePreset::macos_sonoma(),
        "macos big sur" | "big sur" => ThemePreset::macos_big_sur(),
        "cupertino dark" | "dark dock" => ThemePreset::cupertino_dark(),
        "cupertino light" | "light dock" => ThemePreset::cupertino_light(),
        "clean glass" | "clean" => ThemePreset::clean_glass(),
        "windows default" | "windows" | "stock" => ThemePreset::windows_default(),
        "minimal clean" | "minimal" => ThemePreset::minimal_clean(),
        "cyberpunk neon" | "cyberpunk" | "neon" => ThemePreset::cyberpunk_neon(),
        "frosted glass" | "frosted" => ThemePreset::frosted_glass(),
        "nordic dark" | "nordic" => ThemePreset::nordic_dark(),
        _ => ThemePreset::macos_sonoma(),
    };

    state
        .taskbar
        .set_config(preset.taskbar)
        .map_err(|e| e.to_string())?;
    save_settings(&state.taskbar.config().map_err(|e| e.to_string())?)?;

    if let Ok(a) = state.audio.lock() {
        a.set_tint(preset.accent_color);
    }

    Ok(())
}

#[tauri::command]
fn reset_defaults(state: tauri::State<AppState>) -> Result<(), String> {
    state.taskbar.reset_all().map_err(|e| e.to_string())?;
    state.taskbar.repair_icons().map_err(|e| e.to_string())?;
    save_settings(&state.taskbar.config().map_err(|e| e.to_string())?)
}
#[tauri::command]
fn repair_taskbar_icons(state: tauri::State<AppState>) -> Result<TaskbarConfig, String> {
    let mut config = state.taskbar.config().map_err(|e| e.to_string())?;
    config.appearance.icon_size = 0;
    config.appearance.hover_scale = 100;
    state.taskbar.repair_icons().map_err(|e| e.to_string())?;
    state
        .taskbar
        .set_config(config.clone())
        .map_err(|e| e.to_string())?;
    save_settings(&config)?;
    Ok(config)
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

#[derive(Clone, Serialize, Deserialize)]
pub struct TaskbarAppIcon {
    pub name: String,
    pub icon: String,
    pub is_running: bool,
    pub is_active: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TaskbarPreviewInfo {
    pub apps: Vec<TaskbarAppIcon>,
    pub clock: String,
    pub language: String,
}

fn base64_encode(data: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARSET[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARSET[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARSET[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARSET[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn hicon_to_bmp_base64(
    icon: windows::Win32::UI::WindowsAndMessaging::HICON,
    size: i32,
) -> Option<String> {
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::UI::WindowsAndMessaging::*;
    unsafe {
        let hdc = CreateCompatibleDC(None);
        if hdc.0.is_null() {
            return None;
        }

        let bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size,
                biHeight: -size,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                biSizeImage: (size * size * 4) as u32,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD::default()],
        };

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbmp = CreateDIBSection(hdc, &bi, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        let old_bmp = SelectObject(hdc, hbmp);

        std::slice::from_raw_parts_mut(bits as *mut u8, (size * size * 4) as usize).fill(0);

        let _ = DrawIconEx(hdc, 0, 0, icon, size, size, 0, None, DI_NORMAL);

        let pixel_bytes = std::slice::from_raw_parts(bits as *const u8, (size * size * 4) as usize);

        let pixels = pixel_bytes.as_chunks::<4>().0;
        let has_alpha = pixels.iter().any(|p| p[3] != 0);
        let mut rgba = Vec::with_capacity(pixel_bytes.len());
        for p in pixels {
            let alpha = if has_alpha {
                p[3]
            } else if p[..3].iter().any(|v| *v != 0) {
                255
            } else {
                0
            };
            let unpremultiply = |v: u8| {
                if alpha == 0 {
                    0
                } else {
                    ((v as u32 * 255) / (alpha as u32)).min(255) as u8
                }
            };
            rgba.extend_from_slice(&[
                unpremultiply(p[2]),
                unpremultiply(p[1]),
                unpremultiply(p[0]),
                alpha,
            ]);
        }

        SelectObject(hdc, old_bmp);
        let _ = DeleteObject(hbmp);
        let _ = DeleteDC(hdc);

        let mut output = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut output, size as u32, size as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().ok()?;
            writer.write_image_data(&rgba).ok()?;
        }
        Some(format!("data:image/png;base64,{}", base64_encode(&output)))
    }
}

#[repr(C)]
struct ShFileInfoW {
    h_icon: windows::Win32::UI::WindowsAndMessaging::HICON,
    i_icon: i32,
    dw_attributes: u32,
    sz_display_name: [u16; 260],
    sz_type_name: [u16; 80],
}

#[link(name = "shell32")]
extern "system" {
    fn SHGetFileInfoW(
        pszpath: *const u16,
        dwfileattributes: u32,
        psfi: *mut ShFileInfoW,
        cbfileinfo: u32,
        uflags: u32,
    ) -> usize;
}

#[link(name = "user32")]
extern "system" {
    fn GetKeyboardLayout(id_thread: u32) -> isize;
}

#[tauri::command]
fn get_taskbar_preview_info() -> Result<TaskbarPreviewInfo, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::*;

    struct OpenWinItem {
        title: String,
        short_name: String,
        icon: Option<String>,
        is_active: bool,
    }

    let foreground = unsafe { GetForegroundWindow() };

    unsafe extern "system" fn enum_win(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let state = unsafe { &mut *(lparam.0 as *mut (Vec<OpenWinItem>, HWND)) };
        if state.0.len() >= 32 {
            return BOOL(0);
        }
        if unsafe { IsWindowVisible(hwnd) }.as_bool() {
            let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
            let ex = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;
            if (style & WS_CHILD.0) == 0 && (ex & WS_EX_TOOLWINDOW.0) == 0 {
                let mut title = [0u16; 256];
                let len = unsafe { GetWindowTextW(hwnd, &mut title) };
                if len > 0 {
                    let text = String::from_utf16_lossy(&title[..len as usize]);
                    let trimmed = text.trim();
                    if !trimmed.is_empty() && trimmed != "Program Manager" {
                        let short = trimmed
                            .split(" - ")
                            .last()
                            .unwrap_or(trimmed)
                            .trim()
                            .to_string();
                        let is_active = hwnd == state.1;
                        let mut b64 = None;
                        let mut icon = HICON(unsafe {
                            SendMessageW(hwnd, WM_GETICON, WPARAM(2), LPARAM(0)).0
                                as *mut std::ffi::c_void
                        });
                        if icon.0.is_null() {
                            icon = HICON(unsafe {
                                SendMessageW(hwnd, WM_GETICON, WPARAM(0), LPARAM(0)).0
                                    as *mut std::ffi::c_void
                            });
                        }
                        if icon.0.is_null() {
                            icon = HICON(unsafe {
                                GetClassLongPtrW(hwnd, GCLP_HICON) as *mut std::ffi::c_void
                            });
                        }
                        if !icon.0.is_null() {
                            b64 = hicon_to_bmp_base64(icon, 32);
                        }
                        state.0.push(OpenWinItem {
                            title: text,
                            short_name: short,
                            icon: b64,
                            is_active,
                        });
                    }
                }
            }
        }
        BOOL(1)
    }

    let mut open_state: (Vec<OpenWinItem>, HWND) = (Vec::new(), foreground);
    let _ = unsafe { EnumWindows(Some(enum_win), LPARAM(&mut open_state as *mut _ as isize)) };
    let open_windows = open_state.0;

    let mut apps = Vec::new();
    let mut used_open_windows = std::collections::HashSet::new();

    let pinned_dir = std::env::var_os("APPDATA")
        .map(std::path::PathBuf::from)
        .map(|p| p.join("Microsoft\\Internet Explorer\\Quick Launch\\User Pinned\\TaskBar"));

    if let Some(dir) = pinned_dir {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("lnk") {
                    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
                    wide.push(0);

                    let mut shinfo = ShFileInfoW {
                        h_icon: HICON::default(),
                        i_icon: 0,
                        dw_attributes: 0,
                        sz_display_name: [0; 260],
                        sz_type_name: [0; 80],
                    };

                    let res = unsafe {
                        SHGetFileInfoW(
                            wide.as_ptr(),
                            0,
                            &mut shinfo,
                            std::mem::size_of::<ShFileInfoW>() as u32,
                            0x000000100,
                        )
                    };

                    if res != 0 && !shinfo.h_icon.0.is_null() {
                        if let Some(b64) = hicon_to_bmp_base64(shinfo.h_icon, 32) {
                            let name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("App")
                                .to_string();

                            let mut is_running = false;
                            let mut is_active = false;
                            for (idx, win) in open_windows.iter().enumerate() {
                                let pn = name.to_lowercase();
                                let wt = win.title.to_lowercase();
                                let ws = win.short_name.to_lowercase();
                                if wt.contains(&pn) || ws.contains(&pn) || pn.contains(&ws) {
                                    is_running = true;
                                    if win.is_active {
                                        is_active = true;
                                    }
                                    used_open_windows.insert(idx);
                                    break;
                                }
                            }

                            apps.push(TaskbarAppIcon {
                                name,
                                icon: b64,
                                is_running,
                                is_active,
                            });
                        }
                        let _ = unsafe { DestroyIcon(shinfo.h_icon) };
                    }
                }
            }
        }
    }

    let mut seen: std::collections::HashSet<String> = apps
        .iter()
        .map(|a: &TaskbarAppIcon| a.name.to_lowercase())
        .collect();

    for (idx, win) in open_windows.iter().enumerate() {
        if !used_open_windows.contains(&idx) {
            if let Some(b64) = &win.icon {
                let key = win.short_name.to_lowercase();
                if !seen.contains(&key) && apps.len() < 24 {
                    seen.insert(key);
                    apps.push(TaskbarAppIcon {
                        name: win.short_name.clone(),
                        icon: b64.clone(),
                        is_running: true,
                        is_active: win.is_active,
                    });
                }
            }
        }
    }

    let st = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    let clock = format!("{:02}:{:02}", st.wHour, st.wMinute);

    let layout = unsafe { GetKeyboardLayout(0) };
    let lang_id = (layout as usize & 0xFFFF) as u16;
    let language = match lang_id {
        0x0422 => "УКР",
        0x0409 => "ENG",
        0x0419 => "РУС",
        0x0407 => "DEU",
        0x040C => "FRA",
        0x0415 => "POL",
        _ => "УКР",
    }
    .to_string();

    Ok(TaskbarPreviewInfo {
        apps,
        clock,
        language,
    })
}

pub fn run() {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn SetConsoleCtrlHandler(
                handler_routine: Option<unsafe extern "system" fn(u32) -> i32>,
                add: i32,
            ) -> i32;
        }
        unsafe extern "system" fn ctrl_handler(_: u32) -> i32 {
            if dock_test_mode() {
                return 0;
            }
            let taskbars = atelier_taskbar::discovery::find_all_taskbars();
            for &hwnd in &taskbars {
                let _ = atelier_taskbar::style::apply_taskbar_style(
                    hwnd,
                    atelier_common::config::TaskbarStyle::Default,
                    false,
                );
                let _ = atelier_taskbar::dock::reset_floating_dock(hwnd, false);
            }
            0
        }
        unsafe {
            let _ = SetConsoleCtrlHandler(Some(ctrl_handler), 1);
        }
    }

    let config = AtelierConfig {
        taskbar: TaskbarConfig::default(),
        ..Default::default()
    };

    let taskbar = Arc::new(if dock_test_mode() {
        TaskbarManager::passive(config.taskbar)
    } else {
        TaskbarManager::new(config.taskbar)
    });
    if !dock_test_mode() {
        let _ = taskbar.start_recovery();
    }

    let wallpaper = if dock_test_mode() {
        None
    } else {
        WallpaperEngine::new(true).ok().map(Arc::new)
    };

    let audio = Arc::new(Mutex::new(AudioManager::new(0x00FF8800)));
    if !dock_test_mode() {
        let _ = audio.lock().map(|mut a| a.start());
    }

    let widget = Arc::new(WidgetSubsystem::new());

    let state = AppState {
        taskbar,
        wallpaper,
        audio,
        widget,
    };

    let _ = tauri::Builder::default()
        .manage(state)
        .manage(desktop_dock::DockState::new())
        .setup(|app| {
            let icon = Image::new(include_bytes!("../icons/icon.rgba"), 128, 128);
            if let Some(window) = app.get_webview_window("main") {
                window.set_icon(icon.clone())?;
                window.set_title("Atelier Desktop Studio")?;
                let _ = window.set_theme(Some(tauri::Theme::Dark));
                #[cfg(target_os = "windows")]
                {
                    use windows::Win32::Foundation::HWND;
                    use windows::Win32::Graphics::Dwm::{
                        DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE,
                    };
                    if let Ok(raw_hwnd) = window.hwnd() {
                        let dark: i32 = 1;
                        let _ = unsafe {
                            DwmSetWindowAttribute(
                                HWND(raw_hwnd.0 as _),
                                DWMWA_USE_IMMERSIVE_DARK_MODE,
                                &dark as *const _ as _,
                                std::mem::size_of::<i32>() as u32,
                            )
                        };
                    }
                }
            }
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
                        let dock_app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = desktop_dock::reset_desktop_dock(dock_app).await;
                        });
                        let state = app.state::<AppState>();
                        let _ = reset_defaults(state);
                    }
                    "quit" => {
                        app.state::<desktop_dock::DockState>().shutdown();
                        let state = app.state::<AppState>();
                        let _ = state.taskbar.reset_all();
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

            builder = builder.icon(icon);

            let _tray = builder.build(app)?;
            if let Err(error) = desktop_dock::start(app.handle()) {
                tracing::error!(%error, "Could not start desktop dock");
            }
            #[cfg(debug_assertions)]
            if dock_test_mode() && std::env::var_os("ATELIER_DOCK_NATIVE_TEST").is_some() {
                let app = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.eval(include_str!("../tests/dock-native.js"));
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_taskbar_config,
            get_taskbar_capabilities,
            set_taskbar_config,
            apply_style,
            set_floating_dock,
            apply_preset,
            reset_defaults,
            repair_taskbar_icons,
            hide_window,
            get_system_status,
            get_taskbar_preview_info,
            desktop_dock::get_dock_snapshot,
            desktop_dock::set_dock_config,
            desktop_dock::reset_desktop_dock,
            desktop_dock::dock_resize,
            desktop_dock::dock_activate,
            desktop_dock::dock_pin,
            desktop_dock::dock_reorder,
            desktop_dock::dock_set_icon,
            desktop_dock::dock_open_studio,
            desktop_dock::dock_native_test_probe,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if window.label() != "main" {
                    return;
                }
                window.state::<desktop_dock::DockState>().shutdown();
                let state = window.state::<AppState>();
                if !dock_test_mode() {
                    let _ = state.taskbar.reset_all();
                }
                window.app_handle().exit(0);
            }
        })
        .run(tauri::generate_context!());
}
