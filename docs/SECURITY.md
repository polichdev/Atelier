# Security Considerations

Atelier interacts with the Windows shell and local application windows. Its Windows 11 XAML bridge is native code loaded into the Explorer taskbar process; it modifies taskbar elements, so Windows updates or other shell customization software can affect compatibility. The dock can hide taskbar windows and launch applications selected by the user.

The dock reads local shortcut paths, enumerates visible windows and stores pinned paths and optional custom icon data in the application data directory. Treat those files as user data. Studio's WebView has Tauri commands enabled and the current Tauri configuration sets `csp` to `null`; changes that load remote content or broaden IPC access deserve particular review.

Recovery helpers reduce the risk of leaving a taskbar hidden or moved after the parent exits, but cannot guarantee recovery from every system failure. See [Failure Modes & Recovery](FAILURES_AND_RECOVERY.md).

For a suspected vulnerability, avoid publishing exploit details in a public issue. Use the repository's private vulnerability reporting option if it is enabled, or contact the maintainer through an available private channel. No response-time commitment is specified here.
