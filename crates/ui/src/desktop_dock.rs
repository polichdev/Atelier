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
        Foundation::{BOOL, COLORREF, HWND, LPARAM, WPARAM},
        System::{Com::*, Threading::*},
        UI::{Shell::*, WindowsAndMessaging::*},
    },
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct DockAudioConfig {
    pub enabled: bool,
    pub color: String,
    pub strength: u32,
    pub shape: String,
    pub shape_strength: u32,
    pub sensitivity: u32,
    pub smoothing: u32,
    pub frequency: String,
}
impl Default for DockAudioConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            color: "#8b5cf6".into(),
            strength: 45,
            shape: "off".into(),
            shape_strength: 40,
            sensitivity: 100,
            smoothing: 60,
            frequency: "mix".into(),
        }
    }
}
impl DockAudioConfig {
    fn validate(&mut self) -> Result<(), String> {
        if self.color.len() != 7
            || !self.color.starts_with('#')
            || !self.color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
        {
            return Err("Invalid audio glow color".into());
        }
        self.strength = self.strength.min(100);
        self.shape_strength = self.shape_strength.min(100);
        self.sensitivity = self.sensitivity.clamp(1, 200);
        self.smoothing = self.smoothing.min(95);
        if !["off", "breathe", "corners", "both"].contains(&self.shape.as_str())
            || !["mix", "bass", "mid", "treble"].contains(&self.frequency.as_str())
        {
            return Err("Invalid dock audio effect".into());
        }
        Ok(())
    }
}

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
    pub show_start: bool,
    pub show_search: bool,
    pub show_studio: bool,
    pub show_tray: bool,
    pub show_language: bool,
    pub show_network: bool,
    pub show_volume: bool,
    pub launch_animation: bool,
    pub maximize_on_open: bool,
    pub maximized_bottom_gap: u32,
    pub hidden_apps: Vec<Pin>,
    pub auto_hide: bool,
    pub hide_delay: u32,
    pub hide_windows_taskbar: bool,
    pub audio: DockAudioConfig,
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
            show_start: true,
            show_search: true,
            show_studio: true,
            show_tray: false,
            show_language: false,
            show_network: false,
            show_volume: false,
            launch_animation: true,
            maximize_on_open: true,
            maximized_bottom_gap: 0,
            hidden_apps: Vec::new(),
            auto_hide: false,
            hide_delay: 700,
            hide_windows_taskbar: false,
            audio: DockAudioConfig::default(),
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
    fn preserve_managed(&mut self, old: &Self) {
        self.pinned = old.pinned.clone();
        self.imported = old.imported;
        self.audio = old.audio.clone();
        self.hidden_apps = old.hidden_apps.clone();
    }
    fn validate(&mut self) -> Result<(), String> {
        self.audio.validate()?;
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
        self.maximized_bottom_gap = self.maximized_bottom_gap.min(200);
        self.radius = self.radius.min(48);
        self.opacity = self.opacity.clamp(10, 100);
        self.magnification = self.magnification.clamp(100, 180);
        self.inactive_opacity = self.inactive_opacity.clamp(20, 100);
        self.hide_delay = self.hide_delay.clamp(200, 3000);
        self.hide_windows_taskbar = self.enabled;
        if self.hidden_apps.len() > 256 {
            return Err("Too many hidden apps".into());
        }
        let mut hidden_seen = HashSet::new();
        self.hidden_apps
            .retain(|p| !p.path.is_empty() && hidden_seen.insert(key(&p.path)));
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
    appbar_lease: Mutex<Option<AppbarLease>>,
    language_target: Mutex<Option<(isize, u32)>>,
    gap_appbar: Mutex<Option<(isize, usize, u32)>>,
    fullscreen_hidden: AtomicBool,
    stopping: AtomicBool,
}
impl DockState {
    pub fn audio_effects_enabled(&self) -> bool {
        !self.fullscreen_hidden.load(Ordering::Relaxed)
            && self
                .config
                .lock()
                .map(|c| c.enabled && c.audio.enabled)
                .unwrap_or(false)
    }
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
            appbar_lease: Mutex::new(None),
            language_target: Mutex::new(None),
            gap_appbar: Mutex::new(None),
            fullscreen_hidden: AtomicBool::new(false),
            stopping: AtomicBool::new(false),
        }
    }
    pub fn restore_taskbar(&self) {
        self.remove_gap_appbar();
        if let Ok(mut lease) = self.appbar_lease.lock() {
            if let Some(lease) = lease.take() {
                lease.restore();
            }
        }
        if let Ok(mut hidden) = self.hidden.lock() {
            for handle in hidden.drain(..) {
                show_taskbar(handle);
            }
        }
        for taskbar in atelier_taskbar::discovery::find_all_taskbars() {
            show_taskbar(taskbar.0 as isize);
        }
        dismiss_stuck_input_hosts();
    }
    fn remove_gap_appbar(&self) {
        if let Ok(mut current) = self.gap_appbar.lock() {
            if let Some((handle, _, _)) = current.take() {
                let mut data = APPBARDATA {
                    cbSize: std::mem::size_of::<APPBARDATA>() as u32,
                    hWnd: hwnd(handle),
                    ..Default::default()
                };
                unsafe {
                    SHAppBarMessage(ABM_REMOVE, &mut data);
                }
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
                if is_primary_taskbar(hwnd) {
                    let mut lease = self.appbar_lease.lock().map_err(|e| e.to_string())?;
                    if lease.is_none() {
                        *lease = begin_appbar_autohide(hwnd)?;
                    }
                }
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
impl Drop for DockState {
    fn drop(&mut self) {
        self.shutdown();
    }
}
pub fn restore_all_taskbars() {
    for taskbar in atelier_taskbar::discovery::find_all_taskbars() {
        show_taskbar(taskbar.0 as isize);
    }
    dismiss_stuck_input_hosts();
}
fn key(s: &str) -> String {
    s.replace('/', "\\").to_lowercase()
}
fn hwnd(v: isize) -> HWND {
    HWND(v as *mut std::ffi::c_void)
}
fn is_primary_taskbar(handle: HWND) -> bool {
    let mut class = [0u16; 80];
    let n = unsafe { GetClassNameW(handle, &mut class) };
    n > 0 && String::from_utf16_lossy(&class[..n as usize]) == "Shell_TrayWnd"
}
fn primary_taskbar() -> Option<HWND> {
    atelier_taskbar::discovery::find_all_taskbars()
        .into_iter()
        .find(|handle| is_primary_taskbar(*handle))
}
fn appbar_state(hwnd: HWND) -> u32 {
    let mut data = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        ..Default::default()
    };
    unsafe { SHAppBarMessage(ABM_GETSTATE, &mut data) as u32 }
}
fn set_appbar_state(hwnd: HWND, state: u32) {
    let mut data = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        lParam: LPARAM(state as isize),
        ..Default::default()
    };
    unsafe {
        SHAppBarMessage(ABM_SETSTATE, &mut data);
    }
}
fn should_restore_appbar(original: u32, applied: u32, observed: u32) -> bool {
    original != applied && observed == applied
}
fn restore_appbar_if_owned(original: u32, applied: u32) {
    if let Some(hwnd) = primary_taskbar() {
        if should_restore_appbar(original, applied, appbar_state(hwnd)) {
            set_appbar_state(hwnd, original);
        }
    }
}
struct AppbarLease {
    original: u32,
    applied: u32,
    cancel: isize,
}
impl AppbarLease {
    fn restore(self) {
        restore_appbar_if_owned(self.original, self.applied);
        let event = windows::Win32::Foundation::HANDLE(self.cancel as *mut _);
        unsafe {
            let _ = SetEvent(event);
            let _ = windows::Win32::Foundation::CloseHandle(event);
        }
    }
}
fn begin_appbar_autohide(hwnd: HWND) -> Result<Option<AppbarLease>, String> {
    let original = appbar_state(hwnd);
    if original & ABS_AUTOHIDE != 0 {
        return Ok(None);
    }
    let applied = original | ABS_AUTOHIDE;
    let cancel = spawn_appbar_guardian(original, applied)?;
    set_appbar_state(hwnd, applied);
    if appbar_state(hwnd) & ABS_AUTOHIDE == 0 {
        unsafe {
            let _ = SetEvent(cancel);
            let _ = windows::Win32::Foundation::CloseHandle(cancel);
        }
        return Err(
            "Windows did not enable taskbar auto-hide; the taskbar was kept visible".into(),
        );
    }
    Ok(Some(AppbarLease {
        original,
        applied,
        cancel: cancel.0 as isize,
    }))
}
fn spawn_appbar_guardian(
    original: u32,
    applied: u32,
) -> Result<windows::Win32::Foundation::HANDLE, String> {
    use std::os::windows::process::CommandExt;
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = NEXT.fetch_add(1, Ordering::Relaxed);
    let prefix = format!(
        "Local\\Atelier.Dock.Appbar.{}.{}",
        std::process::id(),
        nonce
    );
    let ready_name = format!("{prefix}.ready");
    let cancel_name = format!("{prefix}.cancel");
    let wide = |name: &str| name.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let ready_w = wide(&ready_name);
    let cancel_w = wide(&cancel_name);
    let ready = unsafe { CreateEventW(None, true, false, PCWSTR(ready_w.as_ptr())) }
        .map_err(|e| e.to_string())?;
    let cancel = match unsafe { CreateEventW(None, true, false, PCWSTR(cancel_w.as_ptr())) } {
        Ok(h) => h,
        Err(e) => {
            let _ = unsafe { windows::Win32::Foundation::CloseHandle(ready) };
            return Err(e.to_string());
        }
    };
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(ready);
                let _ = windows::Win32::Foundation::CloseHandle(cancel);
            }
            return Err(e.to_string());
        }
    };
    let child = std::process::Command::new(exe)
        .args([
            "--dock-appbar-watchdog",
            &std::process::id().to_string(),
            &original.to_string(),
            &applied.to_string(),
            &ready_name,
            &cancel_name,
        ])
        .creation_flags(0x08000000)
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(ready);
                let _ = windows::Win32::Foundation::CloseHandle(cancel);
            }
            return Err(e.to_string());
        }
    };
    let ok =
        unsafe { WaitForSingleObject(ready, 2000) } == windows::Win32::Foundation::WAIT_OBJECT_0;
    unsafe {
        let _ = windows::Win32::Foundation::CloseHandle(ready);
    }
    if !ok {
        let _ = child.kill();
        let _ = child.wait();
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(cancel);
        }
        return Err("Taskbar recovery guardian did not start".into());
    }
    Ok(cancel)
}
fn appbar_guardian_args(args: &[String]) -> bool {
    if args.get(1).map(String::as_str) != Some("--dock-appbar-watchdog") {
        return false;
    }
    if let (Some(pid), Some(original), Some(applied), Some(ready_name), Some(cancel_name)) = (
        args.get(2).and_then(|v| v.parse::<u32>().ok()),
        args.get(3).and_then(|v| v.parse::<u32>().ok()),
        args.get(4).and_then(|v| v.parse::<u32>().ok()),
        args.get(5),
        args.get(6),
    ) {
        let wide = |name: &str| name.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let ready = wide(ready_name);
        let cancel = wide(cancel_name);
        if let (Ok(process), Ok(cancel), Ok(ready)) = unsafe {
            (
                OpenProcess(PROCESS_SYNCHRONIZE, false, pid),
                OpenEventW(SYNCHRONIZATION_SYNCHRONIZE, false, PCWSTR(cancel.as_ptr())),
                OpenEventW(EVENT_MODIFY_STATE, false, PCWSTR(ready.as_ptr())),
            )
        } {
            unsafe {
                let _ = SetEvent(ready);
                let _ = windows::Win32::Foundation::CloseHandle(ready);
            }
            let result = unsafe { WaitForMultipleObjects(&[cancel, process], false, INFINITE) };
            if result.0 == windows::Win32::Foundation::WAIT_OBJECT_0.0 + 1 {
                restore_appbar_if_owned(original, applied);
            }
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(cancel);
                let _ = windows::Win32::Foundation::CloseHandle(process);
            }
        }
    }
    true
}
fn show_taskbar(handle: isize) {
    let mut class = [0u16; 80];
    let n = unsafe { GetClassNameW(hwnd(handle), &mut class) };
    if n > 0
        && ["Shell_TrayWnd", "Shell_SecondaryTrayWnd"]
            .contains(&String::from_utf16_lossy(&class[..n as usize]).as_str())
    {
        let h = hwnd(handle);
        unsafe {
            let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
            if ex & WS_EX_LAYERED.0 as isize != 0 {
                SetWindowLongPtrW(h, GWL_EXSTYLE, ex & !(WS_EX_LAYERED.0 as isize));
            }
            let _ = ShowWindow(h, SW_SHOW);
            let _ = SetWindowPos(h, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
            let _ = windows::Win32::Graphics::Gdi::UpdateWindow(h);
        }
    }
}
pub fn dismiss_stuck_input_hosts() {
    unsafe {
        let mut w = GetTopWindow(HWND::default()).unwrap_or_default();
        while !w.0.is_null() {
            if IsWindowVisible(w).as_bool() {
                let mut buf = [0u16; 64];
                let len = GetClassNameW(w, &mut buf);
                let cls = String::from_utf16_lossy(&buf[..len as usize]);
                let mut tbuf = [0u16; 64];
                let tlen = GetWindowTextW(w, &mut tbuf);
                let title = String::from_utf16_lossy(&tbuf[..tlen as usize]);
                if cls == "Windows.UI.Core.CoreWindow" && title.contains("ввода") {
                    let mut rect = windows::Win32::Foundation::RECT::default();
                    let _ = GetWindowRect(w, &mut rect);
                    let screen_w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
                    let screen_h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
                    if (rect.right - rect.left) >= screen_w && (rect.bottom - rect.top) >= screen_h {
                        let _ = ShowWindow(w, SW_HIDE);
                    }
                }
            }
            w = GetWindow(w, GW_HWNDNEXT).unwrap_or_default();
        }
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
    if appbar_guardian_args(&args) {
        return true;
    }
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
    class: String,
    active: bool,
}
fn is_system_host(path: &str) -> bool {
    PathBuf::from(path).file_name().is_some_and(|name| {
        [
            "applicationframehost.exe",
            "shellexperiencehost.exe",
            "textinputhost.exe",
            "searchhost.exe",
            "startmenuexperiencehost.exe",
        ]
        .iter()
        .any(|skip| name.eq_ignore_ascii_case(skip))
    })
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
            if !is_system_host(&path) {
                let mut class = [0u16; 256];
                let class_len = GetClassNameW(handle, &mut class).max(0) as usize;
                list.push(Running {
                    class: String::from_utf16_lossy(&class[..class_len]),
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
#[derive(Default)]
struct ShortcutInfo {
    target: Option<String>,
    process_name: Option<String>,
    icon_path: Option<String>,
    icon_index: i32,
}
fn expand_environment(value: &str) -> String {
    let mut result = String::new();
    let mut parts = value.split('%');
    result.push_str(parts.next().unwrap_or(""));
    while let Some(name) = parts.next() {
        if let Some(rest) = parts.next() {
            if let Some(expanded) = std::env::var_os(name) {
                result.push_str(&expanded.to_string_lossy());
            } else {
                result.push('%');
                result.push_str(name);
                result.push('%');
            }
            result.push_str(rest);
        } else {
            result.push('%');
            result.push_str(name);
        }
    }
    result
}
fn shortcut_info(path: &str) -> Option<ShortcutInfo> {
    let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        link.cast::<IPersistFile>()
            .ok()?
            .Load(PCWSTR(wide.as_ptr()), STGM_READ)
            .ok()?;
        let mut info = ShortcutInfo::default();
        let mut buffer = [0u16; 32768];
        if link.GetPath(&mut buffer, std::ptr::null_mut(), 0).is_ok() {
            let n = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
            if n > 0 {
                info.target = Some(expand_environment(&String::from_utf16_lossy(&buffer[..n])));
            }
        }
        buffer.fill(0);
        if link
            .GetIconLocation(&mut buffer, &mut info.icon_index)
            .is_ok()
        {
            let n = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
            if n > 0 {
                info.icon_path = Some(expand_environment(&String::from_utf16_lossy(&buffer[..n])));
            }
        }
        buffer.fill(0);
        if link.GetArguments(&mut buffer).is_ok() {
            let n = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
            info.process_name = launcher_process_name(&String::from_utf16_lossy(&buffer[..n]));
        }
        if info.target.is_none()
            && info
                .icon_path
                .as_ref()
                .and_then(|p| {
                    PathBuf::from(p)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                })
                .is_some_and(|name| name.eq_ignore_ascii_case("explorer.exe"))
        {
            info.target = info.icon_path.clone();
        }
        Some(info)
    }
}
fn launcher_process_name(args: &str) -> Option<String> {
    let args: Vec<&str> = args.split_whitespace().collect();
    for pair in args.windows(2) {
        if pair[0].eq_ignore_ascii_case("--processStart") {
            let name = pair[1].trim_matches('"');
            if !name.is_empty()
                && !name.contains(['\\', '/', ':'])
                && name.to_ascii_lowercase().ends_with(".exe")
            {
                return Some(name.into());
            }
        }
    }
    None
}
fn pin_matches(target: &str, process_name: Option<&str>, running: &Running) -> bool {
    let explorer = PathBuf::from(target)
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("explorer.exe"));
    if explorer
        && !["CabinetWClass", "ExploreWClass"]
            .iter()
            .any(|class| class.eq_ignore_ascii_case(&running.class))
    {
        return false;
    }
    if key(&running.path) == key(target) {
        return true;
    }
    if PathBuf::from(target)
        .file_name()
        .is_some_and(|n| n.eq_ignore_ascii_case("steam.exe"))
        && PathBuf::from(&running.path)
            .file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case("steamwebhelper.exe"))
    {
        if let Some(parent) = PathBuf::from(target).parent() {
            return key(&running.path).starts_with(&(key(&parent.to_string_lossy()) + "\\"));
        }
    }
    let Some(process_name) = process_name else {
        return false;
    };
    let Some(parent) = PathBuf::from(target)
        .parent()
        .map(|p| key(&p.to_string_lossy()))
    else {
        return false;
    };
    let path = key(&running.path);
    let candidate = PathBuf::from(&running.path)
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case(process_name));
    candidate && path.starts_with(&(parent + "\\"))
}
fn shortcut_icon(info: &ShortcutInfo, state: &DockState) -> String {
    let Some(path) = info.icon_path.as_ref() else {
        return String::new();
    };
    let cache_key = format!("{path},{}", info.icon_index);
    if let Ok(cache) = state.icons.lock() {
        if let Some(icon) = cache.get(&cache_key) {
            return icon.clone();
        }
    }
    let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    let mut handle = HICON::default();
    let mut icon = if path.to_ascii_lowercase().ends_with(".ico") {
        file_icon(path, state)
    } else {
        String::new()
    };
    if icon.is_empty() {
        let result = unsafe {
            SHDefExtractIconW(
                PCWSTR(wide.as_ptr()),
                info.icon_index,
                0,
                Some(&mut handle),
                None,
                256,
            )
        };
        if result.is_ok() && !handle.0.is_null() {
            icon = super::hicon_to_bmp_base64(handle, 256).unwrap_or_default();
            let _ = unsafe { DestroyIcon(handle) };
        }
    }
    if !icon.is_empty() {
        if let Ok(mut cache) = state.icons.lock() {
            if cache.len() > 256 {
                cache.clear();
            }
            cache.insert(cache_key, icon.clone());
        }
    }
    icon
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
            SHGFI_SYSICONINDEX,
        )
    };
    let mut icon = String::new();
    if result != 0 {
        if let Ok(images) =
            unsafe { SHGetImageList::<windows::Win32::UI::Controls::IImageList>(SHIL_JUMBO as i32) }
        {
            if let Ok(handle) = unsafe {
                images.GetIcon(info.iIcon, windows::Win32::UI::Controls::ILD_TRANSPARENT.0)
            } {
                icon = super::hicon_to_bmp_base64(handle, 256).unwrap_or_default();
                let _ = unsafe { DestroyIcon(handle) };
            }
        }
    }
    if icon.is_empty() {
        let result = unsafe {
            SHGetFileInfoW(
                PCWSTR(wide.as_ptr()),
                windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
                Some(&mut info),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            )
        };
        if result != 0 && !info.hIcon.0.is_null() {
            icon = super::hicon_to_bmp_base64(info.hIcon, 128).unwrap_or_default();
            let _ = unsafe { DestroyIcon(info.hIcon) };
        }
    }
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
                if let Some(icon) = super::hicon_to_bmp_base64(HICON(result as *mut _), 128) {
                    return icon;
                }
            }
        }
        let icon = unsafe { GetClassLongPtrW(handle, GCLP_HICON) };
        if icon != 0 {
            if let Some(icon) = super::hicon_to_bmp_base64(HICON(icon as *mut _), 128) {
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
    pub hidden: bool,
}
#[derive(Serialize)]
pub struct DockSnapshot {
    pub config: DockConfig,
    pub items: Vec<DockItem>,
    pub monitors: Vec<String>,
    pub fullscreen_hidden: bool,
}
fn hidden_matches_running(hidden: &Pin, info: Option<&ShortcutInfo>, running: &Running) -> bool {
    if key(&hidden.path) == key(&running.path) {
        return true;
    }
    info.and_then(|i| {
        i.target
            .as_deref()
            .map(|target| pin_matches(target, i.process_name.as_deref(), running))
    })
    .unwrap_or(false)
}
fn items(state: &DockState, c: &DockConfig) -> Vec<DockItem> {
    let running = running_windows();
    let hidden_links: Vec<_> = c
        .hidden_apps
        .iter()
        .map(|h| {
            if h.path.to_ascii_lowercase().ends_with(".lnk") {
                shortcut_info(&h.path)
            } else {
                None
            }
        })
        .collect();
    let mut represented_hidden = HashSet::new();
    let mut used = HashSet::new();
    let mut result = Vec::new();
    for pin in &c.pinned {
        let info = if pin.path.to_ascii_lowercase().ends_with(".lnk") {
            shortcut_info(&pin.path)
        } else {
            None
        };
        let target = info
            .as_ref()
            .and_then(|i| i.target.clone())
            .unwrap_or_else(|| pin.path.clone());
        let matches: Vec<_> = running
            .iter()
            .filter(|r| {
                pin_matches(
                    &target,
                    info.as_ref().and_then(|i| i.process_name.as_deref()),
                    r,
                )
            })
            .collect();
        for r in &matches {
            used.insert(r.handle);
        }
        let hidden = c
            .hidden_apps
            .iter()
            .zip(&hidden_links)
            .fold(false, |any, (h, info)| {
                let matched = key(&h.path) == key(&pin.path)
                    || key(&h.path) == key(&target)
                    || matches
                        .iter()
                        .any(|r| hidden_matches_running(h, info.as_ref(), r));
                if matched {
                    represented_hidden.insert(key(&h.path));
                }
                any || matched
            });
        result.push(DockItem {
            id: pin.path.clone(),
            name: pin.name.clone(),
            icon: pin.icon.clone().unwrap_or_else(|| {
                let mut icon = info
                    .as_ref()
                    .map(|i| shortcut_icon(i, state))
                    .unwrap_or_default();
                if icon.is_empty() {
                    icon = file_icon(&pin.path, state);
                }
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
            hidden,
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
            let hidden = c
                .hidden_apps
                .iter()
                .zip(&hidden_links)
                .fold(false, |any, (h, info)| {
                    let matched = group
                        .iter()
                        .any(|r| hidden_matches_running(h, info.as_ref(), r));
                    if matched {
                        represented_hidden.insert(key(&h.path));
                    }
                    any || matched
                });
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
                hidden,
            });
        }
    }
    for hidden in &c.hidden_apps {
        if !represented_hidden.contains(&key(&hidden.path))
            && !result.iter().any(|item| key(&item.id) == key(&hidden.path))
            && !c
                .pinned
                .iter()
                .any(|pin| key(&pin.path) == key(&hidden.path))
        {
            result.push(DockItem {
                id: hidden.path.clone(),
                name: hidden.name.clone(),
                icon: file_icon(&hidden.path, state),
                pinned: false,
                running: false,
                active: false,
                windows: 0,
                hidden: true,
            });
        }
    }
    if let Some(favorites) = taskband_favorites() {
        merge_running_taskbar_order(&mut result, &favorites);
    }
    result
}
fn taskbar_shortcuts() -> Vec<PathBuf> {
    let Some(root) = std::env::var_os("APPDATA") else {
        return Vec::new();
    };
    let dir =
        PathBuf::from(root).join("Microsoft/Internet Explorer/Quick Launch/User Pinned/TaskBar");
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("lnk")))
        .collect();
    paths.sort();
    paths.truncate(64);
    paths
}
fn taskband_favorites() -> Option<Vec<u8>> {
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_BINARY};
    let mut size = 0u32;
    let path = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Taskband");
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            path,
            w!("Favorites"),
            RRF_RT_REG_BINARY,
            None,
            None,
            Some(&mut size),
        )
    };
    if status.0 != 0 || size == 0 || size > 1024 * 1024 {
        return None;
    }
    let mut data = vec![0u8; size as usize];
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            path,
            w!("Favorites"),
            RRF_RT_REG_BINARY,
            None,
            Some(data.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if status.0 != 0 || size as usize > data.len() {
        return None;
    }
    data.truncate(size as usize);
    Some(data)
}
fn taskband_name_offset(favorites: &[u8], name: &str) -> Option<usize> {
    if name.is_empty() {
        return None;
    }
    let needle: Vec<u8> = name
        .encode_utf16()
        .chain(Some(0))
        .flat_map(u16::to_le_bytes)
        .collect();
    favorites
        .windows(needle.len())
        .position(|part| part == needle)
}
fn merge_running_taskbar_order(items: &mut [DockItem], favorites: &[u8]) {
    let pinned_offsets: Option<Vec<_>> = items
        .iter()
        .filter(|item| item.pinned)
        .map(|item| {
            let path = PathBuf::from(&item.id);
            taskband_name_offset(favorites, path.file_name()?.to_str()?)
        })
        .collect();
    let Some(offsets) = pinned_offsets else {
        return;
    };
    if offsets.is_empty() || offsets.windows(2).any(|pair| pair[0] >= pair[1]) {
        return;
    }
    items.sort_by_key(|item| {
        let path = PathBuf::from(&item.id);
        let name = if item.pinned {
            path.file_name().and_then(|n| n.to_str())
        } else {
            path.file_stem()
                .and_then(|n| n.to_str())
                .filter(|name| name.chars().count() >= 3)
        };
        name.and_then(|name| taskband_name_offset(favorites, name))
            .unwrap_or(usize::MAX)
    });
}
fn order_pins_from_taskband(pins: &mut [Pin], favorites: &[u8]) -> bool {
    let mut positions = Vec::with_capacity(pins.len());
    for pin in pins.iter() {
        let Some(name) = PathBuf::from(&pin.path)
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_owned)
        else {
            return false;
        };
        let needle: Vec<u8> = name
            .encode_utf16()
            .chain(Some(0))
            .flat_map(u16::to_le_bytes)
            .collect();
        let Some(at) = favorites
            .windows(needle.len())
            .position(|part| part == needle)
        else {
            return false;
        };
        positions.push((key(&pin.path), at));
    }
    let mut unique = HashSet::new();
    if !positions.iter().all(|(_, offset)| unique.insert(*offset)) {
        return false;
    }
    let before: Vec<_> = pins.iter().map(|p| p.path.clone()).collect();
    pins.sort_by_key(|pin| {
        positions
            .iter()
            .find(|(path, _)| *path == key(&pin.path))
            .map(|(_, at)| *at)
            .unwrap_or(usize::MAX)
    });
    pins.iter().map(|p| &p.path).ne(before.iter())
}
fn repair_legacy_import(c: &mut DockConfig) -> bool {
    if !c.imported || c.pinned.is_empty() {
        return false;
    }
    let paths = taskbar_shortcuts();
    let stored: Vec<_> = c
        .pinned
        .iter()
        .map(|pin| PathBuf::from(&pin.path))
        .collect();
    if stored != paths {
        return false;
    }
    taskband_favorites()
        .is_some_and(|favorites| order_pins_from_taskband(&mut c.pinned, &favorites))
}
fn import_pins(c: &mut DockConfig) {
    if c.imported {
        return;
    }
    c.imported = true;
    for p in taskbar_shortcuts() {
        if c.pinned.len() >= 64 {
            break;
        }
        c.pinned.push(Pin {
            name: p.file_stem().unwrap_or_default().to_string_lossy().into(),
            path: p.to_string_lossy().into(),
            icon: None,
        });
    }
    if let Some(favorites) = taskband_favorites() {
        order_pins_from_taskband(&mut c.pinned, &favorites);
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
fn apply_bottom_gap(app: &tauri::AppHandle, c: &DockConfig) -> Result<(), String> {
    let state = app.state::<DockState>();
    if !c.enabled || c.maximized_bottom_gap == 0 || super::dock_test_mode() {
        state.remove_gap_appbar();
        return Ok(());
    }
    let previous = *state.gap_appbar.lock().map_err(|e| e.to_string())?;
    if previous
        .is_some_and(|(_, monitor, gap)| monitor == c.monitor && gap == c.maximized_bottom_gap)
    {
        return Ok(());
    }
    if app.get_webview_window("dock-gap").is_none() {
        tauri::WebviewWindowBuilder::new(
            app,
            "dock-gap",
            tauri::WebviewUrl::App("dock-gap.html".into()),
        )
        .decorations(false)
        .skip_taskbar(true)
        .visible(false)
        .focused(false)
        .inner_size(1., 1.)
        .build()
        .map_err(|e| e.to_string())?;
    }
    let window = app
        .get_webview_window("dock-gap")
        .ok_or("Work area window unavailable")?;
    let handle = window.hwnd().map_err(|e| e.to_string())?;
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let monitor = monitors
        .get(c.monitor)
        .or(monitors.first())
        .ok_or("No monitor available")?;
    let pos = monitor.position();
    let size = monitor.size();
    let height = ((c.maximized_bottom_gap as f64 * monitor.scale_factor()).round() as i32)
        .min(size.height as i32 - 1)
        .max(1);
    let mut data = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: HWND(handle.0 as _),
        uCallbackMessage: WM_APP + 42,
        uEdge: ABE_BOTTOM,
        rc: windows::Win32::Foundation::RECT {
            left: pos.x,
            top: pos.y + size.height as i32 - height,
            right: pos.x + size.width as i32,
            bottom: pos.y + size.height as i32,
        },
        ..Default::default()
    };
    let registered =
        previous.is_some_and(|(previous_handle, _, _)| previous_handle == handle.0 as isize);
    if !registered && unsafe { SHAppBarMessage(ABM_NEW, &mut data) } == 0 {
        return Err("Windows could not reserve space for the dock".into());
    }
    unsafe {
        SHAppBarMessage(ABM_QUERYPOS, &mut data);
    }
    data.rc.top = data.rc.bottom - height;
    if unsafe { SHAppBarMessage(ABM_SETPOS, &mut data) } == 0 {
        if !registered {
            unsafe {
                SHAppBarMessage(ABM_REMOVE, &mut data);
            }
        }
        return Err("Windows could not set the dock work area".into());
    }
    *state.gap_appbar.lock().map_err(|e| e.to_string())? =
        Some((handle.0 as isize, c.monitor, c.maximized_bottom_gap));
    Ok(())
}
fn fullscreen_on_dock_monitor(app: &tauri::AppHandle, c: &DockConfig) -> bool {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        MonitorFromPoint, MonitorFromWindow, MONITOR_DEFAULTTONEAREST,
    };
    if super::dock_test_mode() || !c.enabled {
        return false;
    }
    let foreground = unsafe { GetForegroundWindow() };
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(foreground, Some(&mut pid));
    }
    if foreground.0.is_null()
        || pid == 0
        || pid == std::process::id()
        || !unsafe { IsWindowVisible(foreground) }.as_bool()
        || unsafe { IsIconic(foreground) }.as_bool()
    {
        return false;
    }
    if !atelier_wallpaper::occlusion::is_fullscreen_window_active()
        && !atelier_wallpaper::occlusion::is_fullscreen_game_or_presentation()
    {
        return false;
    }
    let Ok(monitors) = app.available_monitors() else {
        return false;
    };
    let Some(monitor) = monitors.get(c.monitor).or(monitors.first()) else {
        return false;
    };
    let point = POINT {
        x: monitor
            .position()
            .x
            .saturating_add((monitor.size().width / 2) as i32),
        y: monitor
            .position()
            .y
            .saturating_add((monitor.size().height / 2) as i32),
    };
    unsafe {
        MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST)
            == MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST)
    }
}

