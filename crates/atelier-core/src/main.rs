use atelier_common::config::{AtelierConfig, TaskbarConfig, TaskbarStyle};
use atelier_taskbar::TaskbarManager;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting Atelier daemon...");

    let config = AtelierConfig {
        taskbar: TaskbarConfig {
            style: TaskbarStyle::Clear,
            floating_dock: false,
            floating_margin: 8,
        },
        ..Default::default()
    };

    let manager = TaskbarManager::new(config.taskbar);
    manager.apply_current_style()?;
    manager.start_recovery()?;

    tracing::info!("Atelier daemon is active. Press Ctrl+C to terminate.");

    tokio::signal::ctrl_c().await?;

    tracing::info!("Shutting down Atelier daemon. Restoring OS defaults...");
    manager.stop_recovery()?;
    manager.reset_all()?;
    tracing::info!("Atelier daemon successfully terminated.");

    Ok(())
}
