use atelier_common::config::{TaskbarAppearance, TaskbarStyle};
use atelier_common::error::{AtelierError, Result};
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::OnceLock;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HMODULE, HWND};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

static XAML_BRIDGE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/atelier_xaml.dll"));

type ApplyBrushFn = unsafe extern "system" fn(u32, u32, u32, *const u16) -> windows::core::HRESULT;
type RepairIconsFn = unsafe extern "system" fn(u32, *const u16) -> windows::core::HRESULT;
type ApplyAppearanceFn = unsafe extern "system" fn(
    u32,
    u32,
    u32,
    i32,
    u32,
    u32,
    u32,
    *const u16,
) -> windows::core::HRESULT;

struct XamlBridge {
    _module: HMODULE,
    apply: ApplyBrushFn,
    appearance: ApplyAppearanceFn,
    repair: RepairIconsFn,
    dll_path: Vec<u16>,
}

unsafe impl Send for XamlBridge {}
unsafe impl Sync for XamlBridge {}

static BRIDGE: OnceLock<Option<XamlBridge>> = OnceLock::new();

fn locate_or_extract_bridge_dll() -> std::io::Result<PathBuf> {
    let hash = XAML_BRIDGE_BYTES
        .iter()
        .fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ *b as u64).wrapping_mul(0x100000001b3)
        });
    let filename = format!("atelier_xaml_{hash:016x}.dll");
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let next_to_exe = parent.join(&filename);
            if next_to_exe.exists() {
                tracing::debug!(path = %next_to_exe.display(), "Found XAML bridge DLL next to executable");
                return Ok(next_to_exe);
            }
        }
    }

    let target_dir = std::env::var_os("ATELIER_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("APPDATA").map(|appdata| PathBuf::from(appdata).join("Atelier"))
        })
        .unwrap_or_else(|| std::env::temp_dir().join("Atelier"));

    tracing::debug!(dir = %target_dir.display(), "Preparing XAML bridge DLL extraction directory");

    std::fs::create_dir_all(&target_dir)?;
    let target_file = target_dir.join(&filename);

    let needs_write = match std::fs::metadata(&target_file) {
        Ok(meta) => {
            let stale = meta.len() != XAML_BRIDGE_BYTES.len() as u64;
            if stale {
                tracing::debug!(
                    path = %target_file.display(),
                    on_disk = meta.len(),
                    embedded = XAML_BRIDGE_BYTES.len(),
                    "Cached XAML bridge DLL is stale; will overwrite"
                );
            }
            stale
        }
        Err(_) => {
            tracing::debug!(path = %target_file.display(), "XAML bridge DLL not found on disk; extracting");
            true
        }
    };

    if needs_write {
        if let Err(err) = std::fs::write(&target_file, XAML_BRIDGE_BYTES) {
            tracing::error!(path = %target_file.display(), error = %err, "Failed to extract embedded XAML bridge DLL");
            return Err(err);
        }
        tracing::debug!(path = %target_file.display(), "Extracted embedded XAML bridge DLL");
    }

    Ok(target_file)
}