fn dock_visibility_action(enabled: bool, fullscreen: bool, visible: bool) -> Option<bool> {
    let should_show = enabled && !fullscreen;
    (visible != should_show).then_some(should_show)
}
fn sync_dock_visibility(app: &tauri::AppHandle, c: &DockConfig) -> Result<(), String> {
    let Some(window) = app.get_webview_window("desktop-dock") else {
        return Ok(());
    };
    let fullscreen = fullscreen_on_dock_monitor(app, c);
    if let Some(should_show) = dock_visibility_action(
        c.enabled,
        fullscreen,
        window.is_visible().map_err(|e| e.to_string())?,
    ) {
        if should_show {
            window.show()
        } else {
            window.hide()
        }
        .map_err(|e| e.to_string())?;
    }
    let state = app.state::<DockState>();
    if state.fullscreen_hidden.swap(fullscreen, Ordering::SeqCst) != fullscreen {
        let _ = app.emit_to("desktop-dock", "dock-fullscreen-changed", fullscreen);
    }
    Ok(())
}

fn apply(app: &tauri::AppHandle, c: &DockConfig) -> Result<(), String> {
    if c.enabled {
        ensure_window(app)?;
        place(app, false)?;
    }
    app.state::<DockState>().maintain_native_taskbar()?;
    apply_bottom_gap(app, c)?;
    sync_dock_visibility(app, c)?;
    app.emit("dock-config-changed", c)
        .map_err(|e| e.to_string())?;
    Ok(())
}
fn workarea_changed(old: &DockConfig, new: &DockConfig) -> bool {
    old.enabled != new.enabled
        || (old.enabled
            && new.enabled
            && (old.maximized_bottom_gap != new.maximized_bottom_gap
                || (new.maximized_bottom_gap != 0 && old.monitor != new.monitor)))
}
pub fn start(app: &tauri::AppHandle) -> Result<(), String> {
    if !super::dock_test_mode() {
        let state = app.state::<DockState>();
        let _write = state.write.lock().map_err(|e| e.to_string())?;
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        let mut migrated = config.clone();
        if repair_legacy_import(&mut migrated) {
            if let Err(error) = persist(&migrated) {
                tracing::warn!(%error, "Could not save imported taskbar order");
            } else {
                *config = migrated;
            }
        }
    }
    let c = app
        .state::<DockState>()
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    apply(app, &c)?;
    let fullscreen_app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let state = fullscreen_app.state::<DockState>();
        if state.stopping.load(Ordering::Relaxed) {
            break;
        }
        let Ok(_write) = state.write.try_lock() else {
            continue;
        };
        let Ok(config) = state.config.lock().map(|c| c.clone()) else {
            continue;
        };
        if let Err(error) = sync_dock_visibility(&fullscreen_app, &config) {
            tracing::warn!(%error, "Could not update dock visibility for fullscreen");
        }
    });
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
            fullscreen_hidden: state.fullscreen_hidden.load(Ordering::Relaxed),
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
    config.preserve_managed(&old);
    if config.enabled {
        import_pins(&mut config)
    }
    *state.config.lock().map_err(|e| e.to_string())? = config.clone();
    if let Err(error) = apply(&app, &config).and_then(|_| persist(&config)) {
        *state.config.lock().map_err(|e| e.to_string())? = old.clone();
        let _ = apply(&app, &old);
        if workarea_changed(&old, &config) {
            super::desktop_wallpaper::repair_after_workarea_change(app.clone());
        }
        return Err(error);
    }
    if workarea_changed(&old, &config) {
        super::desktop_wallpaper::repair_after_workarea_change(app.clone());
    }
    Ok(config)
}
#[derive(Serialize)]
pub struct DockAudioSnapshot {
    pub audio: DockAudioConfig,
    pub dock_enabled: bool,
}
#[tauri::command]
pub fn get_dock_audio(
    window: tauri::Window,
    state: tauri::State<'_, DockState>,
) -> Result<DockAudioSnapshot, String> {
    if window.label() != "main" {
        return Err("Only Studio can configure dock audio".into());
    }
    let c = state.config.lock().map_err(|e| e.to_string())?;
    Ok(DockAudioSnapshot {
        audio: c.audio.clone(),
        dock_enabled: c.enabled,
    })
}
#[tauri::command]
pub fn set_dock_audio(
    window: tauri::Window,
    app: tauri::AppHandle,
    state: tauri::State<'_, DockState>,
    mut audio: DockAudioConfig,
) -> Result<DockAudioSnapshot, String> {
    if window.label() != "main" {
        return Err("Only Studio can configure dock audio".into());
    }
    audio.validate()?;
    let _write = state.write.lock().map_err(|e| e.to_string())?;
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    let mut next = config.clone();
    next.audio = audio;
    persist(&next)?;
    *config = next.clone();
    drop(config);
    app.emit("dock-config-changed", &next)
        .map_err(|e| e.to_string())?;
    Ok(DockAudioSnapshot {
        audio: next.audio,
        dock_enabled: next.enabled,
    })
}
#[tauri::command]
pub fn dock_audio_error(
    window: tauri::Window,
    app: tauri::AppHandle,
    message: String,
) -> Result<(), String> {
    if window.label() != "desktop-dock" {
        return Err("Only the dock can report audio errors".into());
    }
    app.emit_to(
        "main",
        "dock-audio-error",
        message.chars().take(300).collect::<String>(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reset_desktop_dock(app: tauri::AppHandle) -> Result<DockConfig, String> {
    let state = app.state::<DockState>();
    let _write = state.write.lock().map_err(|e| e.to_string())?;
    let old = state.config.lock().map_err(|e| e.to_string())?.clone();
    let c = DockConfig::default();
    persist(&c)?;
    *state.config.lock().map_err(|e| e.to_string())? = c.clone();
    apply(&app, &c)?;
    if old.enabled {
        super::desktop_wallpaper::repair_after_workarea_change(app.clone());
    }
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
            let info = pin
                .filter(|p| p.path.to_ascii_lowercase().ends_with(".lnk"))
                .and_then(|p| shortcut_info(&p.path));
            let target = info
                .as_ref()
                .and_then(|i| i.target.clone())
                .or_else(|| pin.map(|p| p.path.clone()))
                .unwrap_or_else(|| id.clone());
            let running = running_windows();
            let matches: Vec<_> = running
                .iter()
                .filter(|r| match pin {
                    Some(_) => pin_matches(
                        &target,
                        info.as_ref().and_then(|i| i.process_name.as_deref()),
                        r,
                    ),
                    None => key(&r.path) == key(&target),
                })
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
                let window = hwnd(r.handle);
                let can_maximize =
                    unsafe { GetWindowLongW(window, GWL_STYLE) } as u32 & WS_MAXIMIZEBOX.0 != 0;
                if c.maximize_on_open && can_maximize {
                    let _ = unsafe { ShowWindowAsync(window, SW_SHOWMAXIMIZED) };
                } else if unsafe { IsIconic(window) }.as_bool() {
                    let _ = unsafe { ShowWindowAsync(window, SW_RESTORE) };
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
                    if c.maximize_on_open {
                        SW_SHOWMAXIMIZED
                    } else {
                        SW_SHOWNORMAL
                    },
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
pub async fn dock_set_hidden(
    window: tauri::Window,
    app: tauri::AppHandle,
    id: String,
    hidden: bool,
) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Only Studio can change app visibility".into());
    }
    let state = app.state::<DockState>();
    let _write = state.write.lock().map_err(|e| e.to_string())?;
    let mut c = state.config.lock().map_err(|e| e.to_string())?.clone();
    if hidden {
        let known = c
            .pinned
            .iter()
            .find(|p| key(&p.path) == key(&id))
            .cloned()
            .or_else(|| {
                running_windows()
                    .into_iter()
                    .find(|r| key(&r.path) == key(&id))
                    .map(|r| Pin {
                        name: PathBuf::from(&r.path)
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into(),
                        path: r.path,
                        icon: None,
                    })
            })
            .ok_or("App no longer available")?;
        if !c.hidden_apps.iter().any(|p| key(&p.path) == key(&id)) {
            c.hidden_apps.push(known);
        }
    } else {
        let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
        let pin = c.pinned.iter().find(|p| key(&p.path) == key(&id));
        let pin_info = pin
            .filter(|p| p.path.to_ascii_lowercase().ends_with(".lnk"))
            .and_then(|p| shortcut_info(&p.path));
        let target = pin_info
            .as_ref()
            .and_then(|i| i.target.as_deref())
            .unwrap_or(&id);
        let running = running_windows().into_iter().find(|r| {
            pin_matches(
                target,
                pin_info.as_ref().and_then(|i| i.process_name.as_deref()),
                r,
            )
        });
        c.hidden_apps.retain(|p| {
            if key(&p.path) == key(&id) || key(&p.path) == key(target) {
                return false;
            }
            let info = if p.path.to_ascii_lowercase().ends_with(".lnk") {
                shortcut_info(&p.path)
            } else {
                None
            };
            !running
                .as_ref()
                .is_some_and(|r| hidden_matches_running(p, info.as_ref(), r))
        });
        if initialized {
            unsafe { CoUninitialize() }
        }
    }
    c.validate()?;
    persist(&c)?;
    *state.config.lock().map_err(|e| e.to_string())? = c.clone();
    app.emit("dock-config-changed", &c)
        .map_err(|e| e.to_string())?;
    Ok(())
}
fn native_flyout_candidate(
    action: &str,
    class: &str,
    name: &str,
    auto_id: &str,
    languages: &[String],
) -> bool {
    let class = class.to_lowercase();
    let name = name.to_lowercase();
    let auto_id = auto_id.to_lowercase();
    if action == "tray" {
        return class.ends_with("chevroniconview")
            || class.contains("chevron")
            || auto_id.contains("chevron")
            || auto_id.contains("overflow")
            || auto_id == "systemtraychevronbutton"
            || (auto_id == "systemtrayicon"
                && (name.contains("знач")
                    || name.contains("icon")
                    || name.contains("піктограм")
                    || name.contains("hidden")))
            || name.contains("chevron")
            || name.contains("hidden icon")
            || name.contains("overflow")
            || name.contains("приховані піктограми")
            || name.contains("скрытые значки")
            || name.contains("скрытых значков")
            || name.contains("показать скрытые");
    }
    if action == "language" {
        if class.contains("notify") || class.contains("chevron") {
            return false;
        }
        if auto_id.contains("inputindicator")
            || auto_id.contains("inputswitch")
            || auto_id.contains("language")
        {
            return true;
        }
        if name.contains("індикатор вводу")
            || name.contains("индикатор ввода")
            || name.contains("input indicator")
            || name.contains("input switch")
            || name.contains("методы ввода")
            || name.contains("методи вводу")
        {
            return true;
        }
        let language_match = languages.iter().any(|language| {
            let language = language.to_lowercase();
            !language.is_empty()
                && (name == language
                    || name.starts_with(&format!("{language} "))
                    || name.starts_with(&format!("{language} (")))
        });
        if language_match {
            return true;
        }
    }
    false
}

fn open_native_flyout(action: &str, languages: &[String]) -> Result<(), String> {
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationExpandCollapsePattern,
        IUIAutomationInvokePattern, IUIAutomationTogglePattern, TreeScope_Descendants,
        UIA_ExpandCollapsePatternId, UIA_InvokePatternId, UIA_TogglePatternId,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_B,
        VK_LWIN, VK_RETURN, VK_SPACE,
    };
    let taskbar = primary_taskbar().ok_or("Windows taskbar is unavailable")?;
    let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
    let result = (|| unsafe {
        let was_visible = IsWindowVisible(taskbar).as_bool();
        let original_ex_style = GetWindowLongPtrW(taskbar, GWL_EXSTYLE);
        if !was_visible {
            SetWindowLongPtrW(taskbar, GWL_EXSTYLE, original_ex_style | WS_EX_LAYERED.0 as isize);
            let _ = SetLayeredWindowAttributes(taskbar, COLORREF(0), 0, LWA_ALPHA);
            let _ = ShowWindow(taskbar, SW_SHOWNA);
            let _ = SetForegroundWindow(taskbar);
        }
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| e.to_string())?;
        let condition = automation
            .CreateTrueCondition()
            .map_err(|e| e.to_string())?;

        let mut candidates = Vec::new();
        for _ in 0..15 {
            if let Ok(root) = automation.ElementFromHandle(taskbar) {
                if let Ok(elements) = root.FindAll(TreeScope_Descendants, &condition) {
                    let len = elements.Length().unwrap_or(0).min(1500);
                    for index in 0..len {
                        if let Ok(element) = elements.GetElement(index) {
                            let class = element
                                .CurrentClassName()
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            let name = element
                                .CurrentName()
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            let auto_id = element
                                .CurrentAutomationId()
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            if native_flyout_candidate(action, &class, &name, &auto_id, languages) {
                                candidates.push(element);
                            }
                        }
                    }
                }
            }
            if !candidates.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let mut invoked = false;
        for element in &candidates {
            if let Ok(pattern) =
                element.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
            {
                if pattern.Invoke().is_ok() {
                    invoked = true;
                    break;
                }
            }
            if let Ok(pattern) =
                element.GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId)
            {
                if pattern.Toggle().is_ok() {
                    invoked = true;
                    break;
                }
            }
            if let Ok(pattern) = element
                .GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(UIA_ExpandCollapsePatternId)
            {
                if pattern.Expand().is_ok() {
                    invoked = true;
                    break;
                }
            }
        }
        if !was_visible {
            let action_str = action.to_string();
            let taskbar_val = taskbar.0 as isize;
            std::thread::spawn(move || {
                let tb = HWND(taskbar_val as *mut _);
                let flyout_classes: &[&str] = if action_str == "tray" {
                    &["NotifyIconOverflowWindow"]
                } else {
                    &[
                        "Xaml_WindowedPopupClass",
                        "Shell_InputSwitchTopLevelWindow",
                        "XamlExplorerHostIslandWindow_WASDK",
                    ]
                };
                let mut found_flyout = HWND::default();
                for _ in 0..40 {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    for &cls in flyout_classes {
                        let wide: Vec<u16> = cls.encode_utf16().chain(Some(0)).collect();
                        if let Ok(w) = FindWindowW(PCWSTR(wide.as_ptr()), None) {
                            if !w.0.is_null() && IsWindowVisible(w).as_bool() {
                                let mut rect = windows::Win32::Foundation::RECT::default();
                                let _ = GetWindowRect(w, &mut rect);
                                if rect.right > rect.left && rect.bottom > rect.top {
                                    found_flyout = w;
                                    break;
                                }
                            }
                        }
                    }
                    if !found_flyout.0.is_null() {
                        break;
                    }
                }
                if !found_flyout.0.is_null() {
                    while IsWindowVisible(found_flyout).as_bool() {
                        std::thread::sleep(std::time::Duration::from_millis(80));
                    }
                }
                let _ = ShowWindow(tb, SW_HIDE);
                SetWindowLongPtrW(tb, GWL_EXSTYLE, original_ex_style);
                dismiss_stuck_input_hosts();
            });
        }
        if invoked {
            return Ok(());
        }
        if action == "tray" {
            let key = |vk: VIRTUAL_KEY, up: bool| INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: vk,
                        dwFlags: if up {
                            KEYEVENTF_KEYUP
                        } else {
                            Default::default()
                        },
                        ..Default::default()
                    },
                },
            };
            let inputs = vec![
                key(VK_LWIN, false),
                key(VK_B, false),
                key(VK_B, true),
                key(VK_LWIN, true),
            ];
            if SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) == inputs.len() as u32 {
                std::thread::sleep(std::time::Duration::from_millis(40));
                let enter = vec![key(VK_RETURN, false), key(VK_RETURN, true)];
                if SendInput(&enter, std::mem::size_of::<INPUT>() as i32) == enter.len() as u32 {
                    return Ok(());
                }
            }
        }
        if action == "language" {
            let key = |vk: VIRTUAL_KEY, up: bool| INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: vk,
                        dwFlags: if up {
                            KEYEVENTF_KEYUP
                        } else {
                            Default::default()
                        },
                        ..Default::default()
                    },
                },
            };
            let inputs = vec![
                key(VK_LWIN, false),
                key(VK_SPACE, false),
                key(VK_SPACE, true),
                key(VK_LWIN, true),
            ];
            if SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) == inputs.len() as u32 {
                return Ok(());
            }
        }
        if candidates.is_empty() {
            return Err(format!(
                "Windows did not expose a unique {action} control while the taskbar is hidden"
            ));
        }
        Err(format!(
            "Windows does not allow the hidden {action} control to be opened"
        ))
    })();
    if initialized {
        unsafe { CoUninitialize() }
    }
    result
}

