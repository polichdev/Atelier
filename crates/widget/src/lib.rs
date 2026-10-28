pub mod clock;
pub mod media;
pub mod stats;
pub mod window;

use atelier_common::error::Result;
use clock::{ClockWeatherInfo, ClockWeatherWidget};
use media::{MediaTrackInfo, SystemMediaMonitor};
use stats::{HardwareMonitor, HardwareStats};
use std::sync::{Arc, Mutex};
use window::LayeredWidgetWindow;

pub struct WidgetSubsystem {
    hardware_monitor: Arc<Mutex<HardwareMonitor>>,
    clock_widget: Arc<Mutex<ClockWeatherWidget>>,
}

impl WidgetSubsystem {
    pub fn new() -> Self {
        Self {
            hardware_monitor: Arc::new(Mutex::new(HardwareMonitor::new())),
            clock_widget: Arc::new(Mutex::new(ClockWeatherWidget::new())),
        }
    }

    pub fn poll_hardware_stats(&self) -> HardwareStats {
        if let Ok(mut mon) = self.hardware_monitor.lock() {
            mon.poll()
        } else {
            HardwareStats::default()
        }
    }

    pub fn poll_clock(&self) -> ClockWeatherInfo {
        if let Ok(clk) = self.clock_widget.lock() {
            clk.poll()
        } else {
            ClockWeatherInfo {
                time_str: String::new(),
                date_str: String::new(),
                temperature_c: 0.0,
                condition: String::new(),
            }
        }
    }

    pub fn poll_media(&self) -> Result<MediaTrackInfo> {
        SystemMediaMonitor::poll()
    }

    pub fn create_overlay_window(
        &self,
        title: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        click_through: bool,
    ) -> Result<LayeredWidgetWindow> {
        LayeredWidgetWindow::new(title, x, y, width, height, click_through)
    }
}

impl Default for WidgetSubsystem {
    fn default() -> Self {
        Self::new()
    }
}
