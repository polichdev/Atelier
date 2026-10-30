use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use windows::{
    core::{Interface, PWSTR},
    Win32::{
        Foundation::CloseHandle,
        Media::Audio::{
            eMultimedia, eRender, AudioSessionStateActive, Endpoints::IAudioMeterInformation,
            IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator, MMDeviceEnumerator,
        },
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
            },
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
        UI::WindowsAndMessaging::{
            GetClassNameW, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
            IsWindowVisible, IsZoomed,
        },
    },
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppRule {
    pub executable: String,
    pub action: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PerformanceSettings {
    pub fps: u32,
    pub resolution: f64,
    pub antialiasing: bool,
    pub effects: bool,
    pub focused: String,
    pub maximized: String,
    pub fullscreen: Option<String>,
    pub battery: Option<String>,
    pub other_audio: String,
    pub rules: Vec<AppRule>,
}
impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            fps: 30,
            resolution: 1.,
            antialiasing: false,
            effects: true,
            focused: "continue".into(),
            maximized: "continue".into(),
            fullscreen: None,
            battery: None,
            other_audio: "continue".into(),
            rules: Vec::new(),
        }
    }
}
fn action(s: &str) -> bool {
    matches!(s, "continue" | "pause" | "mute")
}
impl PerformanceSettings {
    pub fn normalize(&mut self) {
        self.fps = self.fps.clamp(10, 144);
        self.resolution = if self.resolution.is_finite() {
            self.resolution.clamp(0.25, 1.)
        } else {
            1.
        };
        for s in [
            &mut self.focused,
            &mut self.maximized,
            &mut self.other_audio,
        ] {
            if !action(s) {
                *s = "continue".into()
            }
        }
        for s in [&mut self.fullscreen, &mut self.battery] {
            if s.as_ref().is_some_and(|s| !action(s)) {
                *s = None
            }
        }
        self.rules.truncate(64);
        for r in &mut self.rules {
            r.executable = r.executable.trim().to_lowercase()
        }
        self.rules.retain(|r| {
            r.executable.len() <= 128
                && r.executable.ends_with(".exe")
                && !r.executable.contains(['\\', '/', ':', '\0', '*', '?'])
                && action(&r.action)
        });
    }
}
#[derive(Default)]
pub struct Environment {
    pub focused: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub battery: bool,
    pub other_audio: bool,
    pub executable: String,
}
#[derive(Serialize, Debug, PartialEq)]
pub struct PlaybackState {
    pub paused: bool,
    pub muted: bool,
    pub reason: Option<String>,
}
pub fn evaluate(
    settings: &PerformanceSettings,
    environment: &Environment,
    manual: bool,
    fullscreen_fallback: bool,
    battery_fallback: bool,
) -> PlaybackState {
    let mut paused = manual;
    let mut muted = false;
    let mut reason = manual.then(|| "paused".to_owned());
    let rule = settings
        .rules
        .iter()
        .find(|r| r.executable.eq_ignore_ascii_case(&environment.executable));
    let window_action = if let Some(rule) = rule {
        Some((rule.action.as_str(), "paused-application"))
    } else if environment.fullscreen {
        Some((
            settings
                .fullscreen
                .as_deref()
                .unwrap_or(if fullscreen_fallback {
                    "pause"
                } else {
                    "continue"
                }),
            "paused-fullscreen",
        ))
    } else if environment.maximized {
        let action = if fullscreen_fallback
            && matches!(settings.fullscreen.as_deref(), None | Some("pause"))
            && settings.maximized == "continue"
        {
            "pause"
        } else {
            settings.maximized.as_str()
        };
        Some((action, "paused-maximized"))
    } else if environment.focused {
        Some((settings.focused.as_str(), "paused-application"))
    } else {
        None
    };
    let battery = environment.battery.then_some((
        settings.battery.as_deref().unwrap_or(if battery_fallback {
            "pause"
        } else {
            "continue"
        }),
        "paused-battery",
    ));
    let audio = environment
        .other_audio
        .then_some((settings.other_audio.as_str(), "paused-audio"));
    for (action, label) in window_action.into_iter().chain(battery).chain(audio) {
        if action == "pause" {
            paused = true;
            if reason.is_none() {
                reason = Some(label.into())
            }
        }
        if action == "mute" {
            muted = true
        }
    }
    PlaybackState {
        paused,
        muted,
        reason,
    }
}
pub fn environment(settings: &PerformanceSettings) -> Environment {
    let mut result = Environment {
        battery: atelier_wallpaper::occlusion::is_running_on_battery(),
        fullscreen: atelier_wallpaper::occlusion::is_fullscreen_window_active(),
        ..Default::default()
    };
    unsafe {
        let window = GetForegroundWindow();
        let mut pid = 0;
        GetWindowThreadProcessId(window, Some(&mut pid));
        let mut class = [0u16; 128];
        let length = GetClassNameW(window, &mut class);
        let class = String::from_utf16_lossy(&class[..length.max(0) as usize]);
        result.focused = pid != 0
            && pid != std::process::id()
            && IsWindowVisible(window).as_bool()
            && !IsIconic(window).as_bool()
            && !matches!(
                class.as_str(),
                "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
            );
        if result.focused {
            result.maximized = IsZoomed(window).as_bool();
            if !settings.rules.is_empty() {
                if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                    let mut buffer = [0u16; 1024];
                    let mut len = buffer.len() as u32;
                    if QueryFullProcessImageNameW(
                        handle,
                        PROCESS_NAME_FORMAT(0),
                        PWSTR(buffer.as_mut_ptr()),
                        &mut len,
                    )
                    .is_ok()
                    {
                        result.executable = String::from_utf16_lossy(&buffer[..len as usize])
                            .rsplit('\\')
                            .next()
                            .unwrap_or("")
                            .to_lowercase()
                    }
                    let _ = CloseHandle(handle);
                }
            }
        }
    }
    if settings.other_audio != "continue" {
        static LAST_AUDIO: Mutex<Option<Instant>> = Mutex::new(None);
        if let Ok(mut last) = LAST_AUDIO.lock() {
            if other_audio_playing() {
                *last = Some(Instant::now())
            }
            result.other_audio = last.is_some_and(|t| t.elapsed() < Duration::from_secs(3));
        }
    }
    result
}
fn other_audio_playing() -> bool {
    unsafe {
        let initialized = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
        let detected = (|| -> windows::core::Result<bool> {
            let mut parents = HashMap::new();
            if let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
                let mut entry = PROCESSENTRY32W {
                    dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                    ..Default::default()
                };
                if Process32FirstW(snapshot, &mut entry).is_ok() {
                    loop {
                        parents.insert(entry.th32ProcessID, entry.th32ParentProcessID);
                        if Process32NextW(snapshot, &mut entry).is_err() {
                            break;
                        }
                    }
                }
                let _ = CloseHandle(snapshot);
            }
            let own = |mut pid: u32| {
                for _ in 0..32 {
                    if pid == std::process::id() {
                        return true;
                    }
                    let Some(parent) = parents.get(&pid) else {
                        return false;
                    };
                    if *parent == pid {
                        return false;
                    }
                    pid = *parent;
                }
                false
            };
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
            let sessions = manager.GetSessionEnumerator()?;
            for i in 0..sessions.GetCount()?.min(256) {
                let session = sessions.GetSession(i)?;
                if session.GetState()? != AudioSessionStateActive {
                    continue;
                }
                let control: IAudioSessionControl2 = session.cast()?;
                if control.IsSystemSoundsSession().0 == 0 || own(control.GetProcessId()?) {
                    continue;
                }
                if let Ok(meter) = session.cast::<IAudioMeterInformation>() {
                    if meter.GetPeakValue().unwrap_or(0.) > 0.0005 {
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        })()
        .unwrap_or(false);
        if initialized {
            CoUninitialize()
        }
        detected
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wallpaper_fullscreen_pause_includes_maximized_windows_with_workarea_gaps() {
        let mut settings = PerformanceSettings::default();
        let env = Environment {
            focused: true,
            maximized: true,
            fullscreen: false,
            ..Default::default()
        };
        let paused = evaluate(&settings, &env, false, true, false);
        assert!(paused.paused);
        assert_eq!(paused.reason.as_deref(), Some("paused-maximized"));
        assert!(!evaluate(&settings, &env, false, false, false).paused);
        settings.fullscreen = Some("continue".into());
        assert!(!evaluate(&settings, &env, false, true, false).paused);
        settings.fullscreen = Some("pause".into());
        assert!(evaluate(&settings, &env, false, true, false).paused);
        settings.fullscreen = None;
        settings.maximized = "mute".into();
        let muted = evaluate(&settings, &env, false, true, false);
        assert!(muted.muted && !muted.paused);
    }
    #[test]
    fn global_policies_override_wallpaper_preferences_and_keep_manual_pause() {
        let mut settings = PerformanceSettings {
            fullscreen: Some("continue".into()),
            ..Default::default()
        };
        let env = Environment {
            fullscreen: true,
            ..Default::default()
        };
        assert!(!evaluate(&settings, &env, false, true, false).paused);
        assert!(evaluate(&settings, &env, true, true, false).paused);
        settings.fullscreen = Some("pause".into());
        assert_eq!(
            evaluate(&settings, &env, false, false, false)
                .reason
                .as_deref(),
            Some("paused-fullscreen")
        );
        settings.rules.push(AppRule {
            executable: "game.exe".into(),
            action: "continue".into(),
        });
        let env = Environment {
            executable: "GAME.EXE".into(),
            other_audio: true,
            ..env
        };
        settings.other_audio = "mute".into();
        let state = evaluate(&settings, &env, false, true, false);
        assert!(!state.paused && state.muted);
    }
    #[test]
    fn normalizes_quality_and_keeps_rules_as_executable_names() {
        let mut s = PerformanceSettings {
            fps: 999,
            resolution: f64::NAN,
            rules: vec![
                AppRule {
                    executable: " C:\\game.exe ".into(),
                    action: "pause".into(),
                },
                AppRule {
                    executable: " GAME.EXE ".into(),
                    action: "pause".into(),
                },
            ],
            ..Default::default()
        };
        s.normalize();
        assert_eq!(s.fps, 144);
        assert_eq!(s.resolution, 1.);
        assert_eq!(s.rules.len(), 1);
        assert_eq!(s.rules[0].executable, "game.exe");
    }
}