fn initialize_bridge() -> Option<XamlBridge> {
    let dll_path = match locate_or_extract_bridge_dll() {
        Ok(p) => p,
        Err(err) => {
            tracing::error!(error = %err, "Failed to locate or extract XAML bridge DLL");
            return None;
        }
    };
    tracing::debug!(path = %dll_path.display(), "Loading XAML bridge DLL");
    let mut wide_path: Vec<u16> = dll_path.as_os_str().encode_wide().collect();
    wide_path.push(0);

    let module = match unsafe { LoadLibraryW(PCWSTR(wide_path.as_ptr())) } {
        Ok(m) => m,
        Err(err) => {
            tracing::error!(path = %dll_path.display(), error = %err, "LoadLibraryW failed for XAML bridge DLL");
            return None;
        }
    };
    let proc = match unsafe { GetProcAddress(module, windows::core::s!("AtelierApplyBrush")) } {
        Some(p) => p,
        None => {
            tracing::error!("AtelierApplyBrush export not found in XAML bridge DLL");
            return None;
        }
    };
    let apply =
        unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, ApplyBrushFn>(proc) };
    let proc = unsafe { GetProcAddress(module, windows::core::s!("AtelierApplyAppearance")) }?;
    let appearance = unsafe {
        std::mem::transmute::<unsafe extern "system" fn() -> isize, ApplyAppearanceFn>(proc)
    };
    let repair = unsafe { GetProcAddress(module, windows::core::s!("AtelierRepairIcons")) }?;
    let repair = unsafe {
        std::mem::transmute::<unsafe extern "system" fn() -> isize, RepairIconsFn>(repair)
    };

    tracing::debug!("XAML bridge DLL loaded and AtelierApplyBrush resolved successfully");
    Some(XamlBridge {
        _module: module,
        apply,
        appearance,
        repair,
        dll_path: wide_path,
    })
}

fn get_bridge() -> Option<&'static XamlBridge> {
    BRIDGE.get_or_init(initialize_bridge).as_ref()
}

pub fn is_supported() -> bool {
    get_bridge().is_some()
}
pub fn repair_native_icons(hwnd: HWND) -> Result<()> {
    let bridge = get_bridge().ok_or_else(|| {
        AtelierError::Platform("The native taskbar XAML bridge could not be loaded".into())
    })?;
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    unsafe { (bridge.repair)(pid, bridge.dll_path.as_ptr()) }
        .ok()
        .map_err(|e| {
            AtelierError::Platform(format!("Could not restore native icon rendering: {e}"))
        })
}

pub fn apply_xaml_brush(hwnd: HWND, style: TaskbarStyle) -> Result<()> {
    let bridge = match get_bridge() {
        Some(b) => b,
        None => {
            return Err(AtelierError::Platform(
                "Atelier XAML bridge library failed to load".to_string(),
            ))
        }
    };

    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 {
        return Err(AtelierError::Platform(
            "Could not determine Explorer process ID for taskbar window".to_string(),
        ));
    }

    let (material, color) = match style {
        TaskbarStyle::Default => (0u32, 0u32),
        TaskbarStyle::Clear => (1u32, 0u32),
        TaskbarStyle::Blur => (2u32, 0u32),
        TaskbarStyle::Acrylic { tint } => (3u32, tint),
        TaskbarStyle::Mica => (0u32, 0u32),
    };

    tracing::debug!(
        pid,
        material,
        color,
        "Calling AtelierApplyBrush via XAML bridge"
    );
    let result = unsafe { (bridge.apply)(pid, material, color, bridge.dll_path.as_ptr()) };
    if let Err(err) = result.ok() {
        tracing::error!(pid, material, color, error = %err, "AtelierApplyBrush returned failure HRESULT");
        return Err(AtelierError::Platform(format!(
            "AtelierApplyBrush failed: {err}"
        )));
    }

    tracing::debug!(pid, material, "AtelierApplyBrush succeeded");
    Ok(())
}

pub fn apply_native_appearance(hwnd: HWND, appearance: &TaskbarAppearance) -> Result<()> {
    let bridge = get_bridge().ok_or_else(|| {
        AtelierError::Platform("The native taskbar XAML bridge could not be loaded".into())
    })?;
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 {
        return Err(AtelierError::Platform(
            "The Windows taskbar process is unavailable".into(),
        ));
    }
    let result = unsafe {
        (bridge.appearance)(
            pid,
            appearance.icon_size,
            appearance.button_width,
            appearance.button_radius,
            appearance.inactive_opacity,
            appearance.running_indicator,
            appearance.hover_scale,
            bridge.dll_path.as_ptr(),
        )
    };
    result.ok().map_err(|e| {
        AtelierError::Platform(format!(
            "Could not style the native taskbar buttons on this Windows build: {e}"
        ))
    })?;
    Ok(())
}
