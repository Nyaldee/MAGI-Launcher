# Silent CMD

*[Lire en français](README.fr.md) · [日本語で読む](README.jp.md) · [阅读中文版](README.zh-CN.md)*

A tiny Windows launcher that runs a command or script (`.bat`/`.cmd`) with no visible console window.

## Usage

```
silent_cmd.exe [options] <command> [args...]
```

Everything after the options is passed to `cmd /C` exactly as written, quotes and special characters (`&`, `^`, `%`…) included. The command's exit code and its output redirections work as usual — only the console window is hidden. Options can be combined, in any order, before the command.

| Option | Description |
|---|---|
| `--delay <seconds>` | Waits before running the command. |
| `--log <file>` | Writes the command's output and errors to `file` (overwritten each run). A relative path starts from `--dir` when it's given. |
| `--dir <path>` | Runs the command with `path` as its working directory. |
| `--nowait` | Launches the command and returns immediately instead of waiting for it to finish. |

```
silent_cmd.exe myscript.bat
silent_cmd.exe myscript.bat arg1 arg2
silent_cmd.exe --delay 5 myscript.bat
silent_cmd.exe --log out.log myscript.bat
silent_cmd.exe --dir "C:\Work" myscript.bat
silent_cmd.exe --nowait myscript.bat
silent_cmd.exe --delay 5 --dir "C:\Work" --log out.log myscript.bat arg1 arg2
```

Exit code: the command's own exit code (`0` with `--nowait` once it's launched), `1` if it can't be launched or the `--log` file can't be created, `2` if there's no command or an option is incomplete.

When launched without a console (shortcut, Task Scheduler), the command gets an empty standard input: a `pause` or `set /p` returns immediately instead of waiting forever on an invisible console. The system `cmd.exe` is always used, with the registry AutoRun commands disabled (`/D`).

A `.bat` double-clicked directly always flashes a console for a split second — that's `cmd.exe` itself, not Silent CMD. For a truly invisible launch, the double-click target must be Silent CMD itself, for example a shortcut to `silent_cmd.exe` with the script as its argument, never the `.bat` directly.

## License

Free for personal, non-commercial use. Copyright (C) 2026 Nyaldee, all rights reserved — see [LICENSE](../LICENSE) for the full terms. Download it only from this repository's [releases](https://github.com/Nyaldee/nyaruhodo/releases).

[Claude](https://claude.com), Anthropic's AI, contributed to the code of this project.
