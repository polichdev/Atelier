use serde::{Deserialize, Serialize};
use std::{
    fs,
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
    UI::Shell::{DesktopWallpaper, IDesktopWallpaper},
};

#[derive(Clone, Copy, Serialize, Deserialize)]
struct Record {
    owner: u32,
    slideshow_tick: Option<u32>,
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
    with_desktop(|desktop| {
        use windows::Win32::UI::Shell::DESKTOP_SLIDESHOW_OPTIONS;
        if let Some(tick) = record.slideshow_tick {
            let mut opt = DESKTOP_SLIDESHOW_OPTIONS::default();
            let mut _current = 0u32;
            let _ = unsafe { desktop.GetSlideshowOptions(&mut opt, &mut _current) };
            let _ = unsafe { desktop.SetSlideshowOptions(opt, tick) };
        }
        Ok(())
    })?;
    fs::remove_file(path).map_err(|e| e.to_string())
}

pub fn suspend() -> Result<(), String> {
    let mut lease = LEASE.lock().map_err(|e| e.to_string())?;
    if SHUTTING_DOWN.load(Ordering::SeqCst) {
        return Ok(());
    }
    if lease.is_none() && !ACTIVE.load(Ordering::SeqCst) {
        let _ = recover();
    }
    let _ = with_desktop(|desktop| {
        use windows::Win32::UI::Shell::DESKTOP_SLIDESHOW_OPTIONS;
        let mut opt = DESKTOP_SLIDESHOW_OPTIONS::default();
        let mut tick = 0u32;
        if unsafe { desktop.GetSlideshowOptions(&mut opt, &mut tick) }.is_ok()
            && tick > 0
            && tick != u32::MAX
        {
            suspend_with(&mut lease, &journal()?, Some(tick), |new_tick| {
                if let Some(t) = new_tick {
                    unsafe { desktop.SetSlideshowOptions(opt, t) }.map_err(|e| e.to_string())?;
                }
                Ok(())
            })?;
        }
        Ok(())
    });
    ACTIVE.store(true, Ordering::SeqCst);
    Ok(())
}

pub fn refresh() -> Result<bool, String> {
    Ok(!SHUTTING_DOWN.load(Ordering::SeqCst) && ACTIVE.load(Ordering::SeqCst))
}

pub fn restore() -> Result<(), String> {
    let mut lease = LEASE.lock().map_err(|e| e.to_string())?;
    ACTIVE.store(false, Ordering::SeqCst);
    if lease.is_none() {
        return Ok(());
    }
    with_desktop(|desktop| {
        use windows::Win32::UI::Shell::DESKTOP_SLIDESHOW_OPTIONS;
        restore_with(&mut lease, &journal()?, |tick| {
            if let Some(t) = tick {
                let mut opt = DESKTOP_SLIDESHOW_OPTIONS::default();
                let mut _current = 0u32;
                let _ = unsafe { desktop.GetSlideshowOptions(&mut opt, &mut _current) };
                unsafe { desktop.SetSlideshowOptions(opt, t) }.map_err(|e| e.to_string())?;
            }
            Ok(())
        })
    })
}

fn suspend_with(
    lease: &mut Option<Record>,
    path: &Path,
    slideshow_tick: Option<u32>,
    set_tick: impl FnOnce(Option<u32>) -> Result<(), String>,
) -> Result<(), String> {
    if lease.is_none() {
        let record = Record {
            owner: std::process::id(),
            slideshow_tick,
        };
        fs::create_dir_all(path.parent().ok_or("Invalid background recovery path")?)
            .map_err(|e| e.to_string())?;
        use std::io::Write;
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
    if slideshow_tick.is_some() {
        set_tick(Some(u32::MAX)).map_err(|e| format!("Could not pause Windows slideshow: {e}"))?;
    }
    Ok(())
}

fn restore_with(
    lease: &mut Option<Record>,
    path: &Path,
    set_tick: impl FnOnce(Option<u32>) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(record) = *lease {
        set_tick(record.slideshow_tick)
            .map_err(|e| format!("Could not restore Windows slideshow: {e}"))?;
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
    fn failed_legacy_restore_keeps_journal_for_retry() {
        for original_tick in [Some(60000), None] {
            let path = std::env::temp_dir().join(format!(
                "atelier-background-{}-{:?}.json",
                std::process::id(),
                original_tick
            ));
            let mut lease = Some(Record { owner: std::process::id(), slideshow_tick: original_tick });
            std::fs::write(&path, b"legacy journal").unwrap();
            let system = Cell::new(Some(u32::MAX));
            let set = |tick| {
                system.set(tick);
                Ok(())
            };
            assert!(restore_with(&mut lease, &path, |_| Err("API failed".into())).is_err());
            assert!(path.exists());
            assert!(lease.is_some());
            restore_with(&mut lease, &path, set).unwrap();
            assert_eq!(system.get(), original_tick);
            assert!(lease.is_none());
            assert!(!path.exists());
        }
    }
}
