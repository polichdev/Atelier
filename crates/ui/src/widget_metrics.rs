use std::{
    ffi::c_void,
    sync::{Mutex, OnceLock},
};
#[repr(C)]
struct Value {
    status: u32,
    value: f64,
}
#[repr(C)]
struct Item {
    name: *const u16,
    value: Value,
}
#[link(name = "pdh")]
extern "system" {
    fn PdhOpenQueryW(s: *const u16, u: usize, q: *mut isize) -> u32;
    fn PdhAddEnglishCounterW(q: isize, p: *const u16, u: usize, c: *mut isize) -> u32;
    fn PdhCollectQueryData(q: isize) -> u32;
    fn PdhGetFormattedCounterValue(c: isize, f: u32, t: *mut u32, v: *mut Value) -> u32;
    fn PdhGetFormattedCounterArrayW(
        c: isize,
        f: u32,
        b: *mut u32,
        n: *mut u32,
        p: *mut c_void,
    ) -> u32;
    fn PdhCloseQuery(q: isize) -> u32;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetDiskFreeSpaceExW(
        path: *const u16,
        available: *mut u64,
        total: *mut u64,
        free: *mut u64,
    ) -> i32;
    fn LoadLibraryExW(name: *const u16, file: isize, flags: u32) -> isize;
    fn GetProcAddress(module: isize, name: *const u8) -> *mut c_void;
}
struct Counters {
    q: isize,
    freq: isize,
    net: isize,
    bandwidth: isize,
}
unsafe impl Send for Counters {}
impl Drop for Counters {
    fn drop(&mut self) {
        unsafe {
            PdhCloseQuery(self.q);
        }
    }
}
impl Counters {
    fn new() -> Option<Self> {
        unsafe {
            let mut q = 0;
            if PdhOpenQueryW(std::ptr::null(), 0, &mut q) != 0 {
                return None;
            }
            let add = |path: &str| {
                let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
                let mut c = 0;
                if PdhAddEnglishCounterW(q, wide.as_ptr(), 0, &mut c) != 0 {
                    0
                } else {
                    c
                }
            };
            let freq = add(r"\Processor Information(_Total)\Processor Frequency");
            let net = add(r"\Network Interface(*)\Bytes Total/sec");
            let bandwidth = add(r"\Network Interface(*)\Current Bandwidth");
            let _ = PdhCollectQueryData(q);
            Some(Self {
                q,
                freq,
                net,
                bandwidth,
            })
        }
    }
    unsafe fn values(counter: isize) -> std::collections::HashMap<String, f64> {
        let mut result = std::collections::HashMap::new();
        let (mut size, mut count) = (0, 0);
        if counter == 0
            || PdhGetFormattedCounterArrayW(
                counter,
                0x200,
                &mut size,
                &mut count,
                std::ptr::null_mut(),
            ) != 0x800007d2
            || size == 0
        {
            return result;
        }
        let mut data = vec![0u64; (size as usize + 7) / 8];
        if PdhGetFormattedCounterArrayW(
            counter,
            0x200,
            &mut size,
            &mut count,
            data.as_mut_ptr().cast(),
        ) != 0
        {
            return result;
        }
        for item in std::slice::from_raw_parts(data.as_ptr().cast::<Item>(), count as usize) {
            if item.name.is_null()
                || item.value.status > 1
                || !item.value.value.is_finite()
                || item.value.value < 0.0
            {
                continue;
            }
            let mut n = 0;
            while n < 1024 && *item.name.add(n) != 0 {
                n += 1;
            }
            result.insert(
                String::from_utf16_lossy(std::slice::from_raw_parts(item.name, n)),
                item.value.value,
            );
        }
        result
    }
    fn poll(&mut self) -> (Option<f64>, Option<f64>, Option<f64>) {
        unsafe {
            if PdhCollectQueryData(self.q) != 0 {
                return (None, None, None);
            }
            let mut v = Value {
                status: 0,
                value: 0.0,
            };
            let freq = if self.freq != 0
                && PdhGetFormattedCounterValue(self.freq, 0x200, std::ptr::null_mut(), &mut v) == 0
                && v.status <= 1
                && v.value > 0.0
            {
                Some(v.value)
            } else {
                None
            };
            let rates = Self::values(self.net);
            let bandwidths = Self::values(self.bandwidth);
            let active = rates
                .iter()
                .filter_map(|(name, rate)| {
                    bandwidths.get(name).filter(|b| **b > 0.0).map(|bandwidth| {
                        (*rate, (*rate * 8.0 / bandwidth * 100.0).clamp(0.0, 100.0))
                    })
                })
                .max_by(|a, b| a.0.total_cmp(&b.0));
            (freq, active.map(|v| v.0), active.map(|v| v.1))
        }
    }
}
fn gpu_temperature() -> Option<u32> {
    type Get = unsafe extern "C" fn(u32, *mut *mut c_void) -> i32;
    type Temp = unsafe extern "C" fn(*mut c_void, u32, *mut u32) -> i32;
    static NVML: OnceLock<Option<(Get, Temp)>> = OnceLock::new();
    let (get, temp) = *NVML
        .get_or_init(|| unsafe {
            let name: Vec<u16> = "nvml.dll".encode_utf16().chain(Some(0)).collect();
            let lib = LoadLibraryExW(name.as_ptr(), 0, 0x800);
            if lib == 0 {
                return None;
            }
            let init = GetProcAddress(lib, b"nvmlInit_v2\0".as_ptr());
            let get = GetProcAddress(lib, b"nvmlDeviceGetHandleByIndex_v2\0".as_ptr());
            let temp = GetProcAddress(lib, b"nvmlDeviceGetTemperature\0".as_ptr());
            if init.is_null() || get.is_null() || temp.is_null() {
                return None;
            }
            let init: unsafe extern "C" fn() -> i32 = std::mem::transmute(init);
            if init() != 0 {
                return None;
            }
            Some((
                std::mem::transmute::<_, Get>(get),
                std::mem::transmute::<_, Temp>(temp),
            ))
        })
        .as_ref()?;
    let (mut device, mut value) = (std::ptr::null_mut(), 0);
    if unsafe { get(0, &mut device) != 0 || temp(device, 0, &mut value) != 0 } {
        return None;
    }
    Some(value)
}
pub fn poll() -> (
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<u32>,
) {
    static COUNTERS: OnceLock<Mutex<Option<Counters>>> = OnceLock::new();

    let (freq, network, network_percent) = COUNTERS
        .get_or_init(|| Mutex::new(Counters::new()))
        .lock()
        .ok()
        .and_then(|mut c| c.as_mut().map(Counters::poll))
        .unwrap_or((None, None, None));
    let root = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into()) + "\\";
    let wide: Vec<u16> = root.encode_utf16().chain(Some(0)).collect();
    let (mut free, mut total) = (0, 0);
    let disk = if unsafe {
        GetDiskFreeSpaceExW(wide.as_ptr(), std::ptr::null_mut(), &mut total, &mut free)
    } != 0
        && total > 0
    {
        Some((total - free) as f64 / total as f64 * 100.0)
    } else {
        None
    };

    (freq, network, network_percent, disk, gpu_temperature())
}
