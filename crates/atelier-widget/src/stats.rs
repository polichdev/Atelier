use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;

#[derive(Debug, Clone, Copy, Default)]
pub struct HardwareStats {
    pub cpu_percent: f32,
    pub mem_percent: f32,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
}

pub struct HardwareMonitor {
    prev_idle: u64,
    prev_kernel: u64,
    prev_user: u64,
    initialized: bool,
}

impl HardwareMonitor {
    pub fn new() -> Self {
        let mut monitor = Self {
            prev_idle: 0,
            prev_kernel: 0,
            prev_user: 0,
            initialized: false,
        };
        monitor.poll();
        monitor
    }

    pub fn poll(&mut self) -> HardwareStats {
        let mut idle_ft = FILETIME::default();
        let mut kernel_ft = FILETIME::default();
        let mut user_ft = FILETIME::default();

        let times_res =
            unsafe { GetSystemTimes(Some(&mut idle_ft), Some(&mut kernel_ft), Some(&mut user_ft)) };

        let idle = filetime_to_u64(&idle_ft);
        let kernel = filetime_to_u64(&kernel_ft);
        let user = filetime_to_u64(&user_ft);

        let cpu_percent = if times_res.is_ok() && self.initialized {
            let delta_idle = idle.saturating_sub(self.prev_idle);
            let delta_kernel = kernel.saturating_sub(self.prev_kernel);
            let delta_user = user.saturating_sub(self.prev_user);
            let total_system = delta_kernel + delta_user;

            if total_system > 0 {
                let busy = total_system.saturating_sub(delta_idle);
                (busy as f32 / total_system as f32 * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        self.prev_idle = idle;
        self.prev_kernel = kernel;
        self.prev_user = user;
        self.initialized = true;

        let mut mem_status = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };

        let mem_res = unsafe { GlobalMemoryStatusEx(&mut mem_status) };

        let (mem_percent, mem_used_mb, mem_total_mb) = if mem_res.is_ok() {
            let total = mem_status.ullTotalPhys;
            let avail = mem_status.ullAvailPhys;
            let used = total.saturating_sub(avail);
            (
                mem_status.dwMemoryLoad as f32,
                used / (1024 * 1024),
                total / (1024 * 1024),
            )
        } else {
            (0.0, 0, 0)
        };

        HardwareStats {
            cpu_percent,
            mem_percent,
            mem_used_mb,
            mem_total_mb,
        }
    }
}

impl Default for HardwareMonitor {
    fn default() -> Self {
        Self::new()
    }
}

fn filetime_to_u64(ft: &FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}
