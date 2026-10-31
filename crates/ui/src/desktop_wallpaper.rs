use base64::Engine;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager};
use windows::core::w;
use windows::Win32::{
    Foundation::{COLORREF, HWND, POINT},
    Graphics::Gdi::ScreenToClient,
    UI::WindowsAndMessaging::{
        FindWindowExW, GetAncestor, GetClassNameW, GetDesktopWindow, GetSystemMetrics, GetWindow,
        GetWindowLongPtrW, GetWindowRect, IsWindow, SetLayeredWindowAttributes, SetParent,
        SetWindowLongPtrW, SetWindowPos, ShowWindow, GA_PARENT, GWL_EXSTYLE, GWL_STYLE, GW_HWNDPREV,
        HWND_TOP, LWA_ALPHA, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
        SWP_SHOWWINDOW, SW_SHOW, WS_BORDER, WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN,
        WS_CLIPSIBLINGS, WS_DLGFRAME, WS_EX_CLIENTEDGE, WS_EX_DLGMODALFRAME, WS_EX_LAYERED,
        WS_EX_NOACTIVATE, WS_EX_STATICEDGE, WS_EX_TOOLWINDOW,
        WS_EX_WINDOWEDGE, WS_HSCROLL, WS_POPUP, WS_THICKFRAME, WS_VISIBLE, WS_VSCROLL,
    },
};
const MAX_VIDEO: u64 = 1024 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct WallpaperItem {
    pub id: String,
    pub title: String,
    pub category: String,
    pub kind: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset: Option<String>,
}
fn valid_preset_asset(name: &str) -> bool {
    let Some((stem, ext)) = name.rsplit_once('.') else {
        return false;
    };
    !stem.is_empty()
        && stem.len() <= 80
        && stem
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
        && ["mp4", "webm"].contains(&ext)
}
fn presets() -> Vec<WallpaperItem> {
    serde_json::from_str::<Vec<WallpaperItem>>(include_str!("../ui/wallpaper-catalog.json"))
        .unwrap_or_default()
        .into_iter()
        .filter(|p| {
            (p.kind == "preset"
                || (p.kind == "preset-video" && p.asset.as_deref().is_some_and(valid_preset_asset)))
                && p.id.starts_with("preset-")
                && p.id.len() <= 48
                && p.id
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
        .collect()
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct AudioEffects {
    pub pulse: bool,
    pub glow: bool,
    pub pulse_strength: u32,
    pub glow_strength: u32,
}
impl Default for AudioEffects {
    fn default() -> Self {
        Self {
            pulse: false,
            glow: false,
            pulse_strength: 40,
            glow_strength: 40,
        }
    }
}
impl AudioEffects {
    fn normalize(&mut self) {
        self.pulse_strength = self.pulse_strength.min(100);
        self.glow_strength = self.glow_strength.min(100);
    }
    fn enabled(&self) -> bool {
        self.pulse || self.glow
    }
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct WallpaperOptions {
    pub fit: String,
    pub paused: bool,
    pub mute: bool,
    pub volume: u32,
    pub pause_fullscreen: bool,
    pub pause_battery: bool,
    pub scene: super::scene_properties::SceneOptions,
    pub audio: AudioEffects,
}
impl Default for WallpaperOptions {
    fn default() -> Self {
        Self {
            fit: "cover".into(),
            paused: false,
            mute: true,
            volume: 50,
            pause_fullscreen: true,
            pause_battery: false,
            scene: Default::default(),
            audio: Default::default(),
        }
    }
}
impl WallpaperOptions {
    fn normalize(&mut self) {
        if !["cover", "contain", "stretch"].contains(&self.fit.as_str()) {
            self.fit = "cover".into()
        }
        self.volume = self.volume.min(100);
        self.scene.normalize();
        self.audio.normalize();
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WallpaperConfig {
    pub items: Vec<WallpaperItem>,
    pub active: Option<String>,
    pub settings: HashMap<String, WallpaperOptions>,
    pub performance: super::wallpaper_performance::PerformanceSettings,
    pub fit: String,
    pub paused: bool,
    pub mute: bool,
    pub volume: u32,
    pub pause_fullscreen: bool,
    pub pause_battery: bool,
}
impl Default for WallpaperConfig {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            active: None,
            settings: HashMap::new(),
            performance: Default::default(),
            fit: "cover".into(),
            paused: false,
            mute: true,
            volume: 50,
            pause_fullscreen: true,
            pause_battery: false,
        }
    }
}
impl WallpaperConfig {
    fn legacy_options(&self) -> WallpaperOptions {
        WallpaperOptions {
            fit: self.fit.clone(),
            paused: self.paused,
            mute: self.mute,
            volume: self.volume,
            pause_fullscreen: self.pause_fullscreen,
            pause_battery: self.pause_battery,
            scene: Default::default(),
            audio: Default::default(),
        }
    }
    fn options_for(&self, id: &str) -> WallpaperOptions {
        self.settings.get(id).cloned().unwrap_or_default()
    }
    fn migrate_settings(&mut self) {
        self.performance.normalize();
        if self.settings.is_empty() {
            let mut legacy = self.legacy_options();
            legacy.normalize();
            for item in &self.items {
                self.settings.insert(item.id.clone(), legacy.clone());
            }
        }
        self.settings
            .retain(|id, _| self.items.iter().any(|i| &i.id == id));
        for item in &self.items {
            self.settings
                .entry(item.id.clone())
                .or_default()
                .normalize();
        }
    }
}
struct Upload {
    id: String,
    title: String,
    ext: String,
    path: PathBuf,
    file: File,
    size: u64,
    written: u64,
}
impl Drop for Upload {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
pub struct Wallpapers {
    config: Mutex<WallpaperConfig>,
    upload: Mutex<Option<Upload>>,
    playback: Mutex<String>,
    playback_error: Mutex<Option<String>>,
    recovering: std::sync::atomic::AtomicBool,
}
fn root() -> PathBuf {
    super::settings_path()
        .unwrap_or_else(|| std::env::temp_dir().join("Atelier/settings.json"))
        .with_file_name("wallpapers")
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 32 && id.bytes().all(|x| x.is_ascii_digit())
}
fn save(c: &WallpaperConfig) -> Result<(), String> {
    let root = root();
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let tmp = root.join("library.json.tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(c).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let dest = root.join("library.json");
    let src: Vec<u16> = tmp.as_os_str().encode_wide().chain(Some(0)).collect();
    let dst: Vec<u16> = dest.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        MoveFileExW(
            windows::core::PCWSTR(src.as_ptr()),
            windows::core::PCWSTR(dst.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|e| e.to_string())
}
impl Wallpapers {
    pub fn audio_effects_enabled(&self) -> bool {
        self.config
            .lock()
            .ok()
            .and_then(|c| {
                c.active
                    .as_ref()
                    .map(|id| c.options_for(id).audio.enabled())
            })
            .unwrap_or(false)
    }
    pub fn is_active(&self) -> bool {
        self.config
            .lock()
            .map(|c| c.active.is_some())
            .unwrap_or(false)
    }
    pub fn new() -> Self {
        let mut c: WallpaperConfig = std::fs::read(root().join("library.json"))
            .ok()
            .and_then(|d| serde_json::from_slice(&d).ok())
            .unwrap_or_default();
        c.items.retain(|i| {
            (i.kind == "video" && valid_id(&i.id) && ["mp4", "webm"].contains(&i.category.as_str()))
                || (i.kind == "scene" && valid_id(&i.id))
                || ((i.kind == "preset" || i.kind == "preset-video")
                    && presets().iter().any(|p| p.id == i.id && p.kind == i.kind))
        });
        for item in &mut c.items {
            if let Some(preset) = presets()
                .into_iter()
                .find(|p| p.id == item.id && p.kind == item.kind)
            {
                *item = preset;
            }
        }
        if !["cover", "contain", "stretch"].contains(&c.fit.as_str()) {
            c.fit = "cover".into()
        }
        c.volume = c.volume.min(100);
        if c.active
            .as_ref()
            .is_some_and(|id| !c.items.iter().any(|i| &i.id == id))
        {
            c.active = None
        }
        c.migrate_settings();
        Self {
            config: Mutex::new(c),
            upload: Mutex::new(None),
            playback: Mutex::new("stopped".into()),
            playback_error: Mutex::new(None),
            recovering: std::sync::atomic::AtomicBool::new(false),
        }
    }
}
#[derive(Serialize)]
pub struct Snapshot {
    pub config: WallpaperConfig,
    pub catalog: Vec<WallpaperItem>,
    pub playback: String,
    pub error: Option<String>,
}
pub fn import_workshop_scene(
    app: &tauri::AppHandle,
    source: &Path,
    title: &str,
    description: &str,
) -> Result<WallpaperItem, String> {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos()
        .to_string();
    std::fs::create_dir_all(root()).map_err(|e| e.to_string())?;
    let directory = root().join(format!("{id}.scene"));
    let preview = super::wallpaper_scene::prepare(source, &directory)?;
    let item = WallpaperItem {
        id: id.clone(),
        title: title.into(),
        description: description.into(),
        category: "scene".into(),
        kind: "scene".into(),
        asset: Some(preview),
    };
    let state = app.state::<Wallpapers>();
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    let mut next = config.clone();
    next.items.push(item.clone());
    next.settings.insert(id, WallpaperOptions::default());
    if let Err(error) = save(&next) {
        let _ = std::fs::remove_dir_all(directory);
        return Err(error);
    }
    *config = next;
    Ok(item)
}
#[tauri::command]
pub fn wallpaper_snapshot(state: tauri::State<Wallpapers>) -> Result<Snapshot, String> {
    Ok(Snapshot {
        config: state.config.lock().map_err(|e| e.to_string())?.clone(),
        catalog: presets(),
        playback: state.playback.lock().map_err(|e| e.to_string())?.clone(),
        error: state
            .playback_error
            .lock()
            .map_err(|e| e.to_string())?
            .clone(),
    })
}
fn ensure_studio(window: &tauri::Window) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("Only the studio can change the wallpaper library".into())
    }
}
pub fn import_workshop_video(
    app: &tauri::AppHandle,
    source: &Path,
    workshop_id: &str,
    title: Option<&str>,
    description: Option<&str>,
) -> Result<WallpaperItem, String> {
    let ext = source
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !["mp4", "webm"].contains(&ext.as_str()) {
        return Err("Unsupported video format".into());
    }
    let mut input = File::open(source).map_err(|e| e.to_string())?;
    let size = input.metadata().map_err(|e| e.to_string())?.len();
    if !(12..=MAX_VIDEO).contains(&size) {
        return Err("Choose a video up to 1 GB".into());
    }
    let mut head = [0u8; 12];
    input.read_exact(&mut head).map_err(|e| e.to_string())?;
    if (ext == "mp4" && &head[4..8] != b"ftyp")
        || (ext == "webm" && head[..4] != [0x1a, 0x45, 0xdf, 0xa3])
    {
        return Err("This file does not match the selected video format".into());
    }
    input.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos()
        .to_string();
    let dir = root();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let target = dir.join(format!("{id}.{ext}"));
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .map_err(|e| e.to_string())?;
    let copy_result = std::io::copy(&mut input.take(MAX_VIDEO + 1), &mut output)
        .map_err(|e| e.to_string())
        .and_then(|written| {
            if written == size {
                Ok(())
            } else {
                Err("Video changed while it was being imported".into())
            }
        });
    drop(output);
    if let Err(error) = copy_result {
        let _ = std::fs::remove_file(&target);
        return Err(error);
    }
    let clean_title = title
        .filter(|t| !t.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("Steam Workshop {workshop_id}"));
    let clean_desc = description
        .filter(|d| !d.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "Imported Workshop video".into());
    let item = WallpaperItem {
        id,
        title: clean_title,
        category: ext,
        kind: "video".into(),
        description: clean_desc,
        asset: None,
    };
    let state = app.state::<Wallpapers>();
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    let mut next = config.clone();
    next.items.push(item.clone());
    next.settings
        .insert(item.id.clone(), WallpaperOptions::default());
    if let Err(error) = save(&next) {
        let _ = std::fs::remove_file(target);
        return Err(error);
    }
    *config = next;
    Ok(item)
}
#[tauri::command]
pub async fn wallpaper_import_begin(
    window: tauri::Window,
    state: tauri::State<'_, Wallpapers>,
    name: String,
    size: u64,
) -> Result<String, String> {
    ensure_studio(&window)?;
    if size < 12 || size > MAX_VIDEO {
        return Err("Choose a video up to 1 GB".into());
    }
    let filename = Path::new(&name)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Invalid file name")?;
    let ext = Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !["mp4", "webm"].contains(&ext.as_str()) {
        return Err("Choose an MP4 or WebM video".into());
    }
    let title = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Wallpaper")
        .chars()
        .take(100)
        .collect();
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos()
        .to_string();
    let dir = root();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{id}.part"));
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    *state.upload.lock().map_err(|e| e.to_string())? = Some(Upload {
        id: id.clone(),
        title,
        ext,
        path,
        file,
        size,
        written: 0,
    });
    Ok(id)
}
#[tauri::command]
pub async fn wallpaper_import_chunk(
    window: tauri::Window,
    state: tauri::State<'_, Wallpapers>,
    id: String,
    offset: u64,
    data: String,
) -> Result<(), String> {
    ensure_studio(&window)?;
    if data.len() > 400_000 {
        return Err("Chunk too large".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|_| "Invalid chunk")?;
    let mut guard = state.upload.lock().map_err(|e| e.to_string())?;
    let u = guard
        .as_mut()
        .filter(|u| u.id == id)
        .ok_or("Upload unavailable")?;
    if bytes.is_empty() || offset != u.written || u.written + bytes.len() as u64 > u.size {
        return Err("Invalid upload position".into());
    }
    u.file.write_all(&bytes).map_err(|e| e.to_string())?;
    u.written += bytes.len() as u64;
    Ok(())
}
#[tauri::command]
pub async fn wallpaper_import_finish(
    window: tauri::Window,
    state: tauri::State<'_, Wallpapers>,
    id: String,
) -> Result<WallpaperItem, String> {
    ensure_studio(&window)?;
    let mut guard = state.upload.lock().map_err(|e| e.to_string())?;
    let u = guard
        .as_ref()
        .filter(|u| u.id == id)
        .ok_or("Upload unavailable")?;
    if u.size != u.written {
        return Err("Upload incomplete".into());
    }
    u.file.sync_all().map_err(|e| e.to_string())?;
    let mut head = [0u8; 12];
    File::open(&u.path)
        .and_then(|mut f| f.read_exact(&mut head))
        .map_err(|e| e.to_string())?;
    if (u.ext == "mp4" && &head[4..8] != b"ftyp")
        || (u.ext == "webm" && head[..4] != [0x1a, 0x45, 0xdf, 0xa3])
    {
        return Err("This file does not match the selected video format".into());
    }
    let target = root().join(format!("{}.{}", u.id, u.ext));
    std::fs::copy(&u.path, &target).map_err(|e| e.to_string())?;
    let item = WallpaperItem {
        id: u.id.clone(),
        title: u.title.clone(),
        category: u.ext.clone(),
        kind: "video".into(),
        description: "Imported video".into(),
        asset: None,
    };
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    let mut next = config.clone();
    next.items.push(item.clone());
    next.settings
        .insert(item.id.clone(), WallpaperOptions::default());
    if let Err(e) = save(&next) {
        let _ = std::fs::remove_file(target);
        return Err(e);
    }
    *config = next;
    *guard = None;
    Ok(item)
}
#[tauri::command]
pub fn wallpaper_add_preset(
    window: tauri::Window,
    state: tauri::State<Wallpapers>,
    id: String,
) -> Result<WallpaperConfig, String> {
    ensure_studio(&window)?;
    let p = presets()
        .into_iter()
        .find(|p| p.id == id)
        .ok_or("Unknown preset")?;
    let mut c = state.config.lock().map_err(|e| e.to_string())?;
    let mut next = c.clone();
    if !next.items.iter().any(|i| i.id == id) {
        next.items.push(p);
        next.settings.insert(id, WallpaperOptions::default());
    }
    save(&next)?;
    *c = next.clone();
    Ok(next)
}
fn attach(w: &tauri::WebviewWindow, worker: HWND) -> Result<(), String> {
    let raw = w.hwnd().map_err(|e| e.to_string())?;
    let hwnd = HWND(raw.0 as _);
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let new_style = (style
            & !((WS_POPUP.0
                | WS_CAPTION.0
                | WS_THICKFRAME.0
                | WS_BORDER.0
                | WS_DLGFRAME.0
                | WS_VSCROLL.0
                | WS_HSCROLL.0) as isize))
            | (WS_CHILD.0 | WS_VISIBLE.0 | WS_CLIPSIBLINGS.0 | WS_CLIPCHILDREN.0) as isize;
        SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);

        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let new_ex_style = (ex_style
            & !((WS_EX_DLGMODALFRAME.0
                | WS_EX_CLIENTEDGE.0
                | WS_EX_STATICEDGE.0
                | WS_EX_WINDOWEDGE.0) as isize))
            | (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex_style);
        use windows::Win32::UI::WindowsAndMessaging::WS_EX_NOREDIRECTIONBITMAP;
        if GetWindowLongPtrW(worker, GWL_EXSTYLE) & WS_EX_NOREDIRECTIONBITMAP.0 as isize != 0 {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex_style | WS_EX_LAYERED.0 as isize);
            SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA)
                .map_err(|e| format!("Could not prepare the desktop wallpaper layer: {e}"))?;
        }

        use windows::Win32::Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE,
        };
        let do_not_round: u32 = 1;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &do_not_round as *const _ as _,
            std::mem::size_of::<u32>() as u32,
        );

        let _ = SetWindowPos(
            hwnd,
            HWND::default(),
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );

        if GetAncestor(hwnd, GA_PARENT) != worker {
            let result = SetParent(hwnd, worker);
            if GetAncestor(hwnd, GA_PARENT) != worker {
                SetWindowLongPtrW(hwnd, GWL_STYLE, style);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);
                return Err(format!(
                    "Could not attach wallpaper to the desktop: {}",
                    result
                        .err()
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "Windows did not retain the desktop parent".into())
                ));
            }
        }

        let mut point = POINT {
            x: GetSystemMetrics(SM_XVIRTUALSCREEN),
            y: GetSystemMetrics(SM_YVIRTUALSCREEN),
        };
        let _ = ScreenToClient(worker, &mut point);
        let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        let shelldll = FindWindowExW(worker, HWND::default(), w!("SHELLDLL_DefView"), None).ok();
        let insert_after = match shelldll {
            Some(sh) if !sh.0.is_null() => sh,
            _ => HWND_TOP,
        };

        SetWindowPos(
            hwnd,
            insert_after,
            point.x,
            point.y,
            width,
            height,
            SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
        .map_err(|e| format!("Could not size wallpaper window: {e}"))?;
        if let Some(sh) = shelldll {
            if !sh.0.is_null() {
                let _ = ShowWindow(sh, SW_SHOW);
            }
        }
        if let Ok(static_worker) = FindWindowExW(worker, HWND::default(), w!("WorkerW"), None) {
            if static_worker != hwnd {
                let _ = ShowWindow(static_worker, SW_SHOW);
                let _ = SetWindowPos(
                    static_worker,
                    hwnd,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }

    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    let _ = w.set_size(tauri::Size::Physical(tauri::PhysicalSize {
        width: width as u32,
        height: height as u32,
    }));
    w.with_webview(|webview| unsafe {
        let controller = webview.controller();
        let _ = controller.NotifyParentWindowPositionChanged();
        let _ = controller.SetIsVisible(true);
    })
    .map_err(|e| e.to_string())?;
    let _ = w.eval("window.dispatchEvent(new Event('resize'))");
    Ok(())
}
fn repair_static_worker_order(parent: HWND, wallpaper: HWND) -> Result<(), String> {
    if let Ok(shelldll) = unsafe { FindWindowExW(parent, HWND::default(), w!("SHELLDLL_DefView"), None) } {
        if !shelldll.0.is_null() {
            let _ = unsafe { ShowWindow(shelldll, SW_SHOW) };
        }
    }
    let Ok(background) = (unsafe { FindWindowExW(parent, HWND::default(), w!("WorkerW"), None) })
    else {
        return Ok(());
    };
    if background == wallpaper {
        return Ok(());
    }
    let mut current = background;
    for _ in 0..1024 {
        current = match unsafe { GetWindow(current, GW_HWNDPREV) } {
            Ok(previous) => previous,
            Err(_) => break,
        };
        if current == wallpaper {
            let _ = unsafe { ShowWindow(background, SW_SHOW) };
            return Ok(());
        }
    }
    unsafe {
        let _ = ShowWindow(background, SW_SHOW);
        SetWindowPos(
            background,
            wallpaper,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    }
    .map_err(|e| format!("Could not restore the desktop layer order: {e}"))
}
fn ensure_window(app: &tauri::AppHandle) -> Result<(), String> {
    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    if app.get_webview_window("desktop-wallpaper").is_none() {
        tauri::WebviewWindowBuilder::new(
            app,
            "desktop-wallpaper",
            tauri::WebviewUrl::App("wallpaper.html".into()),
        )
        .title("Atelier Wallpaper")
        .decorations(false)
        .shadow(false)
        .skip_taskbar(true)
        .focused(false)
        .visible(false)
        .resizable(false)
        .position(0.0, 0.0)
        .inner_size(width as f64, height as f64)
        .data_directory(root().join("webview"))
        .additional_browser_args("--autoplay-policy=no-user-gesture-required --disable-background-timer-throttling --disable-renderer-backgrounding --disable-backgrounding-occluded-windows --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection")
        .build()
        .map_err(|e| e.to_string())?;
    }
    let w = app
        .get_webview_window("desktop-wallpaper")
        .ok_or("Wallpaper window unavailable")?;
    let existing = w.hwnd().ok().and_then(|hwnd| unsafe {
        let p = GetAncestor(HWND(hwnd.0 as *mut _), GA_PARENT);
        let mut name = [0u16; 64];
        let n = GetClassNameW(p, &mut name);
        (IsWindow(p).as_bool()
            && matches!(
                String::from_utf16_lossy(&name[..n.max(0) as usize]).as_str(),
                "Progman" | "WorkerW"
            ))
        .then_some(p)
    });
    let worker = match existing {
        Some(worker) => worker,
        None => atelier_wallpaper::workerw::acquire_workerw().map_err(|e| e.to_string())?,
    };
    w.set_ignore_cursor_events(true)
        .map_err(|e| e.to_string())?;
    w.show().map_err(|e| e.to_string())?;
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let worker_value = worker.0 as isize;
    app.run_on_main_thread(move || {
        let result = attach(&w, HWND(worker_value as *mut _));
        let _ = send.send(result);
    })
    .map_err(|e| e.to_string())?;
    receive
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| "Timed out attaching wallpaper to the desktop".to_string())?
}

pub fn repair_after_workarea_change(app: tauri::AppHandle) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static REVISION: AtomicU64 = AtomicU64::new(0);
    let revision = REVISION.fetch_add(1, Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(350));
        if REVISION.load(Ordering::SeqCst) != revision {
            return;
        }
        let state = app.state::<Wallpapers>();
        let active = state
            .config
            .lock()
            .map(|config| config.active.is_some())
            .unwrap_or(false);
        let failed = state
            .playback
            .lock()
            .map(|playback| playback.as_str() == "error")
            .unwrap_or(true);
        if !active || failed {
            return;
        }
        let Some(w) = app.get_webview_window("desktop-wallpaper") else {
            return;
        };
        let is_healthy = w
            .hwnd()
            .ok()
            .map(|hwnd| unsafe {
                let parent = GetAncestor(HWND(hwnd.0 as _), GA_PARENT);
                IsWindow(parent).as_bool()
                    && !parent.0.is_null()
                    && parent != GetDesktopWindow()
                    && (GetWindowLongPtrW(HWND(hwnd.0 as _), GWL_STYLE) & WS_VISIBLE.0 as isize != 0)
            })
            .unwrap_or(false);
        if is_healthy {
            return;
        }
        if let Err(error) = super::desktop_background::refresh().and_then(|active| {
            if active {
                ensure_window(&app)
            } else {
                Ok(())
            }
        }) {
            tracing::warn!(%error, "Could not restore wallpaper after the work area changed");
        }
    });
}
fn apply(app: &tauri::AppHandle, c: &WallpaperConfig) -> Result<(), String> {
    if c.active.is_some() {
        if let Err(error) = super::desktop_background::suspend().and_then(|_| ensure_window(app)) {
            let _ = super::desktop_background::restore();
            return Err(error);
        }
        app.emit_to("desktop-wallpaper", "wallpaper-changed", c)
            .map_err(|e| e.to_string())?;
    } else {
        if let Some(w) = app.get_webview_window("desktop-wallpaper") {
            w.destroy().map_err(|e| e.to_string())?;
        }
        super::desktop_background::restore()?;
    }
    Ok(())
}
#[tauri::command]
pub async fn wallpaper_apply(
    window: tauri::Window,
    app: tauri::AppHandle,
    state: tauri::State<'_, Wallpapers>,
    id: Option<String>,
) -> Result<WallpaperConfig, String> {
    ensure_studio(&window)?;
    let c = state.config.lock().map_err(|e| e.to_string())?.clone();
    if let Some(ref id) = id {
        let item = c
            .items
            .iter()
            .find(|i| &i.id == id)
            .ok_or("Wallpaper is not in your library")?;
        if item.kind == "video"
            && !root()
                .join(format!("{}.{}", item.id, item.category))
                .is_file()
        {
            return Err("Wallpaper video is missing".into());
        }
    }
    if let Some(item) = id
        .as_ref()
        .and_then(|id| c.items.iter().find(|i| &i.id == id))
    {
        if item.kind == "scene"
            && !root()
                .join(format!("{}.scene/scene.json", item.id))
                .is_file()
        {
            return Err("Scene assets are missing. Import the scene again.".into());
        }
    }
    let mut next = c.clone();
    next.active = id;
    save(&next)?;
    *state.config.lock().map_err(|e| e.to_string())? = next.clone();
    *state.playback.lock().map_err(|e| e.to_string())? = if next.active.is_some() {
        "loading"
    } else {
        "stopped"
    }
    .into();
    *state.playback_error.lock().map_err(|e| e.to_string())? = None;
    if let Err(error) = apply(&app, &next) {
        *state.config.lock().map_err(|e| e.to_string())? = c.clone();
        let _ = save(&c);
        let _ = apply(&app, &c);
        return Err(error);
    }
    Ok(next)
}
#[tauri::command]
pub async fn wallpaper_options(
    window: tauri::Window,
    app: tauri::AppHandle,
    state: tauri::State<'_, Wallpapers>,
    id: String,
    fit: String,
    paused: bool,
    mute: bool,
    volume: u32,
    pause_fullscreen: bool,
    pause_battery: bool,
) -> Result<WallpaperConfig, String> {
    ensure_studio(&window)?;
    if !["cover", "contain", "stretch"].contains(&fit.as_str()) {
        return Err("Invalid scaling option".into());
    }
    let mut c = state.config.lock().map_err(|e| e.to_string())?;
    if !c.items.iter().any(|item| item.id == id) {
        return Err("Wallpaper is not in your library".into());
    }
    let mut next = c.clone();
    next.settings.insert(
        id.clone(),
        WallpaperOptions {
            fit,
            paused,
            mute,
            volume: volume.min(100),
            pause_fullscreen,
            pause_battery,
            scene: c.options_for(&id).scene,
            audio: c.options_for(&id).audio,
        },
    );
    save(&next)?;
    *c = next.clone();
    drop(c);
    if next.active.as_deref() == Some(id.as_str()) {
        app.emit_to("desktop-wallpaper", "wallpaper-changed", &next)
            .map_err(|e| e.to_string())?;
    }
    Ok(next)
}

