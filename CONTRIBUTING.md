# Contributing to Atelier

## Environment

Develop on Windows with Rust's MSVC toolchain, Visual C++ Build Tools, a Windows SDK and WebView2. See [Development and testing](docs/DEVELOPMENT.md) for commands and optional Playwright setup.

## Workflow

1. Describe the problem and intended behavior before large changes.
2. Keep changes focused and include tests appropriate to their impact.
3. Run the relevant checks and report limitations, Windows version and display configuration when they matter.
4. Open a pull request describing behavior, validation and recovery implications.

Use concise Conventional Commit-style subjects, for example `fix(taskbar): restore native button properties` or `docs(ui): explain dock configuration`.

## Shell integration

Taskbar and Explorer changes affect the live desktop. Preserve original state, review cleanup and crash recovery, and prefer isolated fixtures for automated tests. Do not assume undocumented Explorer behavior is stable across Windows versions. Do not claim experimental wallpaper, audio or widget features are complete without verifying the full user-facing path.
