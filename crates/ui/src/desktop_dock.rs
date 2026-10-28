use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::{Emitter, Manager};
use windows::{
    core::{w, Interface, PCWSTR},
    Win32::{
        Foundation::{BOOL, HWND, LPARAM},
        System::{Com::*, Threading::*},
        UI::{Shell::*, WindowsAndMessaging::*},
    },
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct DockConfig {
    pub explicit_desktop_opt_in: bool,
    pub enabled: bool,
    pub position: String,
    pub alignment: String,
    pub monitor: usize,
    pub icon_size: u32,
    pub spacing: u32,
    pub padding: u32,
    pub edge_gap: u32,
    pub radius: u32,
    pub opacity: u32,
    pub tint: String,
    pub magnification: u32,
    pub icon_style: String,
    pub inactive_opacity: u32,
    pub active_background: bool,
    pub indicator: String,
    pub indicator_color: String,
    pub show_labels: bool,
    pub show_running: bool,
    pub show_clock: bool,
    pub auto_hide: bool,
    pub hide_delay: u32,
    pub hide_windows_taskbar: bool,
    pub pinned: Vec<Pin>,
    pub imported: bool,
}
impl Default for DockConfig {
    fn default() -> Self {
        Self {
            explicit_desktop_opt_in: false,
            enabled: false,
            position: "bottom".into(),
            alignment: "center".into(),
            monitor: 0,
            icon_size: 44,
            spacing: 6,
            padding: 10,
            edge_gap: 10,
            radius: 18,
            opacity: 72,
            tint: "#232532".into(),
            magnification: 135,
            icon_style: "original".into(),
            inactive_opacity: 85,
            active_background: true,
            indicator: "dot".into(),
            indicator_color: "#f3f0ea".into(),
            show_labels: true,
            show_running: true,
            show_clock: false,
            auto_hide: false,
            hide_delay: 700,
            hide_windows_taskbar: false,
            pinned: Vec::new(),
            imported: false,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Pin {
    pub path: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}
impl DockConfig {
    fn validate(&mut self) -> Result<(), String> {
        if !["bottom", "top", "left", "right"].contains(&self.position.as_str())
            || !["start", "center", "end"].contains(&self.alignment.as_str())
            || !["original", "rounded", "monochrome"].contains(&self.icon_style.as_str())
            || !["dot", "line", "none"].contains(&self.indicator.as_str())
        {
            return Err("Unknown dock option".into());
        }
        for color in [&self.tint, &self.indicator_color] {
            if color.len() != 7
                || !color.starts_with('#')
                || !color[1..].bytes().all(|v| v.is_ascii_hexdigit())
            {
                return Err("Invalid dock color".into());
            }
        }
        self.icon_size = self.icon_size.clamp(20, 96);
        self.spacing = self.spacing.min(24);
        self.padding = self.padding.clamp(4, 28);
        self.edge_gap = self.edge_gap.min(100);
        self.radius = self.radius.min(48);
        self.opacity = self.opacity.clamp(10, 100);
        self.magnification = self.magnification.clamp(100, 180);
        self.inactive_opacity = self.inactive_opacity.clamp(20, 100);
        self.hide_delay = self.hide_delay.clamp(200, 3000);
        self.hide_windows_taskbar = self.enabled;
        if self.pinned.len() > 64 {
            return Err("A dock supports up to 64 pinned apps".into());
        }
        let mut seen = HashSet::new();
        self.pinned.retain(|p| seen.insert(key(&p.path)));
        Ok(())
    }
}
fn path() -> PathBuf {
    super::settings_path()
        .unwrap_or_else(|| std::env::temp_dir().join("Atelier/settings.json"))
        .with_file_name("dock-settings.json")
}
fn persist(c: &DockConfig) -> Result<(), String> {
    let p = path();
    std::fs::create_dir_all(p.parent().ok_or("Invalid settings path")?)
        .map_err(|e| e.to_string())?;
    let temporary = p.with_extension("json.tmp");
    std::fs::write(
        &temporary,
        serde_json::to_vec_pretty(c).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(temporary, p).map_err(|e| e.to_string())
}
pub struct DockState {
    config: Mutex<DockConfig>,
    write: Mutex<()>,
    icons: Mutex<HashMap<String, String>>,
    hidden: Mutex<Vec<isize>>,
    visible_size: Mutex<(f64, f64)>,
    stopping: AtomicBool,
}
impl DockState {
    pub fn new() -> Self {
        let mut config = std::fs::read(path())
            .ok()
            .and_then(|s| serde_json::from_slice::<DockConfig>(&s).ok())
            .filter(|c| {
                let mut c = c.clone();
                c.validate().is_ok()
            })
            .unwrap_or_default();
        if !config.explicit_desktop_opt_in {
            config.enabled = false;
        }
        config.hide_windows_taskbar = config.enabled;
        Self {
            config: Mutex::new(config),
            write: Mutex::new(()),
            icons: Mutex::new(HashMap::new()),
            hidden: Mutex::new(Vec::new()),
            visible_size: Mutex::new((320., 110.)),
            stopping: AtomicBool::new(false),
        }
    }
    pub fn restore_taskbar(&self) {
        if let Ok(mut hidden) = self.hidden.lock() {
            for handle in hidden.drain(..) {
                show_taskbar(handle)
            }
        }
    }
    fn maintain_native_taskbar(&self) -> Result<(), String> {
        if super::dock_test_mode() {
            self.restore_taskbar();
            return Ok(());
        }
        let hide = self
            .config
            .lock()
            .map(|c| c.enabled && c.hide_windows_taskbar)
            .unwrap_or(false);
        if !hide || self.stopping.load(Ordering::Relaxed) {
            self.restore_taskbar();
            return Ok(());
        }
        let mut hidden = self.hidden.lock().map_err(|e| e.to_string())?;
        for hwnd in atelier_taskbar::discovery::find_all_taskbars() {
            let handle = hwnd.0 as isize;
            if !hidden.contains(&handle) {
                if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
                    continue;
                }
                spawn_watchdog(handle)?;
                hidden.push(handle);
            }
            let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
        }
        Ok(())
    }
    pub fn shutdown(&self) {
        self.stopping.store(true, Ordering::Relaxed);
        self.restore_taskbar()
    }
}
fn key(s: &str) -> String {
    s.replace('/', "\\").to_lowercase()
}
fn hwnd(v: isize) -> HWND {
    HWND(v as *mut std::ffi::c_void)
}
fn show_taskbar(handle: isize) {
    let mut class = [0u16; 80];
    let n = unsafe { GetClassNameW(hwnd(handle), &mut class) };
    if n > 0
        && ["Shell_TrayWnd", "Shell_SecondaryTrayWnd"]
            .contains(&String::from_utf16_lossy(&class[..n as usize]).as_str())
    {
        let _ = unsafe { ShowWindow(hwnd(handle), SW_SHOWNA) };
    }
}
pub(crate) fn spawn_watchdog(handle: isize) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let name = format!(
        "Local\\Atelier.Dock.Recovery.{}.{}",
        std::process::id(),
        handle
    );
    let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let event = unsafe { CreateEventW(None, true, false, PCWSTR(wide.as_ptr())) }
        .map_err(|e| e.to_string())?;
    let spawn = std::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
        .args([
            "--dock-watchdog",
            &std::process::id().to_string(),
            &handle.to_string(),
            &name,
        ])
        .creation_flags(0x08000000)
        .spawn();
    let mut child = match spawn {
        Ok(c) => c,
        Err(e) => {
            let _ = unsafe { windows::Win32::Foundation::CloseHandle(event) };
            return Err(e.to_string());
        }
    };
    let ready =
        unsafe { WaitForSingleObject(event, 2000) } == windows::Win32::Foundation::WAIT_OBJECT_0;
    let _ = unsafe { windows::Win32::Foundation::CloseHandle(event) };
    if !ready {
        let _ = child.kill();
        let _ = child.wait();
        return Err(
            "The taskbar recovery guardian could not start; Windows taskbar was kept visible"
                .into(),
        );
    }
    Ok(())
}
pub fn watchdog_args() -> bool {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some("--dock-watchdog") {
        return false;
    }
    if let (Some(pid), Some(handle)) = (
        args.get(2).and_then(|s| s.parse::<u32>().ok()),
        args.get(3).and_then(|s| s.parse::<isize>().ok()),
    ) {
        if let Ok(process) = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) } {
            if let Some(name) = args.get(4) {
                let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
                if let Ok(event) =
                    unsafe { OpenEventW(EVENT_MODIFY_STATE, false, PCWSTR(wide.as_ptr())) }
                {
                    unsafe {
                        let _ = SetEvent(event);
                        let _ = windows::Win32::Foundation::CloseHandle(event);
                    }
                }
            }
            unsafe {
                WaitForSingleObject(process, u32::MAX);
                let _ = windows::Win32::Foundation::CloseHandle(process);
            }
        }
        show_taskbar(handle);
    }
    true
}

#[derive(Clone)]
struct Running {
    handle: isize,
    pid: u32,
    path: String,
    name: String,
    active: bool,
}
fn running_windows() -> Vec<Running> {
    unsafe extern "system" fn visit(handle: HWND, arg: LPARAM) -> BOOL {
        let list = &mut *(arg.0 as *mut Vec<Running>);
        if !IsWindowVisible(handle).as_bool()
            || GetWindow(handle, GW_OWNER).is_ok_and(|h| !h.0.is_null())
            || (GetWindowLongW(handle, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0) != 0
        {
            return BOOL(1);
        }
        let mut title = [0u16; 512];
        let len = GetWindowTextW(handle, &mut title);
        let mut pid = 0;
        GetWindowThreadProcessId(handle, Some(&mut pid));
        if len == 0 || pid == std::process::id() {
            return BOOL(1);
        }
        let process = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(p) => p,
            Err(_) => return BOOL(1),
        };
        let mut buffer = [0u16; 32768];
        let mut size = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut size,
        );
        let _ = windows::Win32::Foundation::CloseHandle(process);
        if result.is_ok() {
            let path = String::from_utf16_lossy(&buffer[..size as usize]);
            if !path.to_lowercase().ends_with("\\applicationframehost.exe")
                && !path.to_lowercase().ends_with("\\shellexperiencehost.exe")
            {
                list.push(Running {
                    handle: handle.0 as isize,
                    pid,
                    path,
                    name: String::from_utf16_lossy(&title[..len as usize]),
                    active: handle == GetForegroundWindow(),
                });
            }
        }
        BOOL(1)
    }
    let mut list = Vec::new();
    let _ = unsafe { EnumWindows(Some(visit), LPARAM(&mut list as *mut _ as isize)) };
    list
}
fn shortcut_target(path: &str) -> Option<String> {
    let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        link.cast::<IPersistFile>()
            .ok()?
            .Load(PCWSTR(wide.as_ptr()), STGM_READ)
            .ok()?;
        let mut buffer = [0u16; 32768];
        link.GetPath(&mut buffer, std::ptr::null_mut(), 0).ok()?;
        let n = buffer.iter().position(|c| *c == 0)?;
        if n == 0 {
            None
        } else {
            Some(String::from_utf16_lossy(&buffer[..n]))
        }
    }
}
fn file_icon(path: &str, state: &DockState) -> String {
    if let Ok(cache) = state.icons.lock() {
        if let Some(icon) = cache.get(path) {
            return icon.clone();
        }
    }
    let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    let mut info = SHFILEINFOW::default();
    let result = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    let icon = if result != 0 && !info.hIcon.0.is_null() {
        let value = super::hicon_to_bmp_base64(info.hIcon, 64).unwrap_or_default();
        let _ = unsafe { DestroyIcon(info.hIcon) };
        value
    } else {
        String::new()
    };
    if !icon.is_empty() {
        if let Ok(mut cache) = state.icons.lock() {
            if cache.len() > 256 {
                cache.clear()
            }
            cache.insert(path.into(), icon.clone());
        }
    }
    icon
}