#[tauri::command]
pub fn wallpaper_audio_error(
    window: tauri::Window,
    app: tauri::AppHandle,
    message: String,
) -> Result<(), String> {
    if window.label() != "desktop-wallpaper" {
        return Err("Only the wallpaper can report audio errors".into());
    }
    let message: String = message.chars().take(300).collect();
    app.emit_to("main", "wallpaper-audio-error", message)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn wallpaper_audio_options(
    window: tauri::Window,
    app: tauri::AppHandle,
    state: tauri::State<'_, Wallpapers>,
    id: String,
    mut options: AudioEffects,
) -> Result<WallpaperConfig, String> {
    ensure_studio(&window)?;
    options.normalize();
    let next = {
        let mut c = state.config.lock().map_err(|e| e.to_string())?;
        if !c.items.iter().any(|item| item.id == id) {
            return Err("Wallpaper is not in your library".into());
        }
        let mut next = c.clone();
        next.settings.entry(id.clone()).or_default().audio = options;
        save(&next)?;
        *c = next.clone();
        next
    };
    if next.active.as_deref() == Some(id.as_str()) {
        app.emit_to("desktop-wallpaper", "wallpaper-changed", &next)
            .map_err(|e| e.to_string())?;
    }
    Ok(next)
}

#[tauri::command]
pub async fn wallpaper_scene_info(
    window: tauri::Window,
    app: tauri::AppHandle,
    id: String,
) -> Result<serde_json::Value, String> {
    ensure_studio(&window)?;
    let item = app
        .state::<Wallpapers>()
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .items
        .iter()
        .find(|i| i.id == id && i.kind == "scene" && valid_id(&id))
        .cloned()
        .ok_or("Unknown scene")?;
    tauri::async_runtime::spawn_blocking(move|| {
        static UPGRADES:Mutex<()>=Mutex::new(());
        let _guard=UPGRADES.lock().map_err(|e|e.to_string())?;
        let directory=root().join(format!("{id}.scene"));
        let mut manifest=super::scene_properties::metadata(&directory)?;
        let mut upgraded=false;
        if manifest["version"].as_u64().unwrap_or(0)<3 {
            if let Some(source)=super::steam_workshop::scene_source_for_title(&item.title) {
                let suffix=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
                let staged=root().join(format!("{id}-update-{suffix}.scene"));
                let backup=root().join(format!("{id}-backup-{suffix}.scene"));
                if super::wallpaper_scene::prepare(&source,&staged).is_ok() {
                    if std::fs::rename(&directory,&backup).is_ok() {
                        match std::fs::rename(&staged,&directory) {
                            Ok(())=>{upgraded=true;let _=std::fs::remove_dir_all(&backup);},
                            Err(_)=>{let _=std::fs::rename(&backup,&directory);},
                        }
                    }
                    if staged.is_dir(){let _=std::fs::remove_dir_all(&staged);}
                }
                if upgraded {
                    manifest=super::scene_properties::metadata(&directory)?;
                    let _=app.emit_to("desktop-wallpaper","wallpaper-scene-updated",&id);
                }
            }
        }
        Ok(serde_json::json!({"properties":manifest["properties"].as_array().cloned().unwrap_or_default(),"upgraded":upgraded,"legacy":manifest["version"].as_u64().unwrap_or(0)<3}))
    }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn wallpaper_scene_options(
    window: tauri::Window,
    app: tauri::AppHandle,
    state: tauri::State<'_, Wallpapers>,
    id: String,
    mut options: super::scene_properties::SceneOptions,
) -> Result<WallpaperConfig, String> {
    ensure_studio(&window)?;
    if !valid_id(&id) {
        return Err("Invalid wallpaper ID".into());
    }
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    if !config.items.iter().any(|i| i.id == id && i.kind == "scene") {
        return Err("Scene is not in your library".into());
    }
    let manifest = super::scene_properties::metadata(&root().join(format!("{id}.scene")))?;
    options.validate(
        manifest["properties"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or(&[]),
    );
    let mut next = config.clone();
    next.settings.entry(id.clone()).or_default().scene = options;
    save(&next)?;
    *config = next.clone();
    drop(config);
    if next.active.as_deref() == Some(&id) {
        app.emit_to("desktop-wallpaper", "wallpaper-changed", &next)
            .map_err(|e| e.to_string())?;
    }
    Ok(next)
}

#[tauri::command]
pub async fn wallpaper_performance_options(
    window: tauri::Window,
    app: tauri::AppHandle,
    state: tauri::State<'_, Wallpapers>,
    mut options: super::wallpaper_performance::PerformanceSettings,
) -> Result<WallpaperConfig, String> {
    ensure_studio(&window)?;
    options.normalize();
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    let mut next = config.clone();
    next.performance = options;
    save(&next)?;
    *config = next.clone();
    drop(config);
    app.emit_to("desktop-wallpaper", "wallpaper-changed", &next)
        .map_err(|e| e.to_string())?;
    Ok(next)
}

#[tauri::command]
pub async fn wallpaper_remove(
    window: tauri::Window,
    app: tauri::AppHandle,
    state: tauri::State<'_, Wallpapers>,
    id: String,
) -> Result<WallpaperConfig, String> {
    ensure_studio(&window)?;
    let mut c = state.config.lock().map_err(|e| e.to_string())?;
    let mut next = c.clone();
    let item = next
        .items
        .iter()
        .find(|i| i.id == id)
        .cloned()
        .ok_or("Unknown wallpaper")?;
    if next.active.as_deref() == Some(&id) {
        next.active = None;
        apply(&app, &next)?
    }
    next.items.retain(|i| i.id != id);
    next.settings.remove(&id);
    save(&next)?;
    *c = next.clone();
    drop(c);
    if next.active.is_none() {
        *state.playback.lock().map_err(|e| e.to_string())? = "stopped".into();
    }
    if item.kind == "scene" {
        let path = root().join(format!("{}.scene", item.id));
        if path.exists() {
            std::fs::remove_dir_all(path).map_err(|e| e.to_string())?;
        }
    }
    if item.kind == "video" {
        let path = root().join(format!("{}.{}", item.id, item.category));
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| e.to_string())?;
        }
    }
    Ok(next)
}
#[tauri::command]
pub async fn wallpaper_repair(window: tauri::Window, app: tauri::AppHandle) -> Result<(), String> {
    if window.label() != "desktop-wallpaper" {
        return Err("Only the wallpaper window can request a desktop refresh".into());
    }
    if app
        .state::<Wallpapers>()
        .playback
        .lock()
        .map_err(|e| e.to_string())?
        .as_str()
        == "error"
    {
        return Ok(());
    }
    tauri::async_runtime::spawn_blocking(move || {
        if !super::desktop_background::refresh()? {
            return Ok(());
        }
        let w = app
            .get_webview_window("desktop-wallpaper")
            .ok_or("Wallpaper window unavailable")?;
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        app.run_on_main_thread(move || {
            let result = w.hwnd().map_err(|e| e.to_string()).and_then(|raw| {
                let parent = unsafe { GetAncestor(HWND(raw.0 as _), GA_PARENT) };
                if parent.0.is_null()
                    || parent == unsafe { GetDesktopWindow() }
                    || !unsafe { IsWindow(parent) }.as_bool()
                {
                    return Err("Desktop layer was replaced".into());
                }
                let mut rect = windows::Win32::Foundation::RECT::default();
                unsafe { GetWindowRect(HWND(raw.0 as _), &mut rect) }.map_err(|e| e.to_string())?;
                let (vx, vy, vcx, vcy) = unsafe {
                    (
                        GetSystemMetrics(SM_XVIRTUALSCREEN),
                        GetSystemMetrics(SM_YVIRTUALSCREEN),
                        GetSystemMetrics(SM_CXVIRTUALSCREEN),
                        GetSystemMetrics(SM_CYVIRTUALSCREEN),
                    )
                };
                let visible = unsafe { GetWindowLongPtrW(HWND(raw.0 as _), GWL_STYLE) }
                    & WS_VISIBLE.0 as isize
                    != 0;
                let width_diff = ((rect.right - rect.left) - vcx).abs();
                let height_diff = ((rect.bottom - rect.top) - vcy).abs();
                let x_diff = (rect.left - vx).abs();
                let y_diff = (rect.top - vy).abs();
                let geometry_ok = width_diff <= 2 && height_diff <= 2 && x_diff <= 2 && y_diff <= 2;
                if visible && geometry_ok {
                    repair_static_worker_order(parent, HWND(raw.0 as _))
                } else {
                    attach(&w, parent)
                }
            });
            let _ = send.send(result);
        })
        .map_err(|e| e.to_string())?;
        match receive.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => ensure_window(&app),
            Err(_) => Err("Timed out refreshing the wallpaper".into()),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn wallpaper_should_pause(state: tauri::State<Wallpapers>) -> Result<bool, String> {
    Ok(wallpaper_pause_reason(state)?.is_some())
}
#[tauri::command]
pub fn wallpaper_pause_reason(state: tauri::State<Wallpapers>) -> Result<Option<String>, String> {
    Ok(wallpaper_playback_state(state)?.reason)
}
#[tauri::command]
pub fn wallpaper_playback_state(
    state: tauri::State<Wallpapers>,
) -> Result<super::wallpaper_performance::PlaybackState, String> {
    let options = {
        let c = state.config.lock().map_err(|e| e.to_string())?;
        let Some(id) = c.active.as_deref() else {
            return Ok(super::wallpaper_performance::PlaybackState {
                paused: true,
                muted: false,
                reason: Some("stopped".into()),
            });
        };
        c.options_for(id)
    };
    Ok(super::wallpaper_performance::PlaybackState {
        paused: options.paused,
        muted: options.mute,
        reason: options.paused.then(|| "paused".to_owned()),
    })
}
#[tauri::command]
pub fn wallpaper_report(
    window: tauri::Window,
    app: tauri::AppHandle,
    state: tauri::State<Wallpapers>,
    status: String,
    detail: Option<String>,
) -> Result<(), String> {
    if window.label() != "desktop-wallpaper"
        || ![
            "loading",
            "playing",
            "paused",
            "paused-fullscreen",
            "paused-battery",
            "paused-maximized",
            "paused-application",
            "paused-audio",
            "stopped",
            "error",
        ]
        .contains(&status.as_str())
    {
        return Err("Invalid status".into());
    }
    *state.playback.lock().map_err(|e| e.to_string())? = status.clone();
    *state.playback_error.lock().map_err(|e| e.to_string())? = if status == "error" {
        Some(
            detail
                .unwrap_or_else(|| "Wallpaper playback failed".into())
                .chars()
                .take(600)
                .collect(),
        )
    } else {
        None
    };
    if status == "error" {
        state
            .recovering
            .store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(w) = app.get_webview_window("desktop-wallpaper") {
            let _ = w.hide();
        }
        let _ = super::desktop_background::restore();
        let _ = app.emit_to(
            "main",
            "wallpaper-error",
            state
                .playback_error
                .lock()
                .map_err(|e| e.to_string())?
                .clone(),
        );
    }
    if status == "playing"
        && state
            .recovering
            .swap(false, std::sync::atomic::Ordering::SeqCst)
    {
        super::desktop_background::suspend()?;
        if let Some(w) = app.get_webview_window("desktop-wallpaper") {
            let _ = w.show();
            let _ = w.with_webview(|v| unsafe {
                let _ = v.controller().SetIsVisible(true);
            });
            let _ = w.eval("window.dispatchEvent(new Event('resize'))");
        }
    }
    let _ = app.emit_to("main", "wallpaper-status", status);
    Ok(())
}
pub fn start(app: &tauri::AppHandle) -> Result<(), String> {
    super::desktop_background::recover()?;
    let c = app
        .state::<Wallpapers>()
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    if c.active.is_some() {
        apply(app, &c)?
    }
    Ok(())
}
pub fn report_start_error(app: &tauri::AppHandle, error: &str) {
    if let Ok(mut playback) = app.state::<Wallpapers>().playback.lock() {
        *playback = "error".into();
    }
    if let Ok(mut detail) = app.state::<Wallpapers>().playback_error.lock() {
        *detail = Some(error.chars().take(600).collect());
    }
    let _ = app.emit_to("main", "wallpaper-error", error);
    let _ = app.emit_to("main", "wallpaper-status", "error");
}
pub(super) fn range(value: Option<&str>, len: u64) -> Result<(u64, u64, bool), ()> {
    if len == 0 {
        return Err(());
    }
    match value {
        None => Ok((0, len - 1, false)),
        Some(v) => {
            let raw = v.strip_prefix("bytes=").ok_or(())?;
            if raw.contains(',') {
                return Err(());
            }
            let (a, b) = raw.split_once('-').ok_or(())?;
            let (start, end) = if a.is_empty() {
                let suffix: u64 = b.parse().map_err(|_| ())?;
                if suffix == 0 {
                    return Err(());
                }
                (len.saturating_sub(suffix), len - 1)
            } else {
                let start: u64 = a.parse().map_err(|_| ())?;
                let end = if b.is_empty() {
                    len - 1
                } else {
                    b.parse::<u64>().map_err(|_| ())?.min(len - 1)
                };
                (start, end)
            };
            if start >= len || end < start {
                return Err(());
            }
            Ok((
                start,
                end.min(start.saturating_add(2 * 1024 * 1024 - 1)),
                true,
            ))
        }
    }
}
pub fn response(request: tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
    response_in(request, &root())
}
fn response_in(
    request: tauri::http::Request<Vec<u8>>,
    directory: &Path,
) -> tauri::http::Response<Vec<u8>> {
    let id = request.uri().path().trim_start_matches('/');
    if !valid_id(id) || request.method() != tauri::http::Method::GET {
        return tauri::http::Response::builder()
            .status(404)
            .body(Vec::new())
            .unwrap();
    }
    let file = ["mp4", "webm"].iter().find_map(|ext| {
        File::open(directory.join(format!("{id}.{ext}")))
            .ok()
            .map(|f| (f, *ext))
    });
    let Some((mut f, ext)) = file else {
        return tauri::http::Response::builder()
            .status(404)
            .body(Vec::new())
            .unwrap();
    };
    let Ok(len) = f.metadata().map(|m| m.len()) else {
        return tauri::http::Response::builder()
            .status(500)
            .body(Vec::new())
            .unwrap();
    };
    let header = request.headers().get("range").and_then(|h| h.to_str().ok());
    let Ok((start, end, partial)) = range(header, len) else {
        return tauri::http::Response::builder()
            .status(416)
            .header("Content-Range", format!("bytes */{len}"))
            .body(Vec::new())
            .unwrap();
    };
    let mut bytes = vec![0; (end - start + 1) as usize];
    if f.seek(SeekFrom::Start(start)).is_err() || f.read_exact(&mut bytes).is_err() {
        return tauri::http::Response::builder()
            .status(500)
            .body(Vec::new())
            .unwrap();
    }
    let mut res = tauri::http::Response::builder()
        .status(if partial { 206 } else { 200 })
        .header(
            "Content-Type",
            if ext == "mp4" {
                "video/mp4"
            } else {
                "video/webm"
            },
        )
        .header("Accept-Ranges", "bytes")
        .header("Content-Length", bytes.len());
    if partial {
        res = res.header("Content-Range", format!("bytes {start}-{end}/{len}"))
    }
    res.body(bytes).unwrap()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audio_effects_are_opt_in_bounded_and_saved_per_wallpaper() {
        let legacy: WallpaperOptions =
            serde_json::from_str(r#"{"fit":"contain","volume":37}"#).unwrap();
        assert!(!legacy.audio.enabled());
        let mut config = WallpaperConfig::default();
        config.settings.insert("one".into(), legacy.clone());
        config.settings.insert("two".into(), legacy);
        let audio = &mut config.settings.get_mut("one").unwrap().audio;
        audio.pulse = true;
        audio.glow = true;
        audio.pulse_strength = 999;
        audio.glow_strength = 150;
        audio.normalize();
        let restored: WallpaperConfig =
            serde_json::from_slice(&serde_json::to_vec(&config).unwrap()).unwrap();
        assert_eq!(restored.options_for("one").audio.pulse_strength, 100);
        assert_eq!(restored.options_for("one").audio.glow_strength, 100);
        assert_eq!(restored.options_for("one").volume, 37);
        assert!(!restored.options_for("two").audio.enabled());
    }
    #[test]
    fn legacy_settings_are_migrated_per_wallpaper_and_remain_independent() {
        let mut c: WallpaperConfig=serde_json::from_str(r#"{"items":[{"id":"preset-aurora","title":"Aurora","category":"Nature","kind":"preset","description":"A"},{"id":"preset-ocean","title":"Ocean","category":"Nature","kind":"preset","description":"B"}],"active":"preset-aurora","fit":"contain","mute":false,"volume":37,"pause_battery":true}"#).unwrap();
        c.migrate_settings();
        assert_eq!(
            c.options_for("preset-aurora"),
            WallpaperOptions {
                fit: "contain".into(),
                mute: false,
                volume: 37,
                pause_battery: true,
                ..Default::default()
            }
        );
        assert_eq!(c.options_for("preset-ocean").volume, 37);
        assert_eq!(c.options_for("preset-ocean").fit, "contain");
        c.settings.get_mut("preset-ocean").unwrap().volume = 88;
        assert_eq!(c.options_for("preset-aurora").volume, 37);
        let saved = serde_json::to_vec(&c).unwrap();
        let restored: WallpaperConfig = serde_json::from_slice(&saved).unwrap();
        assert_eq!(restored.options_for("preset-ocean").volume, 88);
        assert_eq!(restored.options_for("preset-aurora").volume, 37);
    }
    #[test]
    fn ranges_support_streaming_and_reject_bad_input() {
        assert_eq!(range(Some("bytes=10-19"), 100), Ok((10, 19, true)));
        assert_eq!(range(Some("bytes=-5"), 100), Ok((95, 99, true)));
        assert!(range(Some("bytes=101-"), 100).is_err());
        assert!(range(Some("bytes=1-2,3-4"), 100).is_err());
        assert_eq!(range(None, 10), Ok((0, 9, false)))
    }
    #[test]
    fn media_protocol_serves_ranges_and_blocks_path_traversal() {
        let root = std::env::temp_dir().join(format!("atelier-video-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("123.mp4"), b"0123456789").unwrap();
        let req = tauri::http::Request::builder()
            .uri("http://wallpaper-media.localhost/123")
            .header("Range", "bytes=3-6")
            .body(Vec::new())
            .unwrap();
        let res = response_in(req, &root);
        assert_eq!(res.status(), 206);
        assert_eq!(res.body(), b"3456");
        assert_eq!(res.headers()["content-range"], "bytes 3-6/10");
        let bad = tauri::http::Request::builder()
            .uri("http://wallpaper-media.localhost/abc")
            .body(Vec::new())
            .unwrap();
        assert_eq!(response_in(bad, &root).status(), 404);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn preset_video_names_are_safe() {
        assert!(valid_preset_asset("night-sky.mp4"));
        assert!(valid_preset_asset("forest.webm"));
        assert!(!valid_preset_asset("../secret.mp4"));
        assert!(!valid_preset_asset("video.exe"));
    }
    #[test]
    fn preset_ids_are_unique() {
        let p = presets();
        let ids: std::collections::HashSet<_> = p.iter().map(|p| &p.id).collect();
        assert_eq!(p.len(), ids.len());
    }
}
