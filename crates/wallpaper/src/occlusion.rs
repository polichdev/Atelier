use windows::core::w;
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::UI::Shell::{
    SHQueryUserNotificationState, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetClassNameW, GetClientRect, GetForegroundWindow, GetWindowThreadProcessId,
    IsIconic, IsWindowVisible,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcclusionState {
    Active,
    OccludedByFullscreenWindow,
    FullscreenGameOrPresentation,
    BatterySaverActive,
}

pub struct OcclusionDetector {
    pause_on_battery: bool,
}

impl OcclusionDetector {
    pub fn new(pause_on_battery: bool) -> Self {
        Self { pause_on_battery }
    }

    pub fn check_state(&self) -> OcclusionState {
        if is_fullscreen_game_or_presentation() {
            return OcclusionState::FullscreenGameOrPresentation;
        }

        if is_fullscreen_window_active() {
            return OcclusionState::OccludedByFullscreenWindow;
        }

        if self.pause_on_battery && is_running_on_battery() {
            return OcclusionState::BatterySaverActive;
        }

        OcclusionState::Active
    }

    pub fn should_pause(&self) -> bool {
        self.check_state() != OcclusionState::Active
    }
}

pub fn is_fullscreen_game_or_presentation() -> bool {
    match unsafe { SHQueryUserNotificationState() } {
        Ok(state) => state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE,
        Err(_) => false,
    }
}

pub fn is_running_on_battery() -> bool {
    let mut status = SYSTEM_POWER_STATUS::default();
    let res = unsafe { GetSystemPowerStatus(&mut status) };
    if res.is_ok() {
        status.ACLineStatus == 0
    } else {
        false
    }
}

pub fn is_fullscreen_window_active() -> bool {
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.0.is_null() {
        return false;
    }
    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(foreground, Some(&mut process_id));
    }
    if process_id == std::process::id()
        || !unsafe { IsWindowVisible(foreground) }.as_bool()
        || unsafe { IsIconic(foreground) }.as_bool()
    {
        return false;
    }

    let progman = unsafe { FindWindowW(w!("Progman"), None).unwrap_or_default() };
    let taskbar = unsafe { FindWindowW(w!("Shell_TrayWnd"), None).unwrap_or_default() };
    if foreground == progman || foreground == taskbar {
        return false;
    }

    let mut class_name = [0u16; 64];
    let len = unsafe { GetClassNameW(foreground, &mut class_name) };
    if len > 0 {
        let class_str = String::from_utf16_lossy(&class_name[..len as usize]);
        if class_str == "WorkerW" || class_str == "Progman" || class_str == "Shell_TrayWnd" {
            return false;
        }
    }

    let mut window_rect = RECT::default();
    if unsafe { GetClientRect(foreground, &mut window_rect) }.is_err() {
        return false;
    }
    let mut origin = POINT::default();
    if !unsafe { ClientToScreen(foreground, &mut origin) }.as_bool() {
        return false;
    }
    window_rect.right += origin.x;
    window_rect.bottom += origin.y;
    window_rect.left = origin.x;
    window_rect.top = origin.y;

    let monitor = unsafe { MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST) };
    if monitor.0.is_null() {
        return false;
    }

    let mut monitor_info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };

    if !unsafe { GetMonitorInfoW(monitor, &mut monitor_info) }.as_bool() {
        return false;
    }

    covers_monitor(window_rect, monitor_info.rcMonitor)
}

fn covers_monitor(window_rect: RECT, m_rect: RECT) -> bool {
    window_rect.left <= m_rect.left
        && window_rect.top <= m_rect.top
        && window_rect.right >= m_rect.right
        && window_rect.bottom >= m_rect.bottom
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maximized_client_with_taskbar_is_not_fullscreen() {
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        assert!(!covers_monitor(
            RECT {
                bottom: 1040,
                ..monitor
            },
            monitor
        ));
        assert!(covers_monitor(monitor, monitor));
        let left_monitor = RECT {
            left: -1920,
            right: 0,
            ..monitor
        };
        assert!(covers_monitor(left_monitor, left_monitor));
        assert!(!covers_monitor(monitor, left_monitor));
    }
}
