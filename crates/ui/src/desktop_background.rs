use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use windows::Win32::{
    Foundation::{CloseHandle, RPC_E_CHANGED_MODE},
    System::{
        Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
        },
        Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    },
    UI::Shell::{DesktopWallpaper, IDesktopWallpaper, DSS_ENABLED},
};

#[derive(Clone, Copy, Serialize, Deserialize)]
struct Record {
    owner: u32,
    enabled: bool,
}
static LEASE: Mutex<Option<Record>> = Mutex::new(None);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

fn journal() -> Result<PathBuf, String> {
    super::settings_path()
        .map(|p| p.with_file_name("desktop-background-restore.json"))
        .ok_or("Atelier data directory is unavailable".into())
}
struct ComScope(bool);
impl Drop for ComScope {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                CoUninitialize();
            }
        }
    }
}
fn with_desktop<T>(f: impl FnOnce(&IDesktopWallpaper) -> Result<T, String>) -> Result<T, String> {
    let status = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if status.is_err() && status != RPC_E_CHANGED_MODE {
        return Err(status.to_string());
    }
    let _com = ComScope(status.is_ok());
    let desktop: IDesktopWallpaper =
        unsafe { CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL) }
            .map_err(|e| e.to_string())?;
    f(&desktop)
}
fn process_alive(pid: u32) -> bool {
    unsafe {
        match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(handle) => {
                let mut code = 0;
                let active = GetExitCodeProcess(handle, &mut code).is_err() || code == 259;
                let _ = CloseHandle(handle);
                active
            }
            Err(e) => e.code().0 as u32 != 0x80070057,
        }
    }
}

pub fn recover() -> Result<(), String> {
    let path = journal()?;
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.to_string()),
    };
    let record: Record = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if record.owner != std::process::id() && process_alive(record.owner) {
        return Err(
            "Another Atelier instance is controlling the desktop background. Close it first."
                .into(),
        );
    }
    with_desktop(|desktop| unsafe { desktop.Enable(record.enabled) }.map_err(|e| e.to_string()))?;
    fs::remove_file(path).map_err(|e| e.to_string())
}

pub fn suspend() -> Result<(), String> {
    set_suspended(true).map(|_| ())
}

pub fn refresh() -> Result<bool, String> {
    set_suspended(false)
}

fn set_suspended(activate: bool) -> Result<bool, String> {
    let mut lease = LEASE.lock().map_err(|e| e.to_string())?;
    if SHUTTING_DOWN.load(Ordering::SeqCst) {
        return Ok(false);
    }
    if activate {
        ACTIVE.store(true, Ordering::SeqCst);
    }
    if !ACTIVE.load(Ordering::SeqCst) {
        return Ok(false);
    }
    if lease.is_none() {
        recover()?;
    }
    with_desktop(|desktop| {
        let enabled =
            unsafe { desktop.GetStatus() }.map_err(|e| e.to_string())?.0 & DSS_ENABLED.0 != 0;
        suspend_with(&mut lease, &journal()?, enabled, |enabled| {
            unsafe { desktop.Enable(enabled) }.map_err(|e| e.to_string())
        })?;
        Ok(true)
    })
}

fn suspend_with(
    lease: &mut Option<Record>,
    path: &Path,
    enabled: bool,
    set_enabled: impl FnOnce(bool) -> Result<(), String>,
) -> Result<(), String> {
    if lease.is_none() {
        let record = Record {
            owner: std::process::id(),
            enabled,
        };
        fs::create_dir_all(path.parent().ok_or("Invalid background recovery path")?)
            .map_err(|e| e.to_string())?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec(&record).map_err(|e| e.to_string())?)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        *lease = Some(record);
    }
    if enabled {
        set_enabled(false).map_err(|e| format!("Could not suspend the Windows background: {e}"))?;
    }
    Ok(())
}

pub fn restore() -> Result<(), String> {
    let mut lease = LEASE.lock().map_err(|e| e.to_string())?;
    ACTIVE.store(false, Ordering::SeqCst);
    if lease.is_none() {
        return Ok(());
    }
    with_desktop(|desktop| {
        restore_with(&mut lease, &journal()?, |enabled| {
            unsafe { desktop.Enable(enabled) }.map_err(|e| e.to_string())
        })
    })
}

fn restore_with(
    lease: &mut Option<Record>,
    path: &Path,
    set_enabled: impl FnOnce(bool) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(record) = *lease {
        set_enabled(record.enabled)
            .map_err(|e| format!("Could not restore the Windows background: {e}"))?;
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
        *lease = None;
    }
    Ok(())
}

pub fn shutdown() -> Result<(), String> {
    SHUTTING_DOWN.store(true, Ordering::SeqCst);
    restore()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    #[test]
    fn repeated_refresh_preserves_original_background_and_failed_restore_can_retry() {
        for originally_enabled in [true, false] {
            let path = std::env::temp_dir().join(format!(
                "atelier-background-{}-{originally_enabled}.json",
                std::process::id()
            ));
            let mut lease = None;
            let system = Cell::new(originally_enabled);
            let set = |enabled| {
                system.set(enabled);
                Ok(())
            };
            suspend_with(&mut lease, &path, system.get(), set).unwrap();
            assert!(!system.get());
            suspend_with(&mut lease, &path, system.get(), set).unwrap();
            assert_eq!(lease.unwrap().enabled, originally_enabled);
            system.set(true);
            suspend_with(&mut lease, &path, system.get(), set).unwrap();
            assert!(!system.get());
            assert!(restore_with(&mut lease, &path, |_| Err("API failed".into())).is_err());
            assert!(path.exists());
            assert!(lease.is_some());
            restore_with(&mut lease, &path, set).unwrap();
            assert_eq!(system.get(), originally_enabled);
            assert!(lease.is_none());
            assert!(!path.exists());
        }
    }
}
