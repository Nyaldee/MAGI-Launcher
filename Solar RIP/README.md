# Solar RIP

*[Lire en français](README.fr.md) · [日本語で読む](README.jp.md) · [阅读中文版](README.zh-CN.md)*

A small patcher for the `.rip` format, for Windows and Linux — like Lunar IPS, but with 64-bit offsets and sizes instead of IPS's 3-byte fields, so there's no 16 MB size limit. A patch works both ways: it patches the original, and turns the patched file back into the original. Files of any size are patched in a single pass, without loading them into memory. See [Linux](#linux) for what differs there.

## Format

A `.rip` holds the difference between an original file and its modified version: the `RIP` signature, then the records, then the sizes and CRC32s of both files. Fixed-size numbers are little-endian.

| Bytes | Content |
|---|---|
| 3 | Signature `RIP` |
| variable | Records |
| 8 | Original file size |
| 4 | Original file CRC32 |
| 8 | Modified file size |
| 4 | Modified file CRC32 |
| 4 | CRC32 of everything above |

Each record is a number giving its distance from the end of the previous record (from the start of the files for the first one), a number giving its length (at least 1), then that many bytes: the original XOR the modified file at that position. These numbers use the variable-length encoding of BPS: 7 bits per byte, the high bit marks the last byte, and each extra byte shifts the range of values, so every number has exactly one encoding.

Both files are seen as padded with zeros up to the length of the longer one. Outside the records they're identical, and everything past the end of the shorter one is in the records. Applying a patch XORs each record into the file at its position, which turns either file into the other, then sizes the result to the other file's size. The CRC32 is the one used by zip and gzip (IEEE 802.3). The sizes and CRC32s come last so that a patch is written in a single pass.

The format is fixed: there is a single version, and it won't change.

## Safety

Because a patch knows both files, every application is checked:

- **Nothing is replaced until the result is complete and checked.** The new file is written next to the one you picked in a single pass, and only then takes its place. If anything fails, the file is left untouched and the temporary file is deleted.
- **A file that's already patched** is recognized as such, and can be turned back into the original.
- **A file that's neither of the two** is reported with its size and CRC32, and is only patched after confirmation (`--force` on the command line).
- **A patch damaged by accident** is refused as soon as it's opened, thanks to its own CRC32.
- **A patch can't make Solar RIP write more than the files and its own data**: a malformed or malicious patch can't fill the disk.
- **Every patch Solar RIP creates is checked both ways before it's saved**: without writing anything, it must turn the original into exactly the modified file, and the modified file back into exactly the original.

CRC32 protects against accidents — a corrupted download, the wrong file — not against tampering: it isn't a signature, and a patch can be forged to match it. Only apply patches from sources you trust.

## Usage

- **Apply Patch** — pick a `.rip`, then the file to patch. The file is patched in place once the result has been checked (see [Safety](#safety)). Picking a file that's already patched offers to restore the original.
- **Create Patch** — pick an original and a modified file, save the resulting `.rip`. The app closes afterwards.
- **Scramble File** — saves a copy of a file with its first 4 KB replaced by noise, plus the `.rip` that restores it. The original is never modified.
- **Verify Patch** — shows the original and modified files a `.rip` expects (CRC32 and size) and its record count, without touching any file.
- **Register** — associates `.rip` files with this executable (per-user, no admin rights). Double-clicking a `.rip` afterwards skips straight to picking the file to patch, even while Solar RIP is open. Shows OFF again if the executable has been moved: click it to update the association.
- **Create Log** — toggles a `.log` file written next to the patched file (Apply Patch) or the patch (Create Patch, Scramble File).
- **Exit**
- **Website** — opens nyaruhodo.com.

In messages, Enter confirms and Escape cancels. Only one main window runs at a time — launching a second one brings the first to the foreground.

## Command line

The same operations run without the window, for scripts:

```
solar_rip apply <patch.rip> <file> [--force] [--log]
solar_rip restore <patch.rip> <file> [--log]
solar_rip create <original> <modified> <patch.rip> [--log]
solar_rip scramble <file> <scrambled> <patch.rip> [--log]
solar_rip verify <patch.rip>
solar_rip --help
```

- `apply` patches the file in place. It stops if the file is already patched, and if it isn't the original the patch expects, unless `--force` is given.
- `restore` turns a patched file back into the original, in place.
- `--log` writes the same `.log` file as **Create Log**.
- Exit code: `0` on success, `1` if the operation fails, `2` if the command is malformed. Errors go to the error output.

Solar RIP is a windowed program, so on Windows an interactive prompt doesn't wait for it and shows its result after the next prompt. Scripts (`.bat`, PowerShell) wait for it and get its output and exit code; in an interactive `cmd`, use `start /wait /b solar_rip …`.

## Linux

Everything above works on Linux too, with these differences:

- **File dialogs** come from your desktop through the XDG portal (`xdg-desktop-portal`, installed with GNOME, KDE and most desktops), so they look like the rest of the system.
- **Register** writes a MIME type for `*.rip`, a hidden application entry and its icon under `~/.local/share`, then runs `update-mime-database`, `update-desktop-database` and `xdg-mime` when they're installed (`shared-mime-info`, `desktop-file-utils`, `xdg-utils`). Nothing is added to the application menu.
- **Website** opens the address with `xdg-open` (`xdg-utils`).
- **Wayland** doesn't let an application place its own window or bring itself to the front: the compositor decides where Solar RIP opens, and launching it a second time doesn't raise the first window there.

## License

Free for personal, non-commercial use. Copyright (C) 2026 Nyaldee, all rights reserved — see [LICENSE](../LICENSE) for the full terms. Download it only from this repository's [releases](https://github.com/Nyaldee/nyaruhodo/releases).

[Claude](https://claude.com), Anthropic's AI, contributed to the code of this project.