#[tauri::command]
pub async fn dock_open_system(window: tauri::Window, action: String) -> Result<(), String> {
    if window.label() != "desktop-dock" {
        return Err("Only the dock can open system menus".into());
    }
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_A,
        VK_LWIN, VK_N, VK_S,
    };
    let key = |vk: VIRTUAL_KEY, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                ..Default::default()
            },
        },
    };
    let shortcut = |vk: VIRTUAL_KEY| {
        vec![
            key(VK_LWIN, false),
            key(vk, false),
            key(vk, true),
            key(VK_LWIN, true),
        ]
    };
    if action == "tray" || action == "language" {
        let languages = dock_languages(window.clone())
            .map(|list| list.into_iter().map(|l| l.name).collect::<Vec<_>>())
            .unwrap_or_default();
        return tauri::async_runtime::spawn_blocking(move || open_native_flyout(&action, &languages))
            .await.map_err(|e| e.to_string())?;
    }
    let inputs = match action.as_str() {
        "start" => vec![key(VK_LWIN, false), key(VK_LWIN, true)],
        "search" => shortcut(VK_S),
        "network" | "volume" => shortcut(VK_A),
        "clock" => shortcut(VK_N),
        _ => return Err("Unknown system action".into()),
    };
    if unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) } != inputs.len() as u32 {
        let action_key = match action.as_str() {
            "search" => Some(VK_S),
            "network" | "volume" => Some(VK_A),
            "clock" => Some(VK_N),
            _ => None,
        };
        let mut release = Vec::new();
        if let Some(vk) = action_key {
            release.push(key(vk, true));
        }
        release.push(key(VK_LWIN, true));
        let _ = unsafe { SendInput(&release, std::mem::size_of::<INPUT>() as i32) };
        return Err("Windows could not open the system menu".into());
    }
    Ok(())
}

