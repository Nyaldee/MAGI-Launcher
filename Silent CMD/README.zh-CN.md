# Silent CMD

*[Read in English](README.md) · [Lire en français](README.fr.md) · [日本語で読む](README.jp.md)*

一个小巧的 Windows 启动器，运行命令或脚本（`.bat`／`.cmd`）时不显示控制台窗口。

## 用法

```
silent_cmd.exe [选项] <命令> [参数...]
```

选项之后的所有内容都会按原样传给 `cmd /C`，包括引号和特殊字符（`&`、`^`、`%`…）。命令的退出码和输出重定向照常有效——只是隐藏了控制台窗口。选项可以在命令之前以任意顺序组合使用。

| 选项 | 说明 |
|---|---|
| `--delay <秒>` | 等待指定时间后再运行命令。 |
| `--log <文件>` | 把命令的输出和错误写入 `文件`（每次运行都会覆盖）。如果指定了 `--dir`，相对路径以它为基准。 |
| `--dir <路径>` | 以 `路径` 作为工作目录运行命令。 |
| `--nowait` | 启动命令后立即返回，不等待它结束。 |

```
silent_cmd.exe myscript.bat
silent_cmd.exe myscript.bat arg1 arg2
silent_cmd.exe --delay 5 myscript.bat
silent_cmd.exe --log out.log myscript.bat
silent_cmd.exe --dir "C:\Work" myscript.bat
silent_cmd.exe --nowait myscript.bat
silent_cmd.exe --delay 5 --dir "C:\Work" --log out.log myscript.bat arg1 arg2
```

退出码：命令自身的退出码（使用 `--nowait` 时，启动成功即为 `0`）；无法启动命令或无法创建 `--log` 文件时为 `1`；没有命令或选项不完整时为 `2`。

在没有控制台的情况下启动时（快捷方式、任务计划程序），命令的标准输入为空：`pause` 或 `set /p` 会立即返回，而不会一直等待一个看不见的控制台。始终使用系统自带的 `cmd.exe`，并禁用注册表中的 AutoRun 命令（`/D`）。

直接双击 `.bat` 总会闪现一下控制台——那是 `cmd.exe` 本身，而不是 Silent CMD。要真正无窗口启动，双击的目标必须是 Silent CMD 本身，例如一个指向 `silent_cmd.exe`、以脚本为参数的快捷方式，而不是直接双击 `.bat`。

## 许可证

个人非商业用途免费。Copyright (C) 2026 Nyaldee，保留所有权利——完整条款见 [LICENSE](../LICENSE)。请仅从本仓库的 [Releases](https://github.com/Nyaldee/nyaruhodo/releases) 下载。

Anthropic 的 AI [Claude](https://claude.com) 参与了本项目的代码编写。
