# CASPER Shot

*[Read in English](README.md) · [Lire en français](README.fr.md) · [日本語で読む](README.jp.md)*

一个小巧的 Windows 截图工具：两个全局快捷键，WebP 格式保存，通知区域图标。没有窗口，无需安装。

## 功能

- 全局快捷键截取光标所在的显示器（默认 `PrintScreen`）。
- 全局快捷键拖动选择区域，或点击某个窗口来选中整个窗口，适用于任意显示器（默认 `Shift+PrintScreen`）。按 Esc 或右键取消。两个快捷键都可以修改，见下文。
- 左键点击通知区域图标可直接进入区域选择模式，无需快捷键。
- 每张截图都会复制到剪贴板，并以 `.webp` 保存到 `YYYY-MM` 子文件夹中，文件名由光标下的窗口、日期和质量组成（`Discord_06-14-2026_q90.webp`）。已有文件永远不会被覆盖，而是加上 ` (1)`、` (2)`…。
- 屏幕底部会短暂显示预览。左键点击打开图片，右键点击打开所在文件夹。预览不会出现在之后的截图中。
- 图标菜单（右键）：启用／禁用快捷键、打开截图文件夹、Settings、Reload、Website（打开 nyaruhodo.com）、退出。

## 设置（`state.json`）

首次运行时在可执行文件旁创建。图标菜单 → Settings 打开它（如果没有应用关联 `.json`，则用记事本打开）；Reload 应用修改。如果文件中的 JSON 无效，CASPER Shot 会使用默认值，并保留文件不动，方便你修正。

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

- `quality`：0-99 为有损 WebP，100 为无损。
- `hotkey_*`：`Ctrl`／`Alt`／`Shift`／`Win` 的任意组合，加上一个字母、数字、F1-F12 或 `PrintScreen`。
- `save_path`：支持 `%ENV_VAR%` 环境变量展开；相对路径以可执行文件所在文件夹为基准。`%USERPROFILE%\Pictures`（以及 Desktop、Documents、Downloads、Music、Videos）会跟随 Windows 移动后的位置，包括 OneDrive。
- `preview_percent`：预览大小，占屏幕宽度和高度的百分比；`0` 完全禁用预览。
- `preview_duration_ms`：预览关闭前的显示时间。

## 快捷键无效？

`RegisterHotKey` 只允许一个进程在整个系统中占用某个按键组合。如果另一个正在运行的应用已经绑定了相同的组合（ShareX、Greenshot、Windows 设置 → 辅助功能 → 键盘中的截图快捷键等），CASPER Shot 的注册会静默失败，图标的提示文字会显示 "hotkeys unavailable" 而不是快捷键列表。请关闭或重新配置占用它们的应用，或在 `state.json` 中选择其他快捷键后 Reload。

## 更新

在 `CASPER Shot` 文件夹中运行 `casper_shot_updater.bat`：它会关闭 CASPER Shot，下载最新版本，就地解压并重新启动。`state.json` 和你的截图不会受到影响。

## 许可证

个人非商业用途免费。Copyright (C) 2026 Nyaldee，保留所有权利——完整条款见 [LICENSE](../LICENSE)。请仅从本仓库的 [Releases](https://github.com/Nyaldee/nyaruhodo/releases) 下载。

Anthropic 的 AI [Claude](https://claude.com) 参与了本项目的代码编写。
