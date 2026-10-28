#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    if atelier_ui_lib::watchdog_args() || atelier_ui_lib::desktop_test_args() { return; }
    atelier_ui_lib::run();
}