#[tauri::command]
pub fn dock_input_language(window: tauri::Window) -> Result<String, String> {
    if window.label() != "desktop-dock" {
        return Err("Only the dock can read the active input language".into());
    }
    use windows::Win32::Globalization::{GetLocaleInfoW, LOCALE_SABBREVLANGNAME};
    use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
    let state = window.state::<DockState>();
    let foreground = unsafe { GetForegroundWindow() };
    let mut pid = 0;
    let foreground_thread = unsafe { GetWindowThreadProcessId(foreground, Some(&mut pid)) };
    if pid != 0 && pid != std::process::id() {
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        let focused = if unsafe { GetGUIThreadInfo(foreground_thread, &mut info) }.is_ok()
            && !info.hwndFocus.0.is_null()
        {
            info.hwndFocus
        } else {
            foreground
        };
        *state.language_target.lock().map_err(|e| e.to_string())? = Some((focused.0 as isize, pid));
    }
    let target = *state.language_target.lock().map_err(|e| e.to_string())?;
    let thread = target
        .map(|(handle, _)| unsafe { GetWindowThreadProcessId(hwnd(handle), None) })
        .filter(|t| *t != 0)
        .unwrap_or(foreground_thread);
    let layout = unsafe { GetKeyboardLayout(thread) };
    let lang = (layout.0 as usize & 0xffff) as u32;
    let mut buffer = [0u16; 16];
    let n = unsafe { GetLocaleInfoW(lang, LOCALE_SABBREVLANGNAME, Some(&mut buffer)) };
    Ok(if n > 1 {
        String::from_utf16_lossy(&buffer[..n as usize - 1])
    } else {
        "—".into()
    })
}

