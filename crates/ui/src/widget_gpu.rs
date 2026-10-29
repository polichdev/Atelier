use std::{
    collections::HashMap,
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
    fn PdhOpenQueryW(source: *const u16, user: usize, query: *mut isize) -> u32;
    fn PdhAddEnglishCounterW(
        query: isize,
        path: *const u16,
        user: usize,
        counter: *mut isize,
    ) -> u32;
    fn PdhCollectQueryData(query: isize) -> u32;
    fn PdhGetFormattedCounterArrayW(
        counter: isize,
        format: u32,
        size: *mut u32,
        count: *mut u32,
        items: *mut c_void,
    ) -> u32;
    fn PdhCloseQuery(query: isize) -> u32;
}
struct Gpu {
    query: isize,
    counter: isize,
}
unsafe impl Send for Gpu {}
impl Drop for Gpu {
    fn drop(&mut self) {
        unsafe {
            PdhCloseQuery(self.query);
        }
    }
}
impl Gpu {
    fn new() -> Option<Self> {
        unsafe {
            let mut query = 0;
            if PdhOpenQueryW(std::ptr::null(), 0, &mut query) != 0 {
                return None;
            }
            let mut counter = 0;
            let path: Vec<u16> = r"\GPU Engine(*)\Utilization Percentage"
                .encode_utf16()
                .chain(Some(0))
                .collect();
            if PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut counter) != 0 {
                PdhCloseQuery(query);
                return None;
            }
            if PdhCollectQueryData(query) != 0 {
                PdhCloseQuery(query);
                return None;
            }
            Some(Self { query, counter })
        }
    }
    fn poll(&mut self) -> Option<f64> {
        unsafe {
            if PdhCollectQueryData(self.query) != 0 {
                return None;
            }
            let (mut bytes, mut count) = (0, 0);
            let status = PdhGetFormattedCounterArrayW(
                self.counter,
                0x200,
                &mut bytes,
                &mut count,
                std::ptr::null_mut(),
            );
            if status != 0x800007d2 || bytes == 0 {
                return None;
            }
            let mut buffer = vec![0u64; (bytes as usize + 7) / 8];
            if PdhGetFormattedCounterArrayW(
                self.counter,
                0x200,
                &mut bytes,
                &mut count,
                buffer.as_mut_ptr().cast(),
            ) != 0
            {
                return None;
            }
            let items = std::slice::from_raw_parts(buffer.as_ptr().cast::<Item>(), count as usize);
            let mut engines = HashMap::<String, f64>::new();
            for item in items {
                if item.name.is_null() || item.value.status > 1 || !item.value.value.is_finite() {
                    continue;
                }
                let mut n = 0;
                while n < 1024 && *item.name.add(n) != 0 {
                    n += 1
                }
                let name = String::from_utf16_lossy(std::slice::from_raw_parts(item.name, n));
                let key = name.find("_luid_").map(|i| &name[i..]).unwrap_or(&name);
                *engines.entry(key.to_string()).or_default() += item.value.value.max(0.0);
            }
            engines
                .values()
                .copied()
                .reduce(f64::max)
                .map(|v| v.clamp(0.0, 100.0))
        }
    }
}
pub fn poll() -> Option<f64> {
    static GPU: OnceLock<Mutex<Option<Gpu>>> = OnceLock::new();
    let mut gpu = GPU.get_or_init(|| Mutex::new(Gpu::new())).lock().ok()?;
    gpu.as_mut()?.poll()
}
