# atelier-taskbar

The taskbar manager discovers Explorer taskbar windows, applies composition styles and optional floating regions, controls position and coordinates reset/recovery. `accent.rs` and `style.rs` handle composition; `discovery.rs` finds windows; `dock.rs` changes native window regions; `position.rs` manages native edge changes and a guardian; `recovery.rs` listens for Explorer recreation.

`xaml.rs` loads the bridge built from `native/xaml_bridge.cpp`. On supported Windows 11 taskbars, it can modify native background and button elements and retain their original properties for restoration. The bridge is embedded at build time and extracted to a hash-named DLL when needed. Explorer internals and Windows build behavior can change; see [Compatibility](../COMPATIBILITY.md) and [Recovery](../FAILURES_AND_RECOVERY.md).

`native/xaml_probe.cpp` provides a native probe. Tests in `tests/` include real taskbar operations; inspect them before running.