fn window_icon(running: &[&Running]) -> String {
    for window in running {
        let handle = hwnd(window.handle);
        let mut result = 0usize;
        for kind in [2, 0, 1] {
            let delivered = unsafe {
                SendMessageTimeoutW(
                    handle,
                    WM_GETICON,
                    windows::Win32::Foundation::WPARAM(kind),
                    LPARAM(0),
                    SMTO_ABORTIFHUNG,
                    100,
                    Some(&mut result),
                )
            };
            if delivered.0 != 0 && result != 0 {
                if let Some(icon) = super::hicon_to_bmp_base64(HICON(result as *mut _), 64) {
                    return icon;
                }
            }
        }
        let icon = unsafe { GetClassLongPtrW(handle, GCLP_HICON) };
        if icon != 0 {
            if let Some(icon) = super::hicon_to_bmp_base64(HICON(icon as *mut _), 64) {
                return icon;
            }
        }
    }
    String::new()
}

#[derive(Serialize)]
pub struct DockItem {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub pinned: bool,
    pub running: bool,
    pub active: bool,
    pub windows: usize,
}
#[derive(Serialize)]
pub struct DockSnapshot {
    pub config: DockConfig,
    pub items: Vec<DockItem>,
    pub monitors: Vec<String>,
}
fn items(state: &DockState, c: &DockConfig) -> Vec<DockItem> {
    let running = running_windows();
    let mut used = HashSet::new();
    let mut result = Vec::new();
    for pin in &c.pinned {
        let target = if pin.path.to_lowercase().ends_with(".lnk") {
            shortcut_target(&pin.path).unwrap_or_else(|| pin.path.clone())
        } else {
            pin.path.clone()
        };
        let matches: Vec<_> = running
            .iter()
            .filter(|r| key(&r.path) == key(&target))
            .collect();
        for r in &matches {
            used.insert(r.handle);
        }
        result.push(DockItem {
            id: pin.path.clone(),
            name: pin.name.clone(),
            icon: pin.icon.clone().unwrap_or_else(|| {
                let mut icon = file_icon(&pin.path, state);
                if icon.is_empty() && target != pin.path {
                    icon = file_icon(&target, state);
                }
                if icon.is_empty() {
                    icon = window_icon(&matches);
                }
                icon
            }),
            pinned: true,
            running: !matches.is_empty(),
            active: matches.iter().any(|r| r.active),
            windows: matches.len(),
        });
    }
    if c.show_running {
        let mut groups: Vec<(String, Vec<&Running>)> = Vec::new();
        for r in &running {
            if used.contains(&r.handle) {
                continue;
            }
            let k = key(&r.path);
            if let Some((_, group)) = groups.iter_mut().find(|(p, _)| key(p) == k) {
                group.push(r)
            } else {
                groups.push((r.path.clone(), vec![r]));
            }
        }
        for (path, group) in groups {
            result.push(DockItem {
                id: path.clone(),
                name: PathBuf::from(&path)
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&group[0].name)
                    .into(),
                icon: {
                    let icon = file_icon(&path, state);
                    if icon.is_empty() {
                        window_icon(&group)
                    } else {
                        icon
                    }
                },
                pinned: false,
                running: true,
                active: group.iter().any(|r| r.active),
                windows: group.len(),
            });
        }
    }
    result
}
fn import_pins(c: &mut DockConfig) {
    if c.imported {
        return;
    }
    c.imported = true;
    if let Some(root) = std::env::var_os("APPDATA") {
        if let Ok(entries) = std::fs::read_dir(
            PathBuf::from(root)
                .join("Microsoft/Internet Explorer/Quick Launch/User Pinned/TaskBar"),
        ) {
            let mut paths: Vec<_> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("lnk")))
                .collect();
            paths.sort();
            for p in paths {
                if c.pinned.len() >= 64 {
                    break;
                }
                c.pinned.push(Pin {
                    name: p.file_stem().unwrap_or_default().to_string_lossy().into(),
                    path: p.to_string_lossy().into(),
                    icon: None,
                })
            }
        }
    }
}

