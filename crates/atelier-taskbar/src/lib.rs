pub mod accent;
pub mod discovery;
pub mod dock;
pub mod recovery;
pub mod style;

use atelier_common::config::{TaskbarConfig, TaskbarStyle};
use atelier_common::error::Result;
use discovery::find_all_taskbars;
use dock::{apply_floating_dock, reset_floating_dock};
use recovery::TaskbarRecoveryListener;
use std::sync::{Arc, Mutex};
use style::apply_taskbar_style;

#[derive(Clone)]
pub struct TaskbarManager {
    config: Arc<Mutex<TaskbarConfig>>,
    recovery: Arc<Mutex<Option<TaskbarRecoveryListener>>>,
}

impl TaskbarManager {
    pub fn new(config: TaskbarConfig) -> Self {
        Self {
            config: Arc::new(Mutex::new(config)),
            recovery: Arc::new(Mutex::new(None)),
        }
    }

    pub fn apply_current_style(&self) -> Result<()> {
        let config = {
            let lock = self
                .config
                .lock()
                .map_err(|e| atelier_common::error::AtelierError::Generic(e.to_string()))?;
            lock.clone()
        };

        let taskbars = find_all_taskbars();
        for &hwnd in &taskbars {
            apply_taskbar_style(hwnd, config.style)?;

            if config.floating_dock {
                let _ = apply_floating_dock(hwnd, config.floating_margin, 12);
            } else {
                let _ = reset_floating_dock(hwnd);
            }
        }

        Ok(())
    }

    pub fn set_style(&self, style: TaskbarStyle) -> Result<()> {
        {
            let mut lock = self
                .config
                .lock()
                .map_err(|e| atelier_common::error::AtelierError::Generic(e.to_string()))?;
            lock.style = style;
        }
        self.apply_current_style()
    }

    pub fn set_floating_dock(&self, enabled: bool, margin: i32) -> Result<()> {
        {
            let mut lock = self
                .config
                .lock()
                .map_err(|e| atelier_common::error::AtelierError::Generic(e.to_string()))?;
            lock.floating_dock = enabled;
            lock.floating_margin = margin;
        }
        self.apply_current_style()
    }

    pub fn reset_all(&self) -> Result<()> {
        let taskbars = find_all_taskbars();
        for &hwnd in &taskbars {
            let _ = apply_taskbar_style(hwnd, TaskbarStyle::Default);
            let _ = reset_floating_dock(hwnd);
        }
        Ok(())
    }

    pub fn start_recovery(&self) -> Result<()> {
        let self_clone = self.clone();
        let listener = TaskbarRecoveryListener::start(move || {
            let _ = self_clone.apply_current_style();
        })?;

        let mut lock = self
            .recovery
            .lock()
            .map_err(|e| atelier_common::error::AtelierError::Generic(e.to_string()))?;
        *lock = Some(listener);

        Ok(())
    }

    pub fn stop_recovery(&self) -> Result<()> {
        let mut lock = self
            .recovery
            .lock()
            .map_err(|e| atelier_common::error::AtelierError::Generic(e.to_string()))?;
        if let Some(mut listener) = lock.take() {
            listener.stop();
        }
        Ok(())
    }
}

impl Drop for TaskbarManager {
    fn drop(&mut self) {
        if Arc::strong_count(&self.config) <= 2 {
            let _ = self.reset_all();
        }
    }
}
