use atelier_common::error::{AtelierError, Result};
use std::{
    io::{BufRead, Write},
    os::windows::process::CommandExt,
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
#[repr(C)]
struct MonitorInfo {
    size: u32,
    monitor: Rect,
    work: Rect,
    flags: u32,
}
#[link(name = "user32")]
extern "system" {
    fn FindWindowW(class: *const u16, title: *const u16) -> isize;
    fn GetWindowRect(hwnd: isize, rect: *mut Rect) -> i32;
    fn MonitorFromWindow(hwnd: isize, flags: u32) -> isize;
    fn GetMonitorInfoW(monitor: isize, info: *mut MonitorInfo) -> i32;
    fn SendMessageTimeoutW(
        hwnd: isize,
        message: u32,
        wparam: usize,
        lparam: isize,
        flags: u32,
        timeout: u32,
        result: *mut usize,
    ) -> isize;
}
#[link(name = "kernel32")]
extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
    fn WaitForSingleObject(handle: isize, timeout: u32) -> u32;
    fn CloseHandle(handle: isize) -> i32;
}
#[link(name = "advapi32")]
extern "system" {
    fn RegOpenKeyExW(
        root: isize,
        path: *const u16,
        options: u32,
        access: u32,
        key: *mut isize,
    ) -> i32;
    fn RegQueryValueExW(
        key: isize,
        name: *const u16,
        reserved: *mut u32,
        kind: *mut u32,
        data: *mut u8,
        size: *mut u32,
    ) -> i32;
    fn RegSetValueExW(
        key: isize,
        name: *const u16,
        reserved: u32,
        kind: u32,
        data: *const u8,
        size: u32,
    ) -> i32;
    fn RegDeleteValueW(key: isize, name: *const u16) -> i32;
    fn RegCloseKey(key: isize) -> i32;
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn error(s: impl Into<String>) -> AtelierError {
    AtelierError::Platform(s.into())
}
fn edge() -> Result<i32> {
    unsafe {
        let hwnd = FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null());
        let mut rect = Rect::default();
        let mut info = MonitorInfo {
            size: std::mem::size_of::<MonitorInfo>() as u32,
            monitor: Rect::default(),
            work: Rect::default(),
            flags: 0,
        };
        if hwnd == 0
            || GetWindowRect(hwnd, &mut rect) == 0
            || GetMonitorInfoW(MonitorFromWindow(hwnd, 2), &mut info) == 0
        {
            return Err(error("The Windows taskbar position could not be read"));
        }
        Ok(if rect.right - rect.left > rect.bottom - rect.top {
            if rect.top - info.monitor.top < (info.monitor.bottom - info.monitor.top) / 2 {
                1
            } else {
                3
            }
        } else if rect.left - info.monitor.left < (info.monitor.right - info.monitor.left) / 2 {
            0
        } else {
            2
        })
    }
}
fn move_to(request: i32) -> Result<()> {
    if edge()? == request {
        return Ok(());
    }
    unsafe {
        let hwnd = FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null());
        let mut reply = 0;
        if SendMessageTimeoutW(hwnd, 0x5CA, 6, request as isize, 2, 2000, &mut reply) == 0 {
            return Err(error(
                "Explorer did not respond to the taskbar position change",
            ));
        }
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if edge()? == request {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(error(
        "This Windows build did not accept the requested taskbar position",
    ))
}
fn registry(write: Option<Option<u32>>) -> Option<u32> {
    unsafe {
        let mut key = 0;
        if RegOpenKeyExW(
            0x80000001u32 as i32 as isize,
            wide("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced").as_ptr(),
            0,
            0x3,
            &mut key,
        ) != 0
        {
            return None;
        }
        let name = wide("TaskbarLocation");
        let mut value = 0u32;
        let mut size = 4;
        let mut kind = 0;
        let previous = if RegQueryValueExW(
            key,
            name.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            &mut value as *mut _ as *mut u8,
            &mut size,
        ) == 0
            && kind == 4
        {
            Some(value)
        } else {
            None
        };
        if let Some(saved) = write {
            if let Some(value) = saved {
                RegSetValueExW(key, name.as_ptr(), 0, 4, &value as *const _ as *const u8, 4);
            } else {
                RegDeleteValueW(key, name.as_ptr());
            }
        }
        RegCloseKey(key);
        previous
    }
}
struct Saved {
    edge: i32,
    registry: Option<u32>,
    guardian: Child,
}
#[derive(Default)]
pub struct NativePositionController {
    saved: Mutex<Option<Saved>>,
}
impl NativePositionController {
    pub fn apply(&self, request: i32) -> Result<()> {
        let mut lock = self.saved.lock().map_err(|e| error(e.to_string()))?;
        if request < 0 {
            return Self::restore(&mut lock);
        }
        if !(0..=3).contains(&request) {
            return Err(error("Invalid taskbar edge"));
        }
        if lock.is_none() {
            let original = edge()?;
            let saved_registry = registry(None);
            let mut guardian = Command::new(std::env::current_exe()?)
                .args([
                    "--native-taskbar-guardian",
                    &std::process::id().to_string(),
                    &original.to_string(),
                    &saved_registry
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "missing".into()),
                ])
                .creation_flags(0x08000000)
                .stdout(Stdio::piped())
                .spawn()?;
            let output = guardian
                .stdout
                .take()
                .ok_or_else(|| error("Taskbar recovery pipe is unavailable"))?;
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let mut line = String::new();
                let _ = std::io::BufReader::new(output).read_line(&mut line);
                let _ = tx.send(line);
            });
            if rx.recv_timeout(Duration::from_secs(2)).ok().as_deref() != Some("READY\n") {
                let _ = guardian.kill();
                let _ = guardian.wait();
                return Err(error(
                    "Taskbar recovery could not start; position was kept unchanged",
                ));
            }
            *lock = Some(Saved {
                edge: original,
                registry: saved_registry,
                guardian,
            });
        }
        if let Err(e) = move_to(request) {
            let _ = Self::restore(&mut lock);
            return Err(e);
        }
        Ok(())
    }
    fn restore(lock: &mut Option<Saved>) -> Result<()> {
        if let Some(saved) = lock.as_mut() {
            move_to(saved.edge)?;
            registry(Some(saved.registry));
            let _ = saved.guardian.kill();
            let _ = saved.guardian.wait();
        }
        *lock = None;
        Ok(())
    }
}
impl Drop for NativePositionController {
    fn drop(&mut self) {
        if let Ok(lock) = self.saved.get_mut() {
            let _ = Self::restore(lock);
        }
    }
}

