# Native interface motion

The active native interface animates only the `city`, `dreams`, and `tunnel` cover patterns. The Appearance setting `Animate cover patterns` controls them and defaults on for existing settings files. Editor, panel, and graph transitions do not exist, so navigation and editing stay immediate.

On macOS, cover animation is disabled when `NSWorkspace.accessibilityDisplayShouldReduceMotion` is enabled. Windows and Linux use the manual setting because Masonry and winit expose no shared reduced-motion preference. The system preference is read when a cover is created.
