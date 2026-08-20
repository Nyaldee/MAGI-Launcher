# MAGI Launcher

<p align="center">
  <img src="MAGI Launcher.png" alt="MAGI Launcher screenshot">
</p>

*[Lire en français](README.fr.md)*

A fast, tentacular, keyboard-only application launcher for Windows: hit a global hotkey, type a few letters, hit Enter. It launches apps, folders, and shortcuts; doubles as a calculator and a hex color previewer; switches between open windows; sets quick timers; keeps sticky notes; auto-restarts anything you want kept alive; browses and empties the Recycle Bin; looks up emoji by name; controls media playback; keeps an optional RAM-only clipboard history; ejects USB drives that Windows itself sometimes won't; and ships with over 100 color themes. A small, lightweight `.exe` with no installer and nothing to configure beyond your own shortcut list.

## Features

- Global hotkey to summon/dismiss the launcher from anywhere (default `Ctrl+Space`, configurable)
- Fuzzy search over your list of apps, folders, and shortcuts
- A `shortcuts/` folder next to the executable — drop any file in it (`.lnk`, `.bat`, `.cmd`, `.vbs`, anything) and it becomes a launchable entry automatically, listed after `apps.json`'s own entries
- Inline calculator (`2*(3+4)` → `= 14`, copies the result on Enter)
- Hex color preview (`#3a8ea0` fills the list with that color, copies the hex on Enter)
- Window Switcher — fuzzy-search and jump to any open window, or close/kill it on the spot
- Timer with a duration parser (`5m`, `90s`, `1h`) and a DVD-bounce screensaver easter egg when it fires
- Sticky Notes — an unlimited, fuzzy-searchable list of quick text notes, copy or delete on the spot
- Auto-restart — pick any target to keep alive; if its process ever disappears (crash or manual close), it's relaunched automatically within a couple seconds
- Recycle Bin item count/size shown live, browse what's actually in it right from the launcher — copy an item's name or delete it individually — empty it with a keystroke
- Emoji picker — fuzzy-search the official Unicode emoji names, `Enter` copies the emoji itself
- Copy History — opt-in, RAM-only record of everything you copy (text only); browse and re-copy from the launcher, never written to disk
- Eject — fuzzy-searchable list of connected USB drives, `Enter` ejects one on the spot (`Shift+Delete` to force it if something's using it); works even when Windows' own "Eject" option is missing or greyed out
- Media key control (play/pause, next/previous track, volume) without a physical media keyboard
- Over 100 built-in color themes, switchable live from the launcher with instant preview, no restart needed
- Resize the popup and its border live from the keyboard (`Ctrl+1`–`Ctrl+0` / `Ctrl+-`/`Ctrl+=`), persisted immediately
- Runs as a single instance with a tray icon (toggle hotkey / toggle auto-restart / toggle Copy History / GitHub / quit)
- Falls back to "Run" behavior (like Win+R) for anything that doesn't match an app

## Keyboard shortcuts

| Key | Action |
|---|---|
| Global hotkey (default `Ctrl+Space`) | Show / hide the launcher |
| Type | Fuzzy-filter the list live |
| `↑` / `↓` or `Ctrl+W` / `Ctrl+S` | Move selection up / down |
| `←` / `→` or `Ctrl+A` / `Ctrl+D` | Jump a page (10 items) back / forward |
| `Enter` | Launch the selected entry — *(Recycle Bin main entry)* browse what's currently in it instead of emptying it — *(Recycle Bin browse view)* copy the highlighted item's full name (with extension) to the clipboard and close the launcher — *(Copy History)* re-copy the highlighted entry and close the launcher, without re-adding it to the history — *(Eject)* eject the highlighted drive if nothing has it open, otherwise leave it alone (see `Shift+Delete` to force it); the launcher stays open either way so you can eject more than one in a row |
| `Shift+Enter` | Reveal the selected entry in Explorer instead of launching it — *(Recycle Bin main entry)* empty it (launcher stays open) — *(Sticky Notes)* Open `notes.json` in its default editor instead of copying |
| `Tab` | Jump to the Window Switcher — works the same from anywhere in the launcher, not just the main list |
| `Escape` | Back out to the main list from any mode — or close the launcher if you're already there. During the Timer's DVD-bounce screensaver, stops the bounce and returns to the main list instead of closing |
| `Delete` | *(Window Switcher)* Close the highlighted window gently (`WM_CLOSE`, same as clicking its X) — *(Sticky Notes, from the picker)* Delete the highlighted note — *(main list, on the "Note" entry)* Delete the most recent note directly, without opening the picker — *(Auto-restart)* Stop watching the highlighted target — *(Copy History)* Delete the highlighted entry — *(Recycle Bin main entry)* Empty the whole bin (launcher stays open) — *(Recycle Bin browse view)* Permanently delete just the highlighted item — *(Timer, armed or from inside the duration prompt)* Cancel the countdown |
| `Shift+Delete` | *(Window Switcher)* Force-kill the highlighted window's process, for one that stopped responding to `Delete` — *(Sticky Notes)* Delete every note — *(Auto-restart)* Stop watching every target — *(Copy History)* Clear the whole history — *(Recycle Bin browse view)* Empty the whole bin (launcher stays open) — *(Eject)* Force-eject the highlighted drive even if something still has it open; plain `Delete` does nothing here |
| `Ctrl+1`–`Ctrl+9` / `Ctrl+0` | Resize the popup live to 10%–90% / 100% of screen width, persisted to `themes.json` immediately |
| `Ctrl+-` / `Ctrl+=` | Shrink / grow the border by 1px, live, persisted immediately |
| `Backspace` on an empty search | Back out of the Window Switcher / Theme Picker / Timer / Sticky Notes / Auto-restart / Recycle Bin browse view / Emoji picker / Copy History / Eject |
| Left-click tray icon | Show / hide the launcher |
| Right-click tray icon | Toggle hotkey / Toggle auto-restart / Toggle Copy History / GitHub / Quit menu |

The search box always keeps keyboard focus — every key above is intercepted there directly, so you never have to click or tab into anything else to keep typing. The mouse has no effect anywhere inside the launcher's own window (clicks, hover, cursor changes) — only the tray icon reacts to it. `Alt+F4` never closes the launcher's process — it just hides the popup, same as `Escape`; the launcher only ever exits via "Quit" in the tray menu.

## Search modes

Typing something in the search box is interpreted, in order:

1. **Hex color** (`#fff` or `#3a8ea0`) — fills the whole list with that color as a live preview; `Enter` copies the hex code to the clipboard.
2. **Math expression** (must contain an operator, e.g. `2+2`, `100/3`) — shows `= <result>`; `Enter` copies the result. Evaluated through a small hand-written recursive-descent parser restricted to numbers and arithmetic operators — nothing that could look up a name, call anything, or reach outside the expression itself can ever run.
3. **App name** — fuzzy match: your query just needs to appear as a subsequence of the name (in order, not necessarily consecutive), so `vsc` finds "Visual Studio Code". Results are ranked: exact prefix match first, then plain substring, then fuzzy matches (tighter matches ranked above more scattered ones). Ties within the same tier keep the order their entries have in `apps.json` — so among equally-good matches, whichever one is listed higher up in `apps.json` shows up first in the results. There's no usage history/frecency behind this at all: MAGI never remembers what you launched before, the position in `apps.json` is the only tie-breaker, always the same regardless of how often (or recently) you've picked something.
4. **Anything else** — if nothing matches, `Enter` runs the raw text like Windows' Run dialog (`Win+R`), through the same `ShellExecute`/PATH resolution. An email-shaped query (`someone@example.com`) is launched as `mailto:` instead of failing as a bogus file path.

## Window Switcher

Press `Tab` from anywhere in the launcher to list every open top-level window (title, filtered the same fuzzy way as apps):

- `Enter` activates the highlighted window
- `Delete` closes it gently (`WM_CLOSE`, same as clicking the X) and stays in the switcher for the next one
- `Shift+Delete` force-kills its process (`TerminateProcess`) for a window that stopped responding to `Delete`
- `Escape` backs out of the switcher to the main list without touching any window

`Shift+Delete` is deliberately one modifier away from the plain `Delete` close: many windows — every File Explorer folder window, for instance — run inside the very same process as the desktop and taskbar (unless "Launch folder windows in a separate process" is enabled), so `TerminateProcess` on one of them takes down all of `explorer.exe`, not just the highlighted window. Reach for it only when `Delete` truly isn't getting through.

## Timer

Add a `"magi:timer"` entry (see below) to unlock it — the search box's placeholder switches to `Type a duration (5m, 90s, 1h...)` while you're in it. Type a duration (`5m`, `90s`, `1h`, or a bare number defaulting to minutes) and hit `Enter` to arm a countdown, shown live next to "Timer" in the main list (`Timer: --:--` while idle). When it reaches zero, the popup starts bouncing around the screen DVD-screensaver style, switching theme on every wall bounce — dismiss it with the global hotkey, a mouse click anywhere on it, or any key (`Escape`, `Tab`...) while it has keyboard focus. Changed your mind before it alerts you? Hit `Delete` to cancel the countdown, whether you're still inside the duration prompt or have "Timer" highlighted in the main search.

## Sticky Notes

Add a `"magi:notes"` entry (see below) to unlock it — an unlimited scratchpad of quick text notes, stored in `notes.json` next to `apps.json`/`themes.json`. Selecting it lists every note (newest first, shown live next to "Notes" in the main list), fuzzy-searchable exactly like the Window Switcher. The search box's placeholder switches to `Type a note...` while you're in it.

- Type text that matches an existing note, `Enter` copies it to the clipboard and closes the launcher
- Type text that matches nothing, `Enter` adds it as a new note and stays open so you can keep adding more
- `Delete` removes the highlighted note, `Shift+Delete` clears all of them
- `Shift+Enter` opens `notes.json` directly in its associated editor instead of copying — handy for editing a note by hand (multi-line, reordering...)
- `Tab` / `Escape` / `Backspace` on an empty search closes the picker

You don't even need to open the picker to drop the most recent note: hit `Delete` directly on the "Note" entry in the main list, same as canceling the Timer without opening its prompt.

## Auto-restart

Add a `"magi:auto-restart"` entry (see below) to unlock it — a list of targets (any `path`, same format as an `apps.json` entry, arguments included — nothing is rejected up front) that MAGI keeps alive in the background, stored in `restart.json` next to `apps.json`/`themes.json`. A dedicated thread checks every couple seconds whether each watched target's process is still running (by executable name, not by holding on to a handle) and relaunches it the moment it isn't — crash, or you closing it yourself, it makes no difference, it comes back either way. Selecting the entry lists every currently watched target (shown live next to "Auto-restart" in the main list as `Auto-restart: N`, `0` when empty), fuzzy-searchable exactly like Sticky Notes, each one prefixed with `★` (currently running) or `☆` (not running right now). The search box's placeholder switches to `Type a target to watch...` while you're in it.

- Type text that matches an existing target, `Enter` does nothing (there's nothing useful to do with an existing entry here beyond removing it, see `Delete` below)
- Type a path that matches nothing, `Enter` adds it to the watch list and stays open so you can keep adding more — type it exactly as you'd paste it from Explorer's address bar (single backslashes); this box is plain text, not JSON, so no doubling is needed here. The doubled backslashes you'd see if you opened the raw `restart.json` afterward are just its normal JSON escaping on write (the same rule `apps.json`'s `path` follows, see below) — MAGI reads them back correctly either way.
- `Delete` stops watching the highlighted target (does **not** close or kill the app itself, just ends the supervision)
- `Shift+Delete` stops watching every target at once (clears the whole list, same as Sticky Notes)
- `Tab` / `Escape` / `Backspace` on an empty search closes the picker

A target doesn't need to already exist elsewhere in `apps.json` — the two lists are entirely independent, so the same app can be a normal launchable entry, a watched auto-restart target, both, or neither. Since detection is purely "is this executable name running at all," MAGI can't tell a genuine crash apart from you closing the window on purpose — if it's on the list, it comes back, period. There's also no attempt to detect a frozen-but-still-running ("Not Responding") target and force-kill it: that would risk killing something that was only briefly busy and about to recover on its own, which is worse than doing nothing. The tray menu also has its own "Disable/Enable Auto-restart" toggle, for pausing the whole supervisor without touching the watch list itself (same idea as disabling the hotkey).

## Recycle Bin

The built-in `"magi:empty-recycle-bin"` entry (see below) shows the live item count/size next to its name, refreshed immediately the moment you empty it — reopening the launcher right after never shows stale leftovers. Hitting `Enter` on it opens a fuzzy-searchable list of what's actually in the Recycle Bin right now (every drive, read straight from `$Recycle.Bin`, off the UI thread so a slow/large bin never freezes the launcher — no Explorer window involved):

- Type to fuzzy-filter the list of deleted items, same as anywhere else
- `Enter` on a highlighted item copies its full name (with extension) to the clipboard and closes the launcher
- `Delete` permanently deletes just the highlighted item from the Recycle Bin — the rest is untouched
- `Shift+Delete` empties the whole Recycle Bin from inside this view too — the launcher stays open, the (now empty) list refreshes in place
- `Tab` / `Escape` / `Backspace` on an empty search backs out to the main list

To empty the Recycle Bin itself without opening it, use `Shift+Enter` or `Delete` directly on the main list's entry — deliberately a different keystroke than the one that opens the browse view, so glancing at what's in there can never accidentally empty it. Either way, the launcher stays open; only the item count next to the entry updates.

## Emoji

Add a `"magi:emoji"` entry (see below) to unlock it — fuzzy-search the official Unicode emoji names ("fire", "red heart", "grinning face"...) and hit `Enter` to copy the emoji itself to the clipboard and close the launcher. No bundled emoji list, no JSON: it reads `emoji-test.txt`, the plain-text reference file Unicode itself publishes at [unicode.org/Public/emoji/latest/emoji-test.txt](https://www.unicode.org/Public/emoji/latest/emoji-test.txt), placed next to the executable like `apps.json`/`themes.json`. The main list shows `Emoji: Version 17.0` (or whatever version the file declares) next to its name, live; if the file is missing, it shows `Emoji: missing emoji-test.txt` instead and `Enter` on it does nothing — download a copy from the link above and drop it next to the `.exe` (or hit **Reload** afterward if the launcher is already running) to unlock it.

- Type to fuzzy-filter by name, same as anywhere else
- `Tab` / `Escape` / `Backspace` on an empty search backs out to the main list

To pick up a newer emoji set later, just replace `emoji-test.txt` with a fresher copy from Unicode and hit **Reload** — no rebuild needed. Freshly-added emoji may still render as a blank box in the list until Windows' own emoji font catches up (copying still works regardless).

## Copy History

Opt-in (off by default) — toggle it from the tray menu ("Enable/Disable Copy History"), or add a `"magi:copy-history"` entry (see below) to browse it from the launcher itself. While enabled, every piece of text you copy anywhere on the system is recorded, most recent first, shown live next to "Copy History" in the main list as a count (or `disabled` while the toggle is off).

- Type to fuzzy-filter past copies, same as anywhere else
- `Enter` on a highlighted entry copies it back to the clipboard and closes the launcher — without adding a duplicate entry for that re-copy
- `Delete` removes the highlighted entry, `Shift+Delete` clears the whole history
- `Tab` / `Escape` / `Backspace` on an empty search backs out to the main list

**Where it lives:** nowhere on disk, ever. The history is kept purely in the launcher's own process memory (RAM), capped at 1,000,000 characters total (oldest entries are dropped first once full) — turn the PC off and it's gone without a trace, no file to find, no telemetry, nothing but this process can read it. Each entry's memory is pinned with `VirtualLock` so Windows never swaps it to disk under memory pressure, and is overwritten with zeros the moment it's dropped (evicted, deleted, or the app closes). Deliberately **not** encrypted: a decryption key living in the same process wouldn't stop anything that can already read the process's memory, and an encrypted-in-RAM buffer is closer to how an infostealer hides its own captured data than how an ordinary clipboard manager behaves — plain memory that's locked, never persisted, and zeroed on release is both simpler and just as safe here.

The `hotkey_enabled`/`auto_restart_enabled`/`copy_history_enabled` state of all three tray toggles is persisted to `apps.json` (see below) the moment you flip them, so they survive a restart.

## Eject

Add a `"magi:eject"` entry (see below) to unlock it — a fuzzy-searchable list of every USB-connected drive currently plugged in.

- Type to fuzzy-filter the list, same as anywhere else
- `Enter` on a highlighted drive ejects it *only if nothing currently has it open* — on success it drops off the list and the launcher stays open, so ejecting several drives in a row never needs reopening the picker. If something still has a handle open on it (antivirus/indexer mid-scan, an app with a file open...), `Enter` does nothing at all and leaves the drive untouched — see `Shift+Delete` below to force it anyway
- `Shift+Delete` force-ejects the highlighted drive even if something still has it open — see the warning below before relying on this
- `Tab` / `Escape` / `Backspace` on an empty search backs out to the main list

Only lists drives on a USB bus (checked directly against the device, not `GetDriveTypeW`'s removable/fixed flag, which misreports plenty of external USB-SATA/UASP enclosures as "fixed") — never the system drive, never an internal secondary drive.

Deliberately **not** the same mechanism as Windows' own "Safely Remove Hardware" tray icon (`CM_Request_Device_Eject`), which decides whether a device is ejectable by trusting a capability flag its driver reports — plenty of external enclosures never set it correctly, so Windows' own menu ends up missing the option, or greyed out, for a drive that works perfectly fine day to day. This locks and dismounts the drive letter's volume directly (`FSCTL_LOCK_VOLUME`/`FSCTL_DISMOUNT_VOLUME` + `IOCTL_STORAGE_EJECT_MEDIA`) instead, the same path most third-party USB ejector utilities use — it works precisely in the cases where Windows' own option doesn't show up at all.

**`Shift+Delete` forces the eject through — it does not ask permission first.** Unlike plain `Enter` (which backs off cleanly the moment the volume lock is refused), the forced path skips that check entirely: the dismount step succeeds even while another process still has a file open on the drive, and that process's handle is simply invalidated (it gets an I/O error) rather than blocking the eject. A background scan (antivirus, search indexer) being interrupted mid-*read* this way is harmless — nothing was being modified, it just errors out on its end. But a file genuinely being *written* to the drive at that exact moment (an active copy, a save in progress) will end up truncated/corrupted, with no warning beforehand — there's no check for "is anything mid-write right now," forced means forced.

## Configuration

`apps.json` and `themes.json` live next to the executable — never bundled inside it — so you can hand-edit them without rebuilding. Use **Reload** (a `"magi:reload"` entry) to pick up changes without restarting the app.

`notes.json`/`restart.json` live there too, but they're a different kind of file: `notes.json` is a plain JSON array of strings (`["note 1", "note 2"]`), `restart.json` a plain JSON array of `path` strings (`["A:\\Apps\\Foo\\Foo.exe"]`) — both created and rewritten automatically by the app itself whenever you add or remove an entry in Sticky Notes/Auto-restart, so the launcher's own in-memory copy is normally always the source of truth. Not meant to be hand-edited, but **Reload** re-reads both from disk too (alongside `apps.json`/`themes.json`), so a manual edit to either still gets picked up without restarting.

`emoji-test.txt` is yet another kind: a plain-text reference file straight from Unicode (see [Emoji](#emoji) above), not JSON and not written by MAGI itself — you replace the whole file to update it, there's nothing in it to hand-edit entry by entry. Optional: the Emoji picker just stays locked (with an explicit `Emoji: missing emoji-test.txt` in the main list) if it's absent.

A `shortcuts/` folder next to the executable is optional too: every file directly inside it (no subfolders) becomes a launchable entry, listed after `apps.json`'s own entries at equal search rank — `.lnk`, `.bat`, `.cmd`, `.vbs`, anything at all, MAGI doesn't check the extension, it just hands the path to the same `ShellExecute` call every other entry uses (which already knows how to resolve a `.lnk` or run a `.bat`/`.vbs` exactly like double-clicking it in Explorer would). The entry's name is the filename without its extension. Absent folder, or nothing in it: no effect, nothing extra shown.

### `apps.json`

```json
{
  "hotkey": "ctrl+space",
  "hotkey_enabled": true,
  "auto_restart_enabled": true,
  "copy_history_enabled": false,
  "apps": [
    { "name": "Notepad", "path": "%windir%\\system32\\notepad.exe" },
    { "name": "Command Prompt", "path": "%windir%\\system32\\cmd.exe", "cwd": "%HOMEDRIVE%%HOMEPATH%" },
    { "name": "Some background script", "path": "A:\\Scripts\\thing.ps1", "hidden": true }
  ]
}
```

- **`hotkey`** — a spec like `"ctrl+space"`, `"ctrl+alt+f"`, `"win+e"`, `"f14"`. Supports `ctrl`/`control`, `alt`, `shift`, `win`/`super`, `space`, `enter`/`return`, `tab`, `esc`/`escape`, `f1`–`f24`, and single characters.
- **`hotkey_enabled`** / **`auto_restart_enabled`** / **`copy_history_enabled`** (all optional; default `true`, `true`, `false` respectively) — mirror the three tray menu toggles, read once at startup and rewritten automatically the moment you flip one from the tray. Not meant to be hand-edited during a run (the tray is the source of truth while the launcher is up), but safe to set by hand before first launch.
- **`name`** (required) — display name, fuzzy-searchable.
- **`path`** (required) — a plain path, a path with arguments (`"app.exe --flag"`), a shell URI (`ms-settings:...`, `shell:RecycleBinFolder`...), or a [special `magi:` entry](#special-entries) below. Everything goes through `ShellExecute`, so anything Explorer can open (including document types resolved by file association, like `.msc` files) works. Backslashes must be doubled (`"A:\\Apps\\Foo.exe"`) since `\` is a JSON escape character — a single `\` is invalid JSON and can silently corrupt the path (`\t`, `\n`... are real escape sequences). Forward slashes (`"A:/Apps/Foo.exe"`) work too and need no escaping — Windows accepts both.
- **`cwd`** (optional) — working directory. Defaults to the target's own folder (mirroring an Explorer double-click), except for `cmd.exe`/`powershell.exe`-style entries where you'll usually want to set this explicitly (otherwise they start in `system32`).
- **`hidden`** (optional, default `false`) — launch without a visible window (`SW_HIDE`), for scripts that have nothing to show.

#### Special entries

| `path` | Effect |
|---|---|
| `magi:reload` | Reload `apps.json`/`themes.json`/`notes.json`/`restart.json`/`emoji-test.txt`/`shortcuts/` in place, no restart |
| `magi:theme-picker` | Enter a live theme picker (see below) |
| `magi:timer` | Enter the timer's duration prompt; shows `<name>: --:--` while idle, the live countdown once armed |
| `magi:notes` | Enter the Sticky Notes picker (see above); shows `<name>:` when empty, `<name>: <latest note>` otherwise |
| `magi:auto-restart` | Enter the Auto-restart picker (see above); shows `<name>: N` for the number of watched targets, `0` when empty |
| `magi:copy-history` | Enter the Copy History picker (see above); shows `<name>: N` for the number of stored entries, or `<name>: disabled` while the tray toggle is off (Enter does nothing then) |
| `magi:open-folder` | Open the folder containing MAGI Launcher in Explorer — resolved fresh on every launch, so it follows if the folder gets moved |
| `magi:empty-recycle-bin` | Shows `<name>: N items, X MB` when non-empty; `Enter` browses its contents (see [Recycle Bin](#recycle-bin) above), `Shift+Enter` or `Delete` empties it |
| `magi:emoji` | Enter the Emoji picker (see above); shows `<name>: Version X.Y` from `emoji-test.txt`, or `<name>: missing emoji-test.txt` (Enter does nothing) if the file isn't there |
| `magi:eject` | Enter the Eject picker (see above) |
| `magi:media-play-pause`, `magi:media-next`, `magi:media-previous`, `magi:media-stop`, `magi:media-volume-mute`, `magi:media-volume-down`, `magi:media-volume-up` | Sends the corresponding virtual media key, routed the same way a real hardware key would be (global media session, not just this window) |

A genuine "Shut Down Windows" dialog (the same one you get from `Alt+F4` on the desktop) can be added as a normal entry too, no special code needed:

```json
{
  "name": "Shutdown",
  "path": "%windir%\\system32\\WindowsPowerShell\\v1.0\\powershell.exe -command \"(New-Object -ComObject Shell.Application).ShutdownWindows()\"",
  "hidden": true
}
```

### `themes.json`

```json
{
  "theme": "arc-dark",
  "font_family": "Segoe UI",
  "placeholder_text": "Type to search...",
  "show_clock": true,
  "window_size": 30,
  "border": 3,
  "themes": {
    "arc-dark": {
      "search_background": "#404552",
      "search_text": "#7c818c",
      "list_background": "#383c4a",
      "list_text": "#d3dae3",
      "selected_background": "#5294e2",
      "selected_text": "#ffffff",
      "border": "#4b5162"
    }
  }
}
```

Root-level keys apply regardless of the active theme:

- **`theme`** — name of the active entry in `themes`
- **`font_family`** — omit/empty to keep the OS default font. Recommended font: [SGr-Iosevka-Regular.ttc](https://github.com/be5invis/iosevka)
- **`placeholder_text`** — shown in the search box while empty (overridden by a mode-specific placeholder in Timer/Sticky Notes/Auto-restart, see their sections above)
- **`show_clock`** — show the current time (in the user's Windows short time format) next to the search box
- **`window_size`** — percentage (0–100) of the screen width the popup occupies (height/font sizes follow, keeping a 16:9 ratio, always centered on the monitor under the cursor and clamped to never run off-screen). Adjustable live from the keyboard too, see `Ctrl+1`–`Ctrl+0` above — each press writes the new value straight back here.
- **`border`** — simulated border thickness in pixels. Adjustable live too, see `Ctrl+-`/`Ctrl+=` above.

Ships with 100+ built-in themes (mostly character/game palettes) in the `themes` dict — open the launcher and select the `Themes` entry (`magi:theme-picker`) to preview and switch between them live, no restart required. The picker opens on whichever theme is currently active, not the first one alphabetically. Selecting one writes it back to `themes.json`.

## Credits

Built together with [Claude](https://claude.com) (Anthropic's AI coding assistant).

## License

Copyright (C) 2026 Nyaldee. Licensed under the [GNU General Public License v3.0](LICENSE) — see the `LICENSE` file for the full text.