pub fn guardian_args() -> bool {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--native-taskbar-guardian") {
        if let (Some(pid), Some(original)) = (
            args.get(2).and_then(|v| v.parse::<u32>().ok()),
            args.get(3).and_then(|v| v.parse::<i32>().ok()),
        ) {
            let saved = args.get(4).and_then(|v| v.parse::<u32>().ok());
            unsafe {
                let parent = OpenProcess(0x00100000, 0, pid);
                if parent != 0 {
                    println!("READY");
                    let _ = std::io::stdout().flush();
                    WaitForSingleObject(parent, u32::MAX);
                    CloseHandle(parent);
                    let _ = move_to(original);
                    registry(Some(saved));
                }
            }
        }
        return true;
    }
    #[cfg(debug_assertions)]
    if args.get(1).map(String::as_str) == Some("--native-position-probe") {
        let controller = NativePositionController::default();
        if args.get(2).map(String::as_str) == Some("--wait-for-crash") {
            controller.apply(0).unwrap();
            println!("READY FOR CRASH TEST");
            let _ = std::io::stdout().flush();
            std::thread::sleep(Duration::from_secs(30));
            return true;
        }
        for target in [0, 1, 2, 3, -1] {
            match controller.apply(target) {
                Ok(()) => println!("position {target}: verified {}", edge().unwrap_or(-1)),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1)
                }
            }
        }
        return true;
    }
    false
}