#[derive(Serialize)]
pub struct DockLanguage {
    id: String,
    name: String,
}

#[tauri::command]
pub async fn dock_open_language(window: tauri::Window, x: f64, y: f64, width: f64, height: f64) -> Result<(), String> {
    if window.label() != "desktop-dock" || ![x, y, width, height].iter().all(|n| n.is_finite()) {
        return Err("Invalid language menu request".into());
    }
    let _ = dock_input_language(window.clone())?;
    let count = dock_languages(window.clone())?.len();
    if count == 0 { return Err("Windows did not report any keyboard layouts".into()); }
    let app = window.app_handle();
    let popup = match app.get_webview_window("dock-language") {
        Some(popup) => popup,
        None => tauri::WebviewWindowBuilder::new(app, "dock-language", tauri::WebviewUrl::App("dock-language.html".into()))
            .title("Input language").decorations(false).transparent(true).shadow(false)
            .skip_taskbar(true).always_on_top(true).resizable(false).visible(false)
            .inner_size(220., 180.).build().map_err(|e| e.to_string())?,
    };
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let origin = window.outer_position().map_err(|e| e.to_string())?;
    let monitor = window.current_monitor().map_err(|e| e.to_string())?.ok_or("No monitor available")?;
    let position = window.state::<DockState>().config.lock().map_err(|e| e.to_string())?.position.clone();
    let pw = (220.0 * scale).round() as i32;
    let ph = ((count.min(8) as f64 * 42.0 + 20.0) * scale).round() as i32;
    let ax = origin.x + (x * scale).round() as i32;
    let ay = origin.y + (y * scale).round() as i32;
    let aw = (width * scale).round() as i32;
    let ah = (height * scale).round() as i32;
    let gap = (8.0 * scale).round() as i32;
    let (mut px, mut py) = match position.as_str() {
        "top" => (ax + aw / 2 - pw / 2, ay + ah + gap),
        "left" => (ax + aw + gap, ay + ah / 2 - ph / 2),
        "right" => (ax - pw - gap, ay + ah / 2 - ph / 2),
        _ => (ax + aw / 2 - pw / 2, ay - ph - gap),
    };
    let mp = monitor.position(); let ms = monitor.size();
    px = px.clamp(mp.x, (mp.x + ms.width as i32 - pw).max(mp.x));
    py = py.clamp(mp.y, (mp.y + ms.height as i32 - ph).max(mp.y));
    popup.set_size(tauri::PhysicalSize::new(pw as u32, ph as u32)).map_err(|e| e.to_string())?;
    popup.set_position(tauri::PhysicalPosition::new(px, py)).map_err(|e| e.to_string())?;
    popup.show().map_err(|e| e.to_string())?;
    popup.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn dock_select_language(window: tauri::Window, id: String) -> Result<(), String> {
    if window.label() != "dock-language" { return Err("Only the language menu can select a layout".into()); }
    let languages = dock_languages(window.clone())?;
    if !languages.iter().any(|language| language.id == id) { return Err("Unknown keyboard layout".into()); }
    let layout = usize::from_str_radix(&id, 16).map_err(|e| e.to_string())?;
    let target = *window.state::<DockState>().language_target.lock().map_err(|e| e.to_string())?;
    let (handle, expected_pid) = target.ok_or("Select an application before changing its input language")?;
    let target = hwnd(handle);
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(target, Some(&mut pid));
        if pid != expected_pid || !IsWindow(target).as_bool() { return Err("The target application is no longer available".into()); }
        let root = GetAncestor(target, GA_ROOT);
        if !root.0.is_null() { let _ = SetForegroundWindow(root); }
        PostMessageW(target, WM_INPUTLANGCHANGEREQUEST, WPARAM(0), LPARAM(layout as isize)).map_err(|e| e.to_string())?;
    }
    window.hide().map_err(|e| e.to_string())?;
    Ok(())
}

