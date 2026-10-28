# Development & Testing

## Setup

Use Windows 10/11 with a stable Rust MSVC toolchain, Visual C++ build tools, a Windows SDK and WebView2 Runtime. The taskbar build script invokes the C++ compiler for the XAML bridge. Node.js and Playwright are required for the browser tests; they are not required to load Studio's checked-in web assets through Cargo.

```powershell
cargo check --workspace
cargo run -p atelier-ui --bin Atelier
```

`scripts\run-dev.cmd` builds and launches a copy under `target\dev-run`. It stops matching Atelier development processes first; do not use it if another matching development instance must remain running.

## Checks

```powershell
cargo test -p atelier-common
cargo test -p atelier-ui --lib
cargo check --workspace
```

The Windows CI configuration also runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace` and `cargo build --release`. These commands describe configured checks, not a guarantee that the current tree passes every check.

### Dock tests

```powershell
powershell -File scripts/test-desktop-dock.ps1
node --test scripts/tests/desktop-dock.test.cjs
```

The PowerShell script builds an isolated test copy under `target/` and uses fixture windows; debug dock-only mode does not hide the user's Explorer taskbar. It also tests the watchdog. The browser tests require Playwright and an available browser. Set `NODE_PATH` to its installation when needed; `ATELIER_TEST_BROWSER=msedge` selects installed Edge. Browser screenshots are written to `target/dock-tests/`. Windows foreground activation and browser availability can affect integration results.

### Taskbar integration tests

`crates/atelier-taskbar/tests/` includes tests that operate on real Explorer taskbars. The optional `xaml-probe` feature builds the native fixture and probe. Inspect the test before running it; do not confuse an isolated XAML fixture with a test against Explorer itself.

## Packaging

`packaging/nsis/installer.nsi` and `packaging/msix/AppxManifest.xml` provide installer sources. They expect the appropriate release executable and packaging tools/assets; a Cargo build alone does not verify an installer or produce a signed package.
