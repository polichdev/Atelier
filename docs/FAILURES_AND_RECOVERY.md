# Failure Modes & Recovery

## Dock hides the Windows taskbar

Before hiding each visible Explorer taskbar, Atelier starts a watchdog and waits for its readiness signal. When Windows taskbar auto-hide was off, the dock enables it so Windows can release the reserved work area. A separate guardian restores the previous setting on unexpected exit; normal shutdown restores it only if it still matches the state Atelier applied. The parent records hidden windows and restores them when the dock is disabled or the application shuts down. The watchdog waits for the parent process and attempts to show its taskbar if the parent exits. It only restores windows that were visible when Atelier took responsibility for them.

If the taskbar is still missing, disable the macOS Taskbar in Studio or use the Windows reset action. If needed, restart Windows Explorer through Task Manager. The dock's hidden-icons and language buttons attempt to invoke Explorer controls without showing the taskbar. This is not reliable on every Windows build: the current implementation can report that it could not identify a unique control. See [Desktop dock](DOCK.md#system-controls) for the limitation.

## Native appearance or position is incorrect

Use **Reset button styles**, **Restore sharp icons**, or the Windows reset action in Studio. If a previous version left Explorer in an inconsistent state, close Atelier and restart Windows Explorer. Taskbar positioning uses a separate guardian to attempt restoring the original edge and saved registry value after a controller exit. Some Windows builds do not accept every requested position.

The XAML bridge tracks changes to native properties and attempts to restore them when its controller goes away. These are recovery mechanisms, not a guarantee against all Windows crashes, shell changes or third-party conflicts. A style may also be unsupported on a particular Explorer build.

## Explorer restarts

The taskbar recovery listener attempts to apply the current session's configuration when it receives the registered Explorer taskbar-created message. Dock maintenance checks for taskbar windows while the dock is enabled. Review the actual result after an Explorer or Windows update.

## Tests and diagnostics

See [Development and testing](DEVELOPMENT.md) for isolated dock fixtures and output paths. Do not run the taskbar integration tests on an important desktop session without first reviewing their effects.
