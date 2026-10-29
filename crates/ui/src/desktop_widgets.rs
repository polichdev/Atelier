use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

const RESIZE_MARGIN: u32 = 18;
const KINDS: [&str; 6] = ["clock", "calendar", "system", "media", "battery", "notes"];
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WidgetConfig {
    pub kind: String,
    pub enabled: bool,
    pub size: String,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: u32,
    pub height: u32,
    pub radius: u32,
    pub background: String,
    pub foreground: String,
    pub opacity: u32,
    pub show_date: bool,
    pub show_seconds: bool,
    pub hour_24: bool,
    pub show_month: bool,
    pub show_weekday: bool,
    pub show_cpu: bool,
    pub show_memory: bool,
    pub show_gpu: bool,
    pub show_details: bool,
    pub show_artist: bool,
    pub show_album: bool,
    pub show_status: bool,
    pub notes: String,
    pub city: String,
    pub fahrenheit: bool,
    pub photo: String,
    pub countdown_title: String,
    pub countdown_date: String,
    pub show_network: bool,
    pub show_disk: bool,
    pub show_artwork: bool,
    pub show_forecast: bool,
}
impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            kind: String::new(),
            enabled: false,
            size: "small".into(),
            x: None,
            y: None,
            width: 0,
            height: 156,
            radius: 22,
            background: String::new(),
            foreground: String::new(),
            opacity: 94,
            show_date: true,
            show_seconds: false,
            hour_24: true,
            show_month: true,
            show_weekday: true,
            show_cpu: true,
            show_memory: true,
            show_gpu: false,
            show_details: true,
            show_artist: true,
            show_album: false,
            show_status: true,
            notes: String::new(),
            city: String::new(),
            fahrenheit: false,
            photo: String::new(),
            countdown_title: String::new(),
            countdown_date: String::new(),
            show_network: false,
            show_disk: false,
            show_artwork: true,
            show_forecast: true,
        }
    }
}
fn normalize(c: &mut WidgetConfig) -> Result<(), String> {
    if !KINDS.contains(&c.kind.as_str()) {
        return Err("Unknown widget".into());
    }
    if c.width == 0 {
        c.width = if c.size == "medium" { 328 } else { 156 };
    }
    c.width = c.width.clamp(140, 1200);
    c.height = c.height.clamp(120, 900);
    c.radius = c.radius.min(80);
    c.opacity = c.opacity.clamp(20, 100);
    if c.background.is_empty() {
        c.background = match c.kind.as_str() {
            "clock" => "#20212b",
            "media" => "#7651bb",
            "weather" => "#447cb7",
            "notes" => "#fff1a6",
            _ => "#f8f9fc",
        }
        .into();
    }
    if c.foreground.is_empty() {
        c.foreground = if ["clock", "media", "weather"].contains(&c.kind.as_str()) {
            "#ffffff"
        } else {
            "#202332"
        }
        .into();
    }
    for color in [&c.background, &c.foreground] {
        if color.len() != 7
            || !color.starts_with('#')
            || !color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
        {
            return Err("Invalid color".into());
        }
    }
    if c.notes.chars().count() > 2000 {
        return Err("Note is too long".into());
    }
    if c.city.len() > 120
        || c.countdown_title.len() > 120
        || c.countdown_date.len() > 10
        || c.photo.len() > 7_000_000
        || (!c.photo.is_empty()
            && !(c.photo.starts_with("data:image/jpeg;base64,")
                || c.photo.starts_with("data:image/gif;base64,")
                || c.photo.strip_prefix("stored:").is_some_and(|id| {
                    !id.is_empty() && id.len() <= 32 && id.bytes().all(|b| b.is_ascii_digit())
                })))
    {
        return Err("Invalid widget content".into());
    }
    Ok(())
}
pub struct Widgets(pub Mutex<Vec<WidgetConfig>>);
fn path() -> std::path::PathBuf {
    super::settings_path()
        .unwrap_or_else(|| std::env::temp_dir().join("Atelier/settings.json"))
        .with_file_name("widgets.json")
}
impl Widgets {
    pub fn new() -> Self {
        let saved: Vec<WidgetConfig> = std::fs::read(path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self(Mutex::new(
            KINDS
                .iter()
                .map(|kind| {
                    let mut c = saved
                        .iter()
                        .find(|c| c.kind == *kind)
                        .cloned()
                        .unwrap_or_else(|| WidgetConfig {
                            kind: kind.to_string(),
                            ..Default::default()
                        });
                    if normalize(&mut c).is_err() {
                        c = WidgetConfig {
                            kind: kind.to_string(),
                            ..Default::default()
                        };
                        let _ = normalize(&mut c);
                    }
                    c
                })
                .collect(),
        ))
    }
}
fn persist(config: &[WidgetConfig]) -> Result<(), String> {
    let p = path();
    std::fs::create_dir_all(p.parent().ok_or("Invalid settings path")?)
        .map_err(|e| e.to_string())?;
    std::fs::write(
        p,
        serde_json::to_vec_pretty(config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
fn apply(app: &tauri::AppHandle, c: &WidgetConfig, index: usize) -> Result<(), String> {
    let label = format!("widget-{}", c.kind);
    if !c.enabled {
        if let Some(w) = app.get_webview_window(&label) {
            w.hide().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let w = if let Some(w) = app.get_webview_window(&label) {
        w
    } else {
        let monitor = app.primary_monitor().map_err(|e| e.to_string())?;
        let (x, y) = if let Some(m) = monitor {
            let a = m.work_area();
            let scale = m.scale_factor();
            let width = (c.width as f64 * scale).round() as i32;
            let height = (c.height as f64 * scale).round() as i32;
            let step_y = ((170.0 * scale).round() as i32).max(height + 12);
            let rows = ((a.size.height as i32 - 30).max(step_y) / step_y).max(1);
            let column = index as i32 / rows;
            let row = index as i32 % rows;
            let right = a.position.x + a.size.width as i32;
            let x = (right - width - 30 - column * (width + 16)).max(a.position.x);
            let y = (a.position.y + 30 + row * step_y)
                .min(a.position.y + (a.size.height as i32 - height).max(0));
            (x, y)
        } else {
            (50, 50 + index as i32 * 170)
        };
        let w = tauri::WebviewWindowBuilder::new(
            app,
            &label,
            tauri::WebviewUrl::App(format!("widget.html?kind={}", c.kind).into()),
        )
        .title(format!("Atelier {}", c.kind))
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .inner_size(
            (c.width + 2 * RESIZE_MARGIN) as f64,
            (c.height + 2 * RESIZE_MARGIN) as f64,
        )
        .build()
        .map_err(|e| e.to_string())?;
        let margin =
            (RESIZE_MARGIN as f64 * w.scale_factor().map_err(|e| e.to_string())?).round() as i32;
        w.set_position(tauri::PhysicalPosition::new(
            c.x.unwrap_or(x) - margin,
            c.y.unwrap_or(y) - margin,
        ))
        .map_err(|e| e.to_string())?;
        w
    };
    w.set_size(tauri::LogicalSize::new(
        c.width + 2 * RESIZE_MARGIN,
        c.height + 2 * RESIZE_MARGIN,
    ))
    .map_err(|e| e.to_string())?;
    w.show().map_err(|e| e.to_string())?;
    let _ = app.emit_to(&label, "widget-config", c);
    Ok(())
}
pub fn start(app: &tauri::AppHandle) -> Result<(), String> {
    let configs = app
        .state::<Widgets>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    for (i, c) in configs.iter().enumerate() {
        apply(app, c, i)?;
    }
    Ok(())
}
#[tauri::command]
pub fn get_widgets(state: tauri::State<Widgets>) -> Result<Vec<WidgetConfig>, String> {
    Ok(state.0.lock().map_err(|e| e.to_string())?.clone())
}
#[tauri::command]
pub async fn set_widget(
    app: tauri::AppHandle,
    state: tauri::State<'_, Widgets>,
    mut config: WidgetConfig,
) -> Result<Vec<WidgetConfig>, String> {
    normalize(&mut config)?;
    let mut configs = state.0.lock().map_err(|e| e.to_string())?;
    let i = configs
        .iter()
        .position(|c| c.kind == config.kind)
        .ok_or("Unknown widget")?;
    config.x = configs[i].x;
    config.y = configs[i].y;
    let mut next = configs.clone();
    next[i] = config;
    persist(&next)?;
    *configs = next.clone();
    drop(configs);
    apply(&app, &next[i], i)?;
    Ok(next)
}
pub fn moved(window: &tauri::Window, p: &tauri::PhysicalPosition<i32>) {
    let Some(kind) = window.label().strip_prefix("widget-") else {
        return;
    };
    let state = window.state::<Widgets>();
    if let Ok(mut configs) = state.0.lock() {
        if let Some(c) = configs.iter_mut().find(|c| c.kind == kind) {
            let margin =
                (RESIZE_MARGIN as f64 * window.scale_factor().unwrap_or(1.0)).round() as i32;
            c.x = Some(p.x + margin);
            c.y = Some(p.y + margin);
            let _ = persist(&configs);
        }
    };
}
#[tauri::command]
pub fn widget_open_settings(app: tauri::AppHandle, kind: String) -> Result<(), String> {
    if !KINDS.contains(&kind.as_str()) {
        return Err("Unknown widget".into());
    }
    let w = app.get_webview_window("main").ok_or("Studio unavailable")?;
    w.unminimize().map_err(|e| e.to_string())?;
    w.show().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())?;
    app.emit_to("main", "widget-open-settings", kind)
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn widget_stats(
    state: tauri::State<'_, super::AppState>,
) -> Result<serde_json::Value, String> {
    let widget = state.widget.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let s=widget.poll_hardware_stats();
        let (frequency,network,network_percent,disk,gpu_temp)=super::widget_metrics::poll();
        serde_json::json!({"cpu":s.cpu_percent,"memory":s.mem_percent,"used":s.mem_used_mb,"total":s.mem_total_mb,"gpu":super::widget_gpu::poll(),"frequency":frequency,"network":network,"network_percent":network_percent,"disk":disk,"gpu_temp":gpu_temp})
    }).await.map_err(|e|e.to_string())
}
#[repr(C)]
struct PowerStatus {
    ac: u8,
    flags: u8,
    percent: u8,
    reserved: u8,
    life: u32,
    full: u32,
}
#[link(name = "kernel32")]
extern "system" {
    fn GetSystemPowerStatus(status: *mut PowerStatus) -> i32;
}
#[tauri::command]
pub fn widget_battery() -> serde_json::Value {
    let mut s = PowerStatus {
        ac: 255,
        flags: 255,
        percent: 255,
        reserved: 0,
        life: 0,
        full: 0,
    };
    let ok = unsafe { GetSystemPowerStatus(&mut s) } != 0;
    serde_json::json!({"available":ok && s.flags != 255 && s.flags & 128 == 0 && s.percent <= 100,"percent":s.percent,"charging":s.ac==1})
}
#[tauri::command]
pub async fn widget_media(
    state: tauri::State<'_, super::AppState>,
) -> Result<serde_json::Value, String> {
    let widget = state.widget.clone();
    tauri::async_runtime::spawn_blocking(move||{let m=widget.poll_media().map_err(|e|e.to_string())?;Ok(serde_json::json!({"title":m.title,"artist":m.artist,"album":m.album,"playing":m.is_playing,"can_play":m.can_play,"can_pause":m.can_pause,"can_next":m.can_next,"can_previous":m.can_previous,"can_seek":m.can_seek,"position":m.position,"duration":m.duration,"artwork":m.artwork.map(|data| format!("data:{};base64,{}", if data.starts_with(&[0xff,0xd8,0xff]) {"image/jpeg"} else if data.starts_with(b"GIF8") {"image/gif"} else if data.starts_with(b"RIFF") {"image/webp"} else {"image/png"},super::base64_encode(&data)))}))}).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn widget_media_control(action: String) -> Result<(), String> {
    if !["play", "pause", "previous", "next"].contains(&action.as_str()) {
        return Err("Unknown media action".into());
    }
    let applied = tauri::async_runtime::spawn_blocking(move || {
        atelier_widget::media::SystemMediaMonitor::control(&action)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    if applied {
        Ok(())
    } else {
        Err("The current player does not support this action".into())
    }
}

#[tauri::command]
pub async fn widget_media_seek(seconds: f64) -> Result<(), String> {
    if !seconds.is_finite() || !(0.0..=604800.0).contains(&seconds) {
        return Err("Invalid playback position".into());
    }
    let ok = tauri::async_runtime::spawn_blocking(move || {
        atelier_widget::media::SystemMediaMonitor::seek((seconds * 10_000_000.0) as i64)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    if ok {
        Ok(())
    } else {
        Err("The current player does not support seeking".into())
    }
}

#[tauri::command]
pub fn widget_position(window: tauri::Window) -> Result<tauri::PhysicalPosition<i32>, String> {
    if !window.label().starts_with("widget-") {
        return Err("Not a widget".into());
    }
    window.outer_position().map_err(|e| e.to_string())
}
#[tauri::command]
pub fn widget_move(window: tauri::Window, x: i32, y: i32) -> Result<(), String> {
    if !window.label().starts_with("widget-") {
        return Err("Not a widget".into());
    }
    window
        .set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn widget_resize_focus(window: tauri::Window) -> Result<(), String> {
    if !window.label().starts_with("widget-") {
        return Err("Not a widget".into());
    }
    window.set_focus().map_err(|e| e.to_string())
}
pub fn dismiss_resize(window: &tauri::Window) {
    if let Some(kind) = window.label().strip_prefix("widget-") {
        let _ = window
            .app_handle()
            .emit_to(window.label(), "widget-dismiss-resize", kind);
    }
}
#[derive(Serialize)]
pub struct WidgetBounds {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale: f64,
}
fn content_bounds(x: i32, y: i32, width: u32, height: u32, scale: f64) -> WidgetBounds {
    let margin = (RESIZE_MARGIN as f64 * scale).round() as i32;
    let total_margin = (2.0 * RESIZE_MARGIN as f64 * scale).round() as u32;
    WidgetBounds {
        x: x + margin,
        y: y + margin,
        width: width.saturating_sub(total_margin),
        height: height.saturating_sub(total_margin),
        scale,
    }
}
#[tauri::command]
pub fn widget_bounds(window: tauri::Window) -> Result<WidgetBounds, String> {
    if !window.label().starts_with("widget-") {
        return Err("Not a widget".into());
    }
    let p = window.outer_position().map_err(|e| e.to_string())?;
    let s = window.inner_size().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    Ok(content_bounds(p.x, p.y, s.width, s.height, scale))
}
#[tauri::command]
pub fn widget_resize(
    window: tauri::Window,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    commit: bool,
) -> Result<(), String> {
    let kind = window
        .label()
        .strip_prefix("widget-")
        .ok_or("Not a widget")?
        .to_string();
    if !KINDS.contains(&kind.as_str()) {
        return Err("Unknown widget".into());
    }
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let logical_width = ((width as f64 / scale).round() as u32).clamp(140, 1200);
    let logical_height = ((height as f64 / scale).round() as u32).clamp(120, 900);
    window
        .set_size(tauri::LogicalSize::new(
            logical_width + 2 * RESIZE_MARGIN,
            logical_height + 2 * RESIZE_MARGIN,
        ))
        .map_err(|e| e.to_string())?;
    window
        .set_position(tauri::PhysicalPosition::new(
            x - (RESIZE_MARGIN as f64 * scale).round() as i32,
            y - (RESIZE_MARGIN as f64 * scale).round() as i32,
        ))
        .map_err(|e| e.to_string())?;
    if commit {
        let state = window.state::<Widgets>();
        let updated = {
            let mut configs = state.0.lock().map_err(|e| e.to_string())?;
            let c = configs
                .iter_mut()
                .find(|c| c.kind == kind)
                .ok_or("Unknown widget")?;
            c.width = logical_width;
            c.height = logical_height;
            c.x = Some(x);
            c.y = Some(y);
            let c = c.clone();
            persist(&configs)?;
            c
        };
        let _ = window
            .app_handle()
            .emit_to("main", "widget-geometry", updated);
    }
    Ok(())
}

