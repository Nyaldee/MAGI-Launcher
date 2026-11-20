# Truncate

*[Lire en français](README.fr.md) · [日本語で読む](README.jp.md) · [阅读中文版](README.zh-CN.md)*

Give a file the exact size you want: create it, extend it (zero-filled), shrink it, or shift/crop its content. For Windows and Linux, pure Rust, no dependencies. Inspired by [dbat/truncate](https://github.com/dbat/truncate), with a clearer positional + flag syntax instead of the original's sign-based one.

## Usage

```
truncate <file> [size[K|M|G|T]] [--skip N[K|M|G|T]] [--trim-end N[K|M|G|T]] [--quiet]
truncate <file>
```

The file is created if it doesn't exist and a `size` is given; trimming a missing file is an error. Sizes are in bytes unless suffixed with K/M/G/T (binary, ×1024, case-insensitive).

| Argument | Meaning |
| --- | --- |
| `size` | Final size, created/truncated/extended to exactly this. Extending zero-fills the new bytes. |
| `--skip N` | Bytes to discard from the start (shifts the rest to the beginning; anything past the shifted content is zero-filled). Default 0. |
| `--trim-end N` | Bytes to discard from the end. Only valid without `size`. |
| `--quiet`, `-q` | Don't print the before/after size summary. |
| `--help`, `-h` | Show usage. |

Called with just a file and nothing else, `truncate` doesn't touch anything — it prints the file's current size.

## Examples

```
truncate foo.bin                          # just show its current size
truncate zero.bin 0                       # empty the file (create if needed)
truncate foo.bin 2G                       # create/resize to exactly 2 GiB
truncate bar.bin --skip 19K --trim-end 19K # crop 19 KiB off both ends
truncate baz.bin 1T --skip 777            # shift content from byte 777, zero-fill up to 1 TiB
```

Every resize prints a summary unless `--quiet` is given:

```
> truncate foo.bin 2G
foo.bin: 10.00 MiB (10485760 B) -> 2.00 GiB (2147483648 B)
```

## Menu

`Truncate-Menu.bat` (`Truncate-Menu.sh` on Linux) wraps the executable in a bilingual (French/English) interactive menu: drop a file into `input\`, pick a mode, get the result in `output\` (the original in `input\` is never modified; showing the size reads it in place, without copying). Useful if you don't want to remember the flag syntax.

## Linux

Everything above works the same on Linux, with `./truncate` and `./Truncate-Menu.sh`, and `input/` and `output/` folders.

Linux already has its own `truncate` command (GNU coreutils), with a different syntax (`truncate -s 2G file`). Run this one from its folder, and don't put it in your `PATH` ahead of the system one: scripts that expect the GNU version would break.

## Updating

Run `truncate_updater.bat` (`truncate_updater.sh` on Linux) from the `Truncate` folder: it downloads the latest release and extracts it in place. Your `input` and `output` folders are left untouched.

## Build

- **Windows** — run `build.bat`; the archive ends up at `target\Truncate.Windows.zip`.
- **Linux** — run `build_linux.bat` from Windows (it builds in WSL, distribution `Ubuntu`), or `cargo build --release` on Linux. The archive ends up at `target_linux\Truncate.Linux.tar.gz`.

## License

Copyright (C) 2026 Nyaldee. Licensed under the [GNU General Public License v3.0](LICENSE) — see the `LICENSE` file for the full text.