#[allow(dead_code)]
fn show_language_menu(languages: Vec<DockLanguage>, target: Option<(isize, u32)>) -> Result<(), String> {
    use windows::Win32::Foundation::{POINT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    let target = target.ok_or("Select an application before changing its input language")?;
    if languages.is_empty() {
        return Err("Windows did not report any keyboard layouts".into());
    }
    unsafe {
        let mut point = POINT::default();
        GetCursorPos(&mut point).map_err(|e| e.to_string())?;
        let menu = CreatePopupMenu().map_err(|e| e.to_string())?;
        for (index, language) in languages.iter().enumerate() {
            let label: Vec<u16> = language.name.encode_utf16().chain(Some(0)).collect();
            if let Err(error) = AppendMenuW(menu, MF_STRING, index + 1, PCWSTR(label.as_ptr())) {
                let _ = DestroyMenu(menu);
                return Err(error.to_string());
            }
        }
        let instance = GetModuleHandleW(None).map_err(|e| e.to_string())?;
        let owner = match CreateWindowExW(
            WS_EX_TOOLWINDOW, w!("STATIC"), w!(""), WS_POPUP | WS_VISIBLE,
            -32000, -32000, 1, 1, None, None, instance, None,
        ) {
            Ok(owner) => owner,
            Err(error) => { let _ = DestroyMenu(menu); return Err(error.to_string()); }
        };
        let _ = SetForegroundWindow(owner);
        let selected = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_NONOTIFY | TPM_BOTTOMALIGN,
            point.x, point.y, 0, owner, None).0 as usize;
        let _ = DestroyMenu(menu);
        let _ = DestroyWindow(owner);
        let target_window = hwnd(target.0);
        let root = GetAncestor(target_window, GA_ROOT);
        if !root.0.is_null() {
            let _ = SetForegroundWindow(root);
        }
        if selected == 0 { return Ok(()); }
        let language = languages.get(selected - 1).ok_or("Invalid keyboard layout")?;
        let layout = usize::from_str_radix(&language.id, 16)
            .map_err(|e| e.to_string())?;
        let mut pid = 0;
        GetWindowThreadProcessId(target_window, Some(&mut pid));
        if pid != target.1 || !IsWindow(target_window).as_bool() {
            return Err("The target application is no longer available".into());
        }
        PostMessageW(target_window, WM_INPUTLANGCHANGEREQUEST, WPARAM(0), LPARAM(layout as isize))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn dock_languages(window: tauri::Window) -> Result<Vec<DockLanguage>, String> {
    if window.label() != "desktop-dock" && window.label() != "dock-language" {
        return Err("Only the dock can list input languages".into());
    }
    use windows::Win32::Globalization::{GetLocaleInfoW, LOCALE_SNATIVELANGNAME};
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyboardLayoutList, HKL};
    let count = unsafe { GetKeyboardLayoutList(None) }.clamp(0, 128) as usize;
    let mut layouts = vec![HKL::default(); count];
    let count = unsafe { GetKeyboardLayoutList(Some(&mut layouts)) }.max(0) as usize;
    layouts.truncate(count);
    Ok(layouts
        .into_iter()
        .map(|layout| {
            let lang = (layout.0 as usize & 0xffff) as u32;
            let mut buffer = [0u16; 128];
            let n = unsafe { GetLocaleInfoW(lang, LOCALE_SNATIVELANGNAME, Some(&mut buffer)) };
            DockLanguage {
                id: format!("{:x}", layout.0 as usize),
                name: if n > 1 {
                    String::from_utf16_lossy(&buffer[..n as usize - 1])
                } else {
                    format!("{lang:04X}")
                },
            }
        })
        .collect())
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
        if data.len() > 1_000_000
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
    fn fullscreen_hides_dock_and_restores_only_when_enabled() {
        assert_eq!(dock_visibility_action(true, true, true), Some(false));
        assert_eq!(dock_visibility_action(true, true, false), None);
        assert_eq!(dock_visibility_action(true, false, false), Some(true));
        assert_eq!(dock_visibility_action(true, false, true), None);
        assert_eq!(dock_visibility_action(false, false, false), None);
        assert_eq!(dock_visibility_action(false, false, true), Some(false));
    }

    #[test]
    fn workarea_repair_only_runs_for_changes_that_affect_the_desktop() {
        let old = DockConfig {
            enabled: true,
            maximized_bottom_gap: 40,
            ..DockConfig::default()
        };
        let mut next = old.clone();
        next.icon_size += 1;
        assert!(!workarea_changed(&old, &next));
        next.maximized_bottom_gap = 60;
        assert!(workarea_changed(&old, &next));
        next = old.clone();
        next.monitor += 1;
        assert!(workarea_changed(&old, &next));
        next = old.clone();
        next.enabled = false;
        assert!(workarea_changed(&old, &next));
        next = DockConfig::default();
        let mut disabled = next.clone();
        next.maximized_bottom_gap = 10;
        disabled.monitor = 1;
        assert!(!workarea_changed(&disabled, &next));
    }

    #[test]
    fn native_flyout_matching_never_selects_other_tray_icons() {
        let languages = vec!["українська".to_string(), "English".to_string()];
        assert!(native_flyout_candidate(
            "tray",
            "SystemTray.ChevronIconView",
            "",
            "",
            &languages
        ));
        assert!(native_flyout_candidate(
            "tray",
            "Button",
            "",
            "SystemTrayChevronButton",
            &languages
        ));
        assert!(!native_flyout_candidate(
            "tray",
            "SystemTray.NotifyIconView",
            "",
            "",
            &languages
        ));
        assert!(native_flyout_candidate(
            "language",
            "SystemTray.IconView",
            "Українська (Україна)",
            "",
            &languages
        ));
        assert!(native_flyout_candidate(
            "language",
            "Button",
            "",
            "SystemTrayInputIndicatorButton",
            &languages
        ));
        assert!(!native_flyout_candidate(
            "language",
            "SystemTray.NotifyIconView",
            "English",
            "",
            &languages
        ));
        assert!(!native_flyout_candidate(
            "language",
            "SystemTray.IconView",
            "Englishman",
            "",
            &languages
        ));
    }
    #[test]
    fn taskbar_order_is_used_only_when_all_shortcuts_can_be_found() {
        let pin = |name: &str| Pin {
            path: format!(r"C:\Pins\{name}.lnk"),
            name: name.into(),
            icon: None,
        };
        let mut pins = vec![pin("A"), pin("B"), pin("C")];
        let encoded = |name: &str| {
            name.encode_utf16()
                .chain(Some(0))
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<_>>()
        };
        let mut favorites = vec![17];
        for name in ["C.lnk", "A.lnk", "B.lnk"] {
            favorites.extend(encoded(name));
            favorites.push(31);
        }
        assert!(order_pins_from_taskband(&mut pins, &favorites));
        assert_eq!(
            pins.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["C", "A", "B"]
        );
        let before = pins.clone();
        assert!(!order_pins_from_taskband(&mut pins, &encoded("A.lnk")));
        assert_eq!(pins, before);
    }
    #[test]
    fn running_taskbar_items_fit_between_pins_without_changing_custom_pin_order() {
        let item = |name: &str, pinned: bool| DockItem {
            id: if pinned {
                format!(r"C:\Pins\{name}.lnk")
            } else {
                format!(r"C:\Apps\{name}.exe")
            },
            name: name.into(),
            icon: String::new(),
            pinned,
            running: true,
            active: false,
            windows: 1,
            hidden: false,
        };
        let encoded = |name: &str| {
            name.encode_utf16()
                .chain(Some(0))
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<_>>()
        };
        let mut favorites = Vec::new();
        for name in ["A.lnk", "ChatGPT", "B.lnk"] {
            favorites.extend(encoded(name));
            favorites.push(31);
        }
        let mut items = vec![
            item("A", true),
            item("B", true),
            item("ChatGPT", false),
            item("Other", false),
        ];
        merge_running_taskbar_order(&mut items, &favorites);
        assert_eq!(
            items.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
            ["A", "ChatGPT", "B", "Other"]
        );
        let mut custom = vec![item("B", true), item("A", true), item("ChatGPT", false)];
        merge_running_taskbar_order(&mut custom, &favorites);
        assert_eq!(
            custom.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
            ["B", "A", "ChatGPT"]
        );
    }
    #[test]
    fn launcher_shortcuts_match_only_their_app_and_explorer_matches_its_windows() {
        let running = |path: &str, class: &str| Running {
            handle: 1,
            pid: 1,
            path: path.into(),
            name: "Window".into(),
            class: class.into(),
            active: false,
        };
        let target = r"C:\Apps\Discord\Update.exe";
        assert_eq!(
            launcher_process_name("--processStart \"Discord.exe\"").as_deref(),
            Some("Discord.exe")
        );
        assert!(pin_matches(
            target,
            Some("Discord.exe"),
            &running(r"C:\Apps\Discord\app-1\Discord.exe", "Chrome_WidgetWin_1")
        ));
        assert!(!pin_matches(
            target,
            Some("Discord.exe"),
            &running(r"C:\Other\Discord.exe", "Chrome_WidgetWin_1")
        ));
        assert!(!pin_matches(
            target,
            Some("Discord.exe"),
            &running(r"C:\Apps\Discord\app-1\Other.exe", "Chrome_WidgetWin_1")
        ));
        let explorer = r"C:\Windows\explorer.exe";
        assert!(pin_matches(
            explorer,
            None,
            &running(explorer, "CabinetWClass")
        ));
        assert!(!pin_matches(
            explorer,
            None,
            &running(explorer, "OtherWindow")
        ));
    }
    #[test]
    fn windows_pinned_shortcut_icons_can_be_resolved_when_present() {
        let Some(root) = std::env::var_os("APPDATA") else {
            return;
        };
        let dir = PathBuf::from(root)
            .join("Microsoft/Internet Explorer/Quick Launch/User Pinned/TaskBar");
        let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
        let state = DockState::new();
        for name in ["Discord.lnk", "File Explorer.lnk"] {
            let path = dir.join(name);
            if !path.is_file() {
                continue;
            }
            let info = shortcut_info(&path.to_string_lossy()).expect("Could not load shortcut");
            if info
                .icon_path
                .as_ref()
                .is_some_and(|p| PathBuf::from(p).is_file())
            {
                assert!(
                    shortcut_icon(&info, &state).starts_with("data:image/png;base64,"),
                    "Could not load {name} icon"
                );
            }
        }
        if initialized {
            unsafe { CoUninitialize() }
        }
    }
    #[test]
    fn system_hosts_are_filtered_and_steam_helpers_match_steam() {
        assert!(is_system_host(r"C:\Windows\SystemApps\TextInputHost.exe"));
        assert!(is_system_host(r"C:\Windows\SearchHost.exe"));
        assert!(!is_system_host(r"C:\Apps\Discord.exe"));
        let r = Running {
            handle: 1,
            pid: 1,
            path: r"C:\Steam\bin\steamwebhelper.exe".into(),
            name: "Steam".into(),
            class: "Chrome_WidgetWin_1".into(),
            active: false,
        };
        assert!(pin_matches(r"C:\Steam\steam.exe", None, &r));
        assert!(!pin_matches(r"C:\Other\steam.exe", None, &r));
    }
    #[test]
    fn visibility_preferences_survive_dock_edits_and_legacy_config() {
        let mut old: DockConfig = serde_json::from_str(r#"{"enabled":true}"#).unwrap();
        assert!(old.show_start && old.show_search && old.launch_animation && old.maximize_on_open);
        old.hidden_apps.push(Pin {
            path: r"C:\Apps\one.exe".into(),
            name: "One".into(),
            icon: None,
        });
        let mut changed = DockConfig::default();
        changed.preserve_managed(&old);
        assert_eq!(changed.hidden_apps, old.hidden_apps);
        let reloaded: DockConfig =
            serde_json::from_slice(&serde_json::to_vec(&changed).unwrap()).unwrap();
        assert_eq!(reloaded.hidden_apps[0].name, "One");
        let r = Running {
            handle: 1,
            pid: 1,
            path: r"C:\Apps\one.exe".into(),
            name: "One".into(),
            class: "Class".into(),
            active: false,
        };
        assert!(hidden_matches_running(&reloaded.hidden_apps[0], None, &r));
    }
    #[test]
    fn appbar_restores_only_the_state_owned_by_the_dock() {
        assert!(should_restore_appbar(0, ABS_AUTOHIDE, ABS_AUTOHIDE));
        assert!(!should_restore_appbar(0, ABS_AUTOHIDE, 0));
        assert!(!should_restore_appbar(0, ABS_AUTOHIDE, ABS_AUTOHIDE | 2));
        assert!(!should_restore_appbar(
            ABS_AUTOHIDE,
            ABS_AUTOHIDE,
            ABS_AUTOHIDE
        ));
    }
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
    fn audio_glow_is_opt_in_validated_and_preserved_by_dock_edits() {
        let mut old: DockConfig = serde_json::from_str(r#"{"enabled":true}"#).unwrap();
        assert!(!old.audio.enabled);
        old.audio.enabled = true;
        old.audio.color = "#12abEF".into();
        old.audio.strength = 200;
        old.audio.shape = "both".into();
        old.audio.shape_strength = 999;
        old.audio.sensitivity = 999;
        old.audio.smoothing = 999;
        old.audio.frequency = "treble".into();
        old.validate().unwrap();
        assert_eq!(
            (
                old.audio.shape_strength,
                old.audio.sensitivity,
                old.audio.smoothing
            ),
            (100, 200, 95)
        );
        assert_eq!(old.audio.strength, 100);
        let mut edited = DockConfig::default();
        edited.preserve_managed(&old);
        assert_eq!(edited.audio, old.audio);
        let roundtrip: DockConfig =
            serde_json::from_slice(&serde_json::to_vec(&edited).unwrap()).unwrap();
        assert_eq!(roundtrip.audio, old.audio);
        edited.audio.color = "not-a-color".into();
        assert!(edited.validate().is_err());
        edited.audio.color = "#112233".into();
        edited.audio.shape = "invalid".into();
        assert!(edited.validate().is_err());
        edited.audio.shape = "off".into();
        edited.audio.frequency = "invalid".into();
        assert!(edited.validate().is_err());
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
        use base64::Engine;
        let png = base64::prelude::BASE64_STANDARD
            .decode(icon.split_once(',').unwrap().1)
            .unwrap();
        assert!(u32::from_be_bytes(png[16..20].try_into().unwrap()) >= 128);
        assert!(u32::from_be_bytes(png[20..24].try_into().unwrap()) >= 128);
    }
}
