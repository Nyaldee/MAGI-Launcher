# CASPER Shot

*[Lire en français](README.fr.md) · [日本語で読む](README.jp.md) · [阅读中文版](README.zh-CN.md)*

A tiny Windows screenshot tool: two global hotkeys, WebP output, a tray icon. No window, nothing to install.

## Features

- Global hotkey to capture the monitor under your cursor (default `PrintScreen`).
- Global hotkey to drag-select a region, or click a window to snap to it, across any monitor (default `Shift+PrintScreen`). Escape or right-click cancels. Both hotkeys are configurable, see below.
- Left-click the tray icon to jump straight into region-select mode, no hotkey needed.
- Every capture is copied to the clipboard and saved as `.webp` in a `YYYY-MM` subfolder, named after the window under the cursor, the date and the quality (`Discord_06-14-2026_q90.webp`). An existing file is never overwritten: ` (1)`, ` (2)`… is added instead.
- A preview briefly shows at the bottom of the screen. Left-click it to open the image, right-click it to open its folder. It never shows up in your next captures.
- Tray icon menu (right-click): enable/disable the hotkeys, open the screenshots folder, Settings, Reload, Website (opens nyaruhodo.com), exit.

## Settings (`state.json`)

Created next to the executable on first run. Tray menu → Settings opens it (in Notepad if no app is associated with `.json`); Reload applies your changes. If the file contains invalid JSON, CASPER Shot uses the defaults and leaves the file untouched so you can fix it.

```json
{
  "quality": 90,
  "hotkey_fullscreen": "PrintScreen",
  "hotkey_region": "Shift+PrintScreen",
  "save_path": "%USERPROFILE%\\Pictures\\Screenshots",
  "preview_percent": 40,
  "preview_duration_ms": 2500
}
```

- `quality`: 0-99 is lossy WebP, 100 is lossless.
- `hotkey_*`: any combination of `Ctrl`/`Alt`/`Shift`/`Win` plus a letter, digit, F1-F12, or `PrintScreen`.
- `save_path`: supports `%ENV_VAR%` expansion; relative paths resolve next to the executable. `%USERPROFILE%\Pictures` (and Desktop, Documents, Downloads, Music, Videos) follows the folder wherever Windows has moved it, OneDrive included.
- `preview_percent`: size of the preview, as % of the screen's width and height; `0` disables the preview entirely.
- `preview_duration_ms`: how long the preview stays on screen before closing.

## Hotkeys not working?

`RegisterHotKey` only lets one process own a given key combination system-wide. If another running app already has the same combination bound (ShareX, Greenshot, Windows' own screen-snipping shortcut in Settings → Accessibility → Keyboard, etc.), CASPER Shot's registration fails silently and the tray tooltip reads "hotkeys unavailable" instead of listing the shortcuts. Close or reconfigure whichever app is holding them, or pick different shortcuts in `state.json` and Reload.

## Updating

Run `casper_shot_updater.bat` from the `CASPER Shot` folder: it closes CASPER Shot, downloads the latest release, extracts it in place and starts it again. `state.json` and your captures are left untouched.

## License

Free for personal, non-commercial use. Copyright (C) 2026 Nyaldee, all rights reserved — see [LICENSE](../LICENSE) for the full terms. Download it only from this repository's [releases](https://github.com/Nyaldee/nyaruhodo/releases).

[Claude](https://claude.com), Anthropic's AI, contributed to the code of this project.
