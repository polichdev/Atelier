# Audio reactivity

Atelier can react to sound playing through the default Windows output in three places: a desktop visualizer, optional wallpaper effects and optional dock effects. These are separate controls. Enabling one does not automatically enable the others.

The audio subsystem uses WASAPI loopback to analyze system output locally; it is not a microphone input and Atelier does not record the captured audio. Consumers share capture, which starts on demand and stops after an idle period. Output-device availability and changes can affect capture.

## Desktop visualizer

Open **Audio**, then enable **Show on desktop**. Choose bars, wave, dots, circle or symmetric wave, or start with a preset. The panel controls colors, gradient, glow, thickness, bar count, background opacity, sensitivity, smoothing, dimensions and corner radius.

The Studio preview uses an example animation, so it can move even when no audio is playing. The desktop visualizer responds to the system output. **Hide when silent** hides the drawing while continuing to monitor sound; **Pause in fullscreen** suspends visualizer polling during fullscreen detection.

Drag the desktop widget to move it, double-click it to enter or leave resize mode, and right-click it to open its settings. Settings and position are stored with the other widgets.

## Wallpaper effects

Select an imported wallpaper under **My wallpapers** and use its **Audio reactivity** controls to enable pulse or glow and set their strengths. These effects apply to the whole Atelier wallpaper and pause with playback; they do not implement an author's Wallpaper Engine audio scripts. See [Wallpapers and Workshop](WALLPAPERS.md).

## Dock reactivity

In **Audio**, enable **Dock reactivity** and adjust glow color and strength. Shape effects can be off, breathing, corners, or both; shape strength, frequency band, sensitivity and smoothing are adjustable. The effects apply only when the separate desktop dock is enabled and visible. Fullscreen hiding and the dock's own auto-hide stop the visual effect while hidden.

## Troubleshooting

Check that Windows is playing sound through its default output, that the desired visual effect is enabled, and that sensitivity and strength are high enough to see a response. Check the visualizer's silence and fullscreen settings if it disappears. If the selected output device is unavailable, Atelier reports an audio-device error and attempts to reconnect when the output changes.
