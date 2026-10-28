# atelier-common

Shared Rust types used across the workspace. `config.rs` defines `TaskbarStyle`, taskbar geometry and `TaskbarAppearance`, including default values and normalization. `presets.rs` defines named taskbar presets; `error.rs` defines shared errors. `AtelierConfig` includes autostart and taskbar configuration, but an available field does not by itself establish a complete Studio feature.

Tests cover default appearance, backward-compatible configuration deserialization and button width normalization: `cargo test -p atelier-common`.