#[derive(Debug, PartialEq)]
struct Placement {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
}
fn placement(
    c: &DockConfig,
    origin: (i32, i32),
    size: (u32, u32),
    scale: f64,
    requested: (f64, f64),
    collapsed: bool,
) -> Placement {
    let vertical = c.position == "left" || c.position == "right";
    let gap = ((c.edge_gap as f64 * scale).round() as i32)
        .min(((size.0.min(size.1) as i32 - 1).max(0)) / 2);
    let safe_w = (size.0 as i32 - 2 * gap).max(1) as u32;
    let safe_h = (size.1 as i32 - 2 * gap).max(1) as u32;
    let w = if collapsed && vertical {
        4
    } else {
        (requested.0 * scale).ceil() as u32
    }
    .clamp(1, safe_w);
    let h = if collapsed && !vertical {
        4
    } else {
        (requested.1 * scale).ceil() as u32
    }
    .clamp(1, safe_h);
    let align = |available: i32, extent: i32| match c.alignment.as_str() {
        "start" => gap,
        "end" => available - extent - gap,
        _ => (available - extent) / 2,
    };
    let (x, y) = match c.position.as_str() {
        "top" => (
            align(size.0 as i32, w as i32),
            if collapsed { 0 } else { gap },
        ),
        "left" => (
            if collapsed { 0 } else { gap },
            align(size.1 as i32, h as i32),
        ),
        "right" => (
            size.0 as i32 - w as i32 - if collapsed { 0 } else { gap },
            align(size.1 as i32, h as i32),
        ),
        _ => (
            align(size.0 as i32, w as i32),
            size.1 as i32 - h as i32 - if collapsed { 0 } else { gap },
        ),
    };
    Placement {
        x: origin.0 + x,
        y: origin.1 + y,
        w,
        h,
    }
}
fn place(app: &tauri::AppHandle, collapsed: bool) -> Result<(), String> {
    let state = app.state::<DockState>();
    let c = state.config.lock().map_err(|e| e.to_string())?.clone();
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let monitor = monitors
        .get(c.monitor)
        .or(monitors.first())
        .ok_or("No monitor available")?;
    let size = *state.visible_size.lock().map_err(|e| e.to_string())?;
    let area = monitor.work_area();
    let (origin, extent) = if c.hide_windows_taskbar {
        (
            (monitor.position().x, monitor.position().y),
            (monitor.size().width, monitor.size().height),
        )
    } else {
        (
            (area.position.x, area.position.y),
            (area.size.width, area.size.height),
        )
    };
    let p = placement(
        &c,
        origin,
        extent,
        monitor.scale_factor(),
        size,
        collapsed && c.auto_hide,
    );
    if let Some(window) = app.get_webview_window("desktop-dock") {
        window
            .set_size(tauri::PhysicalSize::new(p.w, p.h))
            .map_err(|e| e.to_string())?;
        window
            .set_position(tauri::PhysicalPosition::new(p.x, p.y))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn ensure_window(app: &tauri::AppHandle) -> Result<(), String> {
    if app.get_webview_window("desktop-dock").is_none() {
        tauri::WebviewWindowBuilder::new(
            app,
            "desktop-dock",
            tauri::WebviewUrl::App("dock.html".into()),
        )
        .title("Atelier Dock")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .disable_drag_drop_handler()
        .inner_size(320., 110.)
        .build()
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn apply(app: &tauri::AppHandle, c: &DockConfig) -> Result<(), String> {
    if c.enabled {
        ensure_window(app)?;
        place(app, false)?;
        app.get_webview_window("desktop-dock")
            .ok_or("Dock window unavailable")?
            .show()
            .map_err(|e| e.to_string())?;
    } else {
        if let Some(window) = app.get_webview_window("desktop-dock") {
            window.hide().map_err(|e| e.to_string())?;
        }
    }
    app.state::<DockState>().maintain_native_taskbar()?;
    app.emit("dock-config-changed", c)
        .map_err(|e| e.to_string())?;
    Ok(())
}
pub fn start(app: &tauri::AppHandle) -> Result<(), String> {
    let c = app
        .state::<DockState>()
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    apply(app, &c)?;
    let app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(2));
        let state = app.state::<DockState>();
        if state.stopping.load(Ordering::Relaxed) {
            break;
        }
        if let Err(error) = state.maintain_native_taskbar() {
            tracing::warn!(%error,"Could not hide native taskbar");
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn get_dock_snapshot(app: tauri::AppHandle) -> Result<DockSnapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
        let state = app.state::<DockState>();
        let c = state.config.lock().map_err(|e| e.to_string())?.clone();
        let result = DockSnapshot {
            items: items(&state, &c),
            config: c,
            monitors: app
                .available_monitors()
                .map_err(|e| e.to_string())?
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    m.name()
                        .cloned()
                        .unwrap_or_else(|| format!("Display {}", i + 1))
                })
                .collect(),
        };
        if initialized {
            unsafe { CoUninitialize() }
        }
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn set_dock_config(
    app: tauri::AppHandle,
    mut config: DockConfig,
) -> Result<DockConfig, String> {
    config.validate()?;
    config.explicit_desktop_opt_in = true;
    let state = app.state::<DockState>();
    let _write = state.write.lock().map_err(|e| e.to_string())?;
    let old = state.config.lock().map_err(|e| e.to_string())?.clone();
    config.pinned = old.pinned.clone();
    config.imported = old.imported;
    if config.enabled {
        import_pins(&mut config)
    }
    *state.config.lock().map_err(|e| e.to_string())? = config.clone();
    if let Err(error) = apply(&app, &config).and_then(|_| persist(&config)) {
        *state.config.lock().map_err(|e| e.to_string())? = old.clone();
        let _ = apply(&app, &old);
        return Err(error);
    }
    Ok(config)
}
#[tauri::command]
pub async fn reset_desktop_dock(app: tauri::AppHandle) -> Result<DockConfig, String> {
    let state = app.state::<DockState>();
    let _write = state.write.lock().map_err(|e| e.to_string())?;
    let c = DockConfig::default();
    persist(&c)?;
    *state.config.lock().map_err(|e| e.to_string())? = c.clone();
    apply(&app, &c)?;
    Ok(c)
}
#[tauri::command]
pub fn dock_resize(
    app: tauri::AppHandle,
    width: f64,
    height: f64,
    collapsed: bool,
) -> Result<(), String> {
    if !width.is_finite() || !height.is_finite() {
        return Err("Invalid dock size".into());
    }
    *app.state::<DockState>()
        .visible_size
        .lock()
        .map_err(|e| e.to_string())? = (width.clamp(48., 8192.), height.clamp(48., 8192.));
    place(&app, collapsed)
}
#[tauri::command]
pub async fn dock_activate(
    app: tauri::AppHandle,
    id: String,
    new_instance: bool,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
        let result = (|| {
            let c = app
                .state::<DockState>()
                .config
                .lock()
                .map_err(|e| e.to_string())?
                .clone();
            let pin = c.pinned.iter().find(|p| p.path == id);
            let target = pin
                .map(|p| {
                    if p.path.to_lowercase().ends_with(".lnk") {
                        shortcut_target(&p.path).unwrap_or_else(|| p.path.clone())
                    } else {
                        p.path.clone()
                    }
                })
                .unwrap_or_else(|| id.clone());
            let running = running_windows();
            let matches: Vec<_> = running
                .iter()
                .filter(|r| key(&r.path) == key(&target))
                .collect();
            if !new_instance && !matches.is_empty() {
                let index = matches
                    .iter()
                    .position(|r| r.active)
                    .map(|i| (i + 1) % matches.len())
                    .unwrap_or(0);
                let r = matches[index];
                let mut current_pid = 0;
                unsafe { GetWindowThreadProcessId(hwnd(r.handle), Some(&mut current_pid)) };
                if current_pid != r.pid {
                    return Err("This window has closed".into());
                }
                if unsafe { IsIconic(hwnd(r.handle)) }.as_bool() {
                    let _ = unsafe { ShowWindowAsync(hwnd(r.handle), SW_RESTORE) };
                }
                if !unsafe { SetForegroundWindow(hwnd(r.handle)) }.as_bool() {
                    return Err("Windows did not allow this window to be activated".into());
                }
                return Ok(());
            }
            if pin.is_none() && !running.iter().any(|r| r.path == id) {
                return Err("App no longer available".into());
            }
            let wide: Vec<_> = id.encode_utf16().chain(Some(0)).collect();
            let result = unsafe {
                ShellExecuteW(
                    HWND::default(),
                    w!("open"),
                    PCWSTR(wide.as_ptr()),
                    None,
                    None,
                    SW_SHOWNORMAL,
                )
            };
            if result.0 as isize <= 32 {
                return Err("Could not launch app".into());
            }
            Ok(())
        })();
        if initialized {
            unsafe { CoUninitialize() }
        }
        result
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn dock_pin(app: tauri::AppHandle, id: String, pinned: bool) -> Result<(), String> {
    let state = app.state::<DockState>();
    let _write = state.write.lock().map_err(|e| e.to_string())?;
    let mut c = state.config.lock().map_err(|e| e.to_string())?.clone();
    if pinned {
        let running = running_windows();
        let r = running
            .iter()
            .find(|r| r.path == id)
            .ok_or("App no longer available")?;
        if !c.pinned.iter().any(|p| p.path == id) {
            c.pinned.push(Pin {
                path: id,
                name: PathBuf::from(&r.path)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into(),
                icon: None,
            })
        }
    } else {
        c.pinned.retain(|p| p.path != id)
    }
    c.validate()?;
    persist(&c)?;
    *state.config.lock().map_err(|e| e.to_string())? = c.clone();
    app.emit("dock-config-changed", &c)
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
pub async fn dock_reorder(app: tauri::AppHandle, ids: Vec<String>) -> Result<(), String> {
    let state = app.state::<DockState>();
    let _write = state.write.lock().map_err(|e| e.to_string())?;
    let mut c = state.config.lock().map_err(|e| e.to_string())?.clone();
    if ids.len() != c.pinned.len()
        || ids.iter().collect::<HashSet<_>>().len() != ids.len()
        || ids.iter().any(|id| !c.pinned.iter().any(|p| &p.path == id))
    {
        return Err("Pinned apps changed; try reordering again".into());
    }
    c.pinned = ids
        .iter()
        .filter_map(|id| c.pinned.iter().find(|p| &p.path == id).cloned())
        .collect();
    persist(&c)?;
    *state.config.lock().map_err(|e| e.to_string())? = c.clone();
    app.emit("dock-config-changed", &c)
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
pub async fn dock_set_icon(
    app: tauri::AppHandle,
    id: String,
    icon: Option<String>,
) -> Result<(), String> {
    if let Some(data) = &icon {
        let body = data
            .strip_prefix("data:image/png;base64,")
            .ok_or("Custom icons must be PNG images")?;
        if data.len() > 262144
            || !body.starts_with("iVBORw0KGgo")
            || !body
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b))
        {
            return Err("Invalid PNG icon, or icon is too large".into());
        }
    }
    let state = app.state::<DockState>();
    let _write = state.write.lock().map_err(|e| e.to_string())?;
    let mut c = state.config.lock().map_err(|e| e.to_string())?.clone();
    let pin = c
        .pinned
        .iter_mut()
        .find(|p| p.path == id)
        .ok_or("Pin the app before changing its icon")?;
    pin.icon = icon;
    persist(&c)?;
    *state.config.lock().map_err(|e| e.to_string())? = c.clone();
    app.emit("dock-config-changed", &c)
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
pub fn dock_open_studio(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        w.show().map_err(|e| e.to_string())?;
        w.unminimize().map_err(|e| e.to_string())?;
        w.set_focus().map_err(|e| e.to_string())?
    }
    Ok(())
}

#[tauri::command]
pub fn dock_native_test_probe(
    app: tauri::AppHandle,
    report: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    if !super::dock_test_mode() {
        return Err("Native test mode is not enabled".into());
    }
    if let Some(report) = report {
        std::fs::write(
            path().with_file_name("dock-native-report.json"),
            serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    let state = app.state::<DockState>();
    let c = state.config.lock().map_err(|e| e.to_string())?.clone();
    if let Some(window) = app.get_webview_window("desktop-dock") {
        let pos = window.outer_position().map_err(|e| e.to_string())?;
        let size = window.inner_size().map_err(|e| e.to_string())?;
        let monitors = app.available_monitors().map_err(|e| e.to_string())?;
        let m = monitors
            .get(c.monitor)
            .or(monitors.first())
            .ok_or("No monitor")?;
        let fixture = running_windows()
            .into_iter()
            .find(|r| r.name == "Atelier Dock test window")
            .map(|r| r.path);
        return Ok(
            serde_json::json!({"position":{"x":pos.x,"y":pos.y},"size":{"width":size.width,"height":size.height},"area":if c.hide_windows_taskbar { serde_json::json!({"position":m.position(),"size":m.size()}) } else { serde_json::json!(m.work_area()) },"visible":window.is_visible().map_err(|e|e.to_string())?,"config":c,"fixture":fixture}),
        );
    }
    Ok(serde_json::json!({"visible":false,"config":c}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_edges_and_negative_monitor_origin() {
        for pos in ["left", "right", "top", "bottom"] {
            let c = DockConfig {
                position: pos.into(),
                ..Default::default()
            };
            let p = placement(&c, (-1920, 0), (1920, 1080), 1.5, (500., 100.), false);
            assert!(p.x >= -1920 && p.y >= 0);
            assert!(p.x + p.w as i32 <= 0 && p.y + p.h as i32 <= 1080);
        }
    }
    #[test]
    fn oversized_dock_stays_inside_monitor() {
        let c = DockConfig {
            edge_gap: 100,
            ..Default::default()
        };
        let p = placement(&c, (0, 0), (640, 480), 2., (8000., 8000.), false);
        assert!(p.w <= 240 && p.h <= 80);
        assert!(p.x >= 0 && p.y >= 0)
    }
    #[test]
    fn auto_hide_strip_reaches_screen_edge() {
        let c = DockConfig::default();
        let p = placement(&c, (0, 0), (1920, 1080), 1., (500., 100.), true);
        assert_eq!(p.h, 4);
        assert_eq!(p.y, 1076)
    }
    #[test]
    fn edge_gap_cannot_push_dock_outside_a_small_work_area() {
        let c = DockConfig {
            edge_gap: 100,
            alignment: "start".into(),
            position: "left".into(),
            ..Default::default()
        };
        let p = placement(&c, (0, 0), (200, 150), 4., (300., 1000.), false);
        assert!(p.x >= 0 && p.y >= 0 && p.x + p.w as i32 <= 200 && p.y + p.h as i32 <= 150);
    }
    #[test]
    fn legacy_config_defaults_and_validation() {
        let mut c: DockConfig = serde_json::from_str("{\"icon_size\":1}").unwrap();
        c.validate().unwrap();
        assert_eq!(c.icon_size, 20);
        assert!(!c.enabled);
        c.position = "diagonal".into();
        assert!(c.validate().is_err())
    }
    #[test]
    fn dock_visibility_controls_windows_taskbar_visibility() {
        let mut c = DockConfig {
            enabled: true,
            hide_windows_taskbar: false,
            ..Default::default()
        };
        c.validate().unwrap();
        assert!(c.hide_windows_taskbar);
        c.enabled = false;
        c.validate().unwrap();
        assert!(!c.hide_windows_taskbar);
    }
    #[test]
    fn windows_executable_supplies_an_icon() {
        let state = DockState::new();
        let system_root = std::env::var("SystemRoot").unwrap();
        let path = format!("{system_root}\\explorer.exe");
        let icon = file_icon(&path, &state);
        assert!(
            icon.starts_with("data:image/png;base64,"),
            "No icon returned for {path}"
        );
    }
}
