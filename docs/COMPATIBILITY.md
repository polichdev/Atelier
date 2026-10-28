# Compatibility

| Area | Current scope |
| --- | --- |
| Operating system | Windows 10 and 11 are the project targets. Exact build coverage is not established by the repository. |
| Native app buttons | Custom button and icon appearance requires a discoverable Windows 11 XAML taskbar and a loadable bridge. |
| Materials | Explorer composition and XAML behavior differ across Windows builds; a requested material may not look identical on every build. |
| Floating Windows taskbar | Region geometry supports horizontal taskbars. Its inset is bounded to leave at least 700 px where the display permits. |
| Native taskbar position | Explorer may reject a requested edge on a particular Windows build; the controller reports failure and attempts restoration. |
| macOS Taskbar | A separate Atelier dock; it is not the macOS Dock. Enabling it hides the Windows taskbar, including its notification area. |
| Studio backdrop | The Studio requests Windows 11 Mica; the result depends on OS and transparency settings. |
| Build | Windows MSVC, Windows SDK, C++ compiler and WebView2 are required for the normal desktop build. |

These are implementation constraints, not a claim that every Windows configuration has been tested. Test on the Windows versions, DPI scales and monitor layouts relevant to a change.
