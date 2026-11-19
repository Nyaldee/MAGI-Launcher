# Truncate

*[Lire en français](README.fr.md) · [日本語で読む](README.jp.md) · [阅读中文版](README.zh-CN.md)*

Give a file the exact size you want: create it, extend it (zero-filled), shrink it, or shift/crop its content. Pure Rust, no dependencies. Inspired by [dbat/truncate](https://github.com/dbat/truncate), with a clearer positional + flag syntax instead of the original's sign-based one.

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

`Truncate-Menu.bat` wraps the exe in a bilingual (French/English) interactive menu: drop a file into `input\`, pick a mode, get the result in `output\` (the original in `input\` is never modified; showing the size reads it in place, without copying). Useful if you don't want to remember the flag syntax.

## Updating

Run `truncate_updater.bat` from the `Truncate` folder: it downloads the latest release and extracts it in place. Your `input\` and `output\` folders are left untouched.

## License

Free for personal, non-commercial use. Copyright (C) 2026 Nyaldee, all rights reserved — see [LICENSE](../LICENSE) for the full terms. Download it only from this repository's [releases](https://github.com/Nyaldee/nyaruhodo/releases).
