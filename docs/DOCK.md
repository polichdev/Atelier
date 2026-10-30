# Desktop dock

The **macOS Taskbar** section in Studio's **Taskbar** tab controls Atelier's separate Windows dock. Enabling it hides the native Windows taskbar; disabling it restores the taskbars that Atelier recorded as visible. It is independent of the Windows taskbar appearance controls.

## Layout and applications

Choose a display, screen edge and alignment. Icon size, magnification, spacing, padding, tint, opacity, corner radius, running indicators and app labels are adjustable. **Automatically hide** collapses the dock to an edge strip; **Hide delay** controls when that happens. The dock also attempts to hide during a fullscreen application on its monitor and returns after the application loses focus or exits fullscreen. Detection depends on Windows and the application's display mode.

The app list supports pinning and unpinning, reordering pinned items, choosing custom icons and hiding or showing individual apps with the eye button. Pinned shortcuts are imported from the Windows taskbar on first enable; the original order is used when it can be determined, otherwise the import falls back to filename order. Some Windows-managed apps do not have importable shortcuts. Running and pinned windows are matched where possible, but application launchers and icon resources vary.

**Launch animation** controls the launch bounce. **Open apps maximized** requests maximization for compatible applications; applications may override this. **Space below maximized apps** reserves a bottom gap on the selected display through a temporary Windows appbar. Set it to `0` to remove Atelier's extra reservation. Existing Windows or third-party appbars can also affect the work area.

## System controls

The app list includes visibility controls for Start, Search, hidden tray icons, input language, network, volume, clock and the Atelier button. Network and volume invoke Windows Quick Settings; the clock opens notifications and calendar. The native Windows taskbar and its notification area remain hidden while the dock is enabled.

The hidden-icons and input-language buttons attempt to invoke Explorer controls through Windows UI Automation without revealing the taskbar. They are **not currently reliable**: Windows may not expose a unique accessible control while the taskbar is hidden, in which case the dock reports an error. Atelier does not recreate the notification area or its third-party tray icons. Disable the dock to use the native taskbar if those controls are unavailable.

## Recovery

Atelier starts recovery helpers before hiding Explorer taskbars and enabling Windows taskbar auto-hide. Disable the dock or use the Windows reset action in Studio to restore the taskbar. If Explorer still does not display it, see [Failure modes and recovery](FAILURES_AND_RECOVERY.md). Dock settings, including hidden apps and custom icon data, are stored in the application data directory; see [Configuration](CONFIGURATION.md).

For audio-reactive glow and shape effects, see [Audio reactivity](AUDIO_REACTIVITY.md#dock-reactivity).
