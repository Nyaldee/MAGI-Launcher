# Truncate

*[Read in English](README.md) · [Lire en français](README.fr.md) · [日本語で読む](README.jp.md)*

让文件变成你想要的确切大小：创建、扩展（用零填充）、缩小，或平移／裁剪其内容。适用于 Windows 和 Linux，纯 Rust，无依赖。灵感来自 [dbat/truncate](https://github.com/dbat/truncate)，但用更清晰的“位置参数 + 选项”写法，取代了原版基于正负号的写法。

## 用法

```
truncate <文件> [大小[K|M|G|T]] [--skip N[K|M|G|T]] [--trim-end N[K|M|G|T]] [--quiet]
truncate <文件>
```

如果文件不存在且给出了 `大小`，就会创建它；对不存在的文件进行裁剪会报错。大小以字节为单位，除非带有 K/M/G/T 后缀（二进制，×1024，不区分大小写）。

| 参数 | 含义 |
| --- | --- |
| `大小` | 最终大小，文件会被创建／截断／扩展到恰好这个大小。扩展出的新字节用零填充。 |
| `--skip N` | 从开头丢弃的字节数（其余内容移到开头；平移后内容之外的部分用零填充）。默认 0。 |
| `--trim-end N` | 从末尾丢弃的字节数。仅在不指定 `大小` 时有效。 |
| `--quiet`、`-q` | 不显示前后大小的摘要。 |
| `--help`、`-h` | 显示用法。 |

只给出一个文件而不带其他参数时，`truncate` 不会改动任何东西——它只显示文件的当前大小。

## 示例

```
truncate foo.bin                          # 只显示当前大小
truncate zero.bin 0                       # 清空文件（必要时创建）
truncate foo.bin 2G                       # 创建／调整到恰好 2 GiB
truncate bar.bin --skip 19K --trim-end 19K # 从两端各裁掉 19 KiB
truncate baz.bin 1T --skip 777            # 从第 777 字节起平移内容，用零填充到 1 TiB
```

每次调整大小都会显示摘要，除非指定了 `--quiet`：

```
> truncate foo.bin 2G
foo.bin: 10.00 MiB (10485760 B) -> 2.00 GiB (2147483648 B)
```

## 菜单

`Truncate-Menu.bat`（Linux 上为 `Truncate-Menu.sh`）把这个可执行文件包装成一个双语（法语／英语）交互式菜单：把文件放进 `input\`，选择一种模式，结果会出现在 `output\` 中（`input\` 中的原文件永远不会被修改；显示大小时直接就地读取，不会复制）。如果你不想记选项的写法，这会很方便。

## Linux

以上所有内容在 Linux 上同样适用，使用 `./truncate` 和 `./Truncate-Menu.sh`，文件夹为 `input/` 和 `output/`。

Linux 已经自带一个 `truncate` 命令（GNU coreutils），写法不同（`truncate -s 2G 文件`）。请在它所在的文件夹中运行本工具，不要把它放进 `PATH` 并排在系统自带的版本前面：否则依赖 GNU 版本的脚本将无法正常工作。

## 更新

在 `Truncate` 文件夹中运行 `truncate_updater.bat`（Linux 上为 `truncate_updater.sh`）：它会下载最新版本并就地解压。你的 `input` 和 `output` 文件夹不会受到影响。

## 构建

- **Windows**——运行 `build.bat`，压缩包会生成在 `target\Truncate.Windows.zip`。
- **Linux**——在 Windows 上运行 `build_linux.bat`（在 WSL 的 `Ubuntu` 发行版中构建），或在 Linux 上运行 `cargo build --release`。压缩包会生成在 `target_linux\Truncate.Linux.tar.gz`。

## 许可证

Copyright (C) 2026 Nyaldee。根据 [GNU General Public License v3.0](LICENSE) 授权，完整文本见 `LICENSE` 文件。
